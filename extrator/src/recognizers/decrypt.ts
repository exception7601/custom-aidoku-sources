import traverseImport, { type NodePath } from '@babel/traverse';
import type { File } from '@babel/types';
import * as t from '@babel/types';

const traverse = ((traverseImport as { default?: unknown }).default ??
  traverseImport) as typeof import('@babel/traverse').default;

import { flattenPlusExpression } from '../evaluate.js';
import type { PassphraseStrategy } from '../manifest.js';

export interface DecryptRecognition {
  algorithm?: 'cryptojs-rabbit';
  passphraseFunctionName?: string;
  passphrase?: PassphraseStrategy;
}

export function recognizeDecryptSignals(ast: File): DecryptRecognition {
  let recognition: DecryptRecognition = {};

  traverse(ast, {
    CallExpression(path) {
      if (!isRabbitDecryptCall(path.node)) {
        return;
      }

      const secondArgument = path.get('arguments.1');
      if (!secondArgument || Array.isArray(secondArgument)) {
        return;
      }

      const passphraseFunctionName = extractPassphraseFunctionName(secondArgument);
      if (!passphraseFunctionName) {
        return;
      }

      recognition = {
        algorithm: 'cryptojs-rabbit',
        passphraseFunctionName,
        passphrase: extractPassphraseStrategy(path.scope, passphraseFunctionName),
      };
      path.stop();
    },
  });

  return recognition;
}

function isRabbitDecryptCall(node: t.CallExpression): boolean {
  if (!t.isMemberExpression(node.callee)) {
    return false;
  }

  const decryptProperty =
    !node.callee.computed && t.isIdentifier(node.callee.property)
      ? node.callee.property.name
      : undefined;
  if (decryptProperty !== 'decrypt') {
    return false;
  }

  const rabbitObject = node.callee.object;
  return (
    t.isMemberExpression(rabbitObject) &&
    !rabbitObject.computed &&
    t.isIdentifier(rabbitObject.property, { name: 'Rabbit' })
  );
}

function extractPassphraseFunctionName(path: NodePath<t.Node>): string | undefined {
  if (path.isIdentifier()) {
    const binding = path.scope.getBinding(path.node.name);
    if (binding?.path.isVariableDeclarator()) {
      const initPath = binding.path.get('init');
      if (initPath && !Array.isArray(initPath) && initPath.isCallExpression()) {
        const calleePath = initPath.get('callee');
        if (calleePath.isIdentifier()) {
          return calleePath.node.name;
        }
      }
    }

    return path.node.name;
  }

  if (path.isCallExpression()) {
    const calleePath = path.get('callee');
    return calleePath.isIdentifier() ? calleePath.node.name : undefined;
  }

  return undefined;
}

function extractPassphraseStrategy(
  scope: NodePath<t.Node>['scope'],
  functionName: string
): PassphraseStrategy | undefined {
  const binding = scope.getBinding(functionName);
  if (!binding) {
    return undefined;
  }

  const functionPath = resolveFunctionPath(binding.path);
  if (!functionPath) {
    return undefined;
  }

  const bodyPath = functionPath.get('body');
  if (Array.isArray(bodyPath) || !bodyPath.isBlockStatement()) {
    return undefined;
  }

  const splitLiteralByName = new Map<string, string>();
  let digestVariableName: string | undefined;
  let digestSlice: PassphraseStrategy['digestSlice'] | undefined;
  let md5Argument: t.Node | undefined;
  let returnExpression: t.Node | undefined;

  for (const statementPath of bodyPath.get('body')) {
    if (statementPath.isVariableDeclaration()) {
      for (const declarationPath of statementPath.get('declarations')) {
        if (!t.isIdentifier(declarationPath.node.id)) {
          continue;
        }

        const name = declarationPath.node.id.name;
        const initPath = declarationPath.get('init');
        if (!initPath || Array.isArray(initPath)) {
          continue;
        }

        const splitLiteral = extractSplitLiteral(initPath as NodePath<t.Node>);
        if (splitLiteral !== undefined) {
          splitLiteralByName.set(name, splitLiteral);
          continue;
        }

        const digestMetadata = extractDigestMetadata(initPath as NodePath<t.Node>);
        if (!digestMetadata) {
          continue;
        }

        digestVariableName = name;
        digestSlice = digestMetadata.digestSlice;
        md5Argument = digestMetadata.md5Argument;
      }

      continue;
    }

    if (statementPath.isReturnStatement()) {
      returnExpression = statementPath.node.argument ?? undefined;
    }
  }

  if (!digestVariableName || !digestSlice || !md5Argument || !returnExpression) {
    return undefined;
  }

  const returnParts = flattenPlusExpression(returnExpression);
  const prefixVariableName = findJoinedVariableName(returnParts[0]);
  const digestReturned = returnParts[1];
  if (!prefixVariableName || !t.isIdentifier(digestReturned, { name: digestVariableName })) {
    return undefined;
  }

  const md5Parts = flattenPlusExpression(resolveBoundExpression(functionPath.scope, md5Argument));
  const saltVariableName = findJoinedVariableName(md5Parts[1]);
  const suffixVariableName = findJoinedVariableName(md5Parts[2]);
  if (!saltVariableName || !suffixVariableName) {
    return undefined;
  }

  const dateExpression = md5Parts[0];
  if (!dateExpression || !isUtcDateTemplate(functionPath.scope, dateExpression)) {
    return undefined;
  }

  const prefix = splitLiteralByName.get(prefixVariableName);
  const salt = splitLiteralByName.get(saltVariableName);
  const suffix = splitLiteralByName.get(suffixVariableName);
  if (!prefix || !salt || !suffix) {
    return undefined;
  }

  return {
    kind: 'utc-md5-derived',
    dateFormat: 'YYYY-MM-DD',
    prefix,
    salt,
    suffix,
    digestEncoding: 'hex',
    digestSlice,
  };
}

function resolveFunctionPath(path: NodePath<t.Node>): NodePath<t.Function> | undefined {
  if (path.isFunctionDeclaration()) {
    return path;
  }

  if (path.isVariableDeclarator()) {
    const initPath = path.get('init');
    if (!initPath || Array.isArray(initPath) || !initPath.isFunction()) {
      return undefined;
    }

    return initPath;
  }

  return undefined;
}

function extractSplitLiteral(path: NodePath<t.Node>): string | undefined {
  if (!path.isCallExpression() || !path.get('callee').isMemberExpression()) {
    return undefined;
  }

  const callee = path.get('callee');
  const objectPath = callee.get('object');
  const propertyPath = callee.get('property');
  const firstArgument = path.get('arguments.0');

  if (
    Array.isArray(objectPath) ||
    Array.isArray(propertyPath) ||
    !objectPath.isStringLiteral() ||
    !propertyPath.isIdentifier({ name: 'split' }) ||
    !firstArgument ||
    Array.isArray(firstArgument) ||
    !firstArgument.isStringLiteral({ value: '' })
  ) {
    return undefined;
  }

  return objectPath.node.value;
}

function extractDigestMetadata(path: NodePath<t.Node>):
  | {
      digestSlice: PassphraseStrategy['digestSlice'];
      md5Argument: t.Node;
    }
  | undefined {
  if (!path.isCallExpression() || !path.get('callee').isMemberExpression()) {
    return undefined;
  }

  const substringCallee = path.get('callee');
  const substringProperty = substringCallee.get('property');
  if (Array.isArray(substringProperty) || !substringProperty.isIdentifier({ name: 'substring' })) {
    return undefined;
  }

  const [startPath, endPath] = path.get('arguments');
  if (
    !startPath ||
    !endPath ||
    Array.isArray(startPath) ||
    Array.isArray(endPath) ||
    !startPath.isNumericLiteral() ||
    !endPath.isNumericLiteral()
  ) {
    return undefined;
  }

  const toStringCallPath = substringCallee.get('object');
  if (
    Array.isArray(toStringCallPath) ||
    !toStringCallPath.isCallExpression() ||
    !toStringCallPath.get('callee').isMemberExpression()
  ) {
    return undefined;
  }

  const toStringCallee = toStringCallPath.get('callee');
  const toStringProperty = toStringCallee.get('property');
  if (Array.isArray(toStringProperty) || !toStringProperty.isIdentifier({ name: 'toString' })) {
    return undefined;
  }

  const md5CallPath = toStringCallee.get('object');
  if (!md5CallPath.isCallExpression() || !md5CallPath.get('callee').isMemberExpression()) {
    return undefined;
  }

  const md5Callee = md5CallPath.get('callee');
  const md5Property = md5Callee.get('property');
  if (Array.isArray(md5Property) || !md5Property.isIdentifier({ name: 'MD5' })) {
    return undefined;
  }

  const md5ArgumentPath = md5CallPath.get('arguments.0');
  if (!md5ArgumentPath || Array.isArray(md5ArgumentPath)) {
    return undefined;
  }

  return {
    digestSlice: {
      start: startPath.node.value,
      end: endPath.node.value,
    },
    md5Argument: md5ArgumentPath.node,
  };
}

function findJoinedVariableName(node: t.Node | undefined): string | undefined {
  if (!node || !t.isCallExpression(node) || !t.isMemberExpression(node.callee)) {
    return undefined;
  }

  if (node.callee.computed || !t.isIdentifier(node.callee.property, { name: 'join' })) {
    return undefined;
  }

  return t.isIdentifier(node.callee.object) ? node.callee.object.name : undefined;
}

function isUtcDateTemplate(scope: NodePath<t.Function>['scope'], node: t.Node): boolean {
  if (!t.isIdentifier(node)) {
    return false;
  }

  const binding = scope.getBinding(node.name);
  if (!binding || !binding.path.isVariableDeclarator()) {
    return false;
  }

  const init = binding.path.node.init;
  if (!t.isTemplateLiteral(init) || init.expressions.length !== 3) {
    return false;
  }

  const [yearExpression, monthExpression, dayExpression] = init.expressions;
  if (!yearExpression || !monthExpression || !dayExpression) {
    return false;
  }

  return (
    containsPropertyName(yearExpression, 'getUTCFullYear') &&
    containsPropertyName(monthExpression, 'getUTCMonth') &&
    containsPropertyName(dayExpression, 'getUTCDate')
  );
}

function resolveBoundExpression(scope: NodePath<t.Function>['scope'], node: t.Node): t.Node {
  if (!t.isIdentifier(node)) {
    return node;
  }

  const binding = scope.getBinding(node.name);
  if (!binding || !binding.path.isVariableDeclarator() || !binding.path.node.init) {
    return node;
  }

  return binding.path.node.init;
}

function containsPropertyName(node: t.Node, propertyName: string): boolean {
  if (t.isMemberExpression(node)) {
    if (!node.computed && t.isIdentifier(node.property, { name: propertyName })) {
      return true;
    }

    return containsPropertyName(node.object, propertyName);
  }

  if (t.isCallExpression(node)) {
    if (containsPropertyName(node.callee, propertyName)) {
      return true;
    }

    return node.arguments.some(
      (argument) => t.isExpression(argument) && containsPropertyName(argument, propertyName)
    );
  }

  if (t.isBinaryExpression(node)) {
    return (
      containsPropertyName(node.left, propertyName) ||
      containsPropertyName(node.right, propertyName)
    );
  }

  return false;
}
