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

  let returnExpression: t.Node | undefined;

  for (const statementPath of bodyPath.get('body')) {
    if (statementPath.isReturnStatement()) {
      returnExpression = statementPath.node.argument ?? undefined;
      break;
    }
  }

  if (!returnExpression) {
    return undefined;
  }

  const returnParts = flattenPlusExpression(returnExpression);
  const digestReturned = returnParts.at(-1);
  const prefixParts = returnParts.slice(0, -1);
  if (!digestReturned || prefixParts.length === 0) {
    return undefined;
  }

  const prefixValues = prefixParts.map((part) => resolveStaticString(functionPath.scope, part));
  if (prefixValues.some((value) => value === undefined)) {
    return undefined;
  }

  const digestMetadata = extractDigestMetadata(
    resolveBoundExpression(functionPath.scope, digestReturned)
  );
  if (!digestMetadata) {
    return undefined;
  }

  const digestParts = flattenPlusExpression(
    resolveBoundExpression(functionPath.scope, digestMetadata.digestArgument)
  );
  const dateExpression = digestParts[0];
  if (!dateExpression || !isUtcDateTemplate(functionPath.scope, dateExpression)) {
    return undefined;
  }

  const literalParts = digestParts
    .slice(1)
    .map((part) => resolveStaticString(functionPath.scope, part));
  if (literalParts.length === 0 || literalParts.some((value) => value === undefined)) {
    return undefined;
  }

  const [salt, ...suffixParts] = literalParts;
  if (salt === undefined) {
    return undefined;
  }

  const prefix = prefixValues.join('');
  const suffix = suffixParts.join('');

  if (digestMetadata.algorithm === 'MD5') {
    return {
      kind: 'utc-md5-derived',
      dateFormat: 'YYYY-MM-DD',
      prefix,
      salt,
      suffix,
      digestEncoding: 'hex',
      digestSlice: digestMetadata.digestSlice,
    };
  }

  return {
    kind: 'utc-sha256-derived',
    dateFormat: 'YYYY-MM-DD',
    prefix,
    salt,
    suffix,
    digestEncoding: 'hex',
    digestSlice: digestMetadata.digestSlice,
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

function resolveStaticString(
  scope: NodePath<t.Function>['scope'],
  node: t.Node
): string | undefined {
  const resolvedNode = resolveBoundExpression(scope, node);
  const directLiteral = extractStringLiteral(resolvedNode);
  if (directLiteral !== undefined) {
    return directLiteral;
  }

  const splitLiteral = extractSplitLiteral(resolvedNode);
  if (splitLiteral !== undefined) {
    return splitLiteral;
  }

  if (t.isBinaryExpression(resolvedNode, { operator: '+' })) {
    const parts = flattenPlusExpression(resolvedNode);
    const values = parts.map((part) => resolveStaticString(scope, part));
    if (values.some((value) => value === undefined)) {
      return undefined;
    }

    return values.join('');
  }

  if (!t.isCallExpression(resolvedNode) || !t.isMemberExpression(resolvedNode.callee)) {
    return undefined;
  }

  if (
    resolvedNode.callee.computed ||
    !t.isIdentifier(resolvedNode.callee.property, { name: 'join' })
  ) {
    return undefined;
  }

  const [delimiter] = resolvedNode.arguments;
  if (!t.isStringLiteral(delimiter, { value: '' })) {
    return undefined;
  }

  return resolveStaticString(scope, resolvedNode.callee.object);
}

function extractStringLiteral(node: t.Node): string | undefined {
  if (t.isStringLiteral(node)) {
    return node.value;
  }

  if (t.isTemplateLiteral(node) && node.expressions.length === 0) {
    return node.quasis.map((quasi) => quasi.value.cooked ?? '').join('');
  }

  return undefined;
}

function extractSplitLiteral(node: t.Node): string | undefined {
  if (!t.isCallExpression(node) || !t.isMemberExpression(node.callee)) {
    return undefined;
  }

  const [firstArgument] = node.arguments;
  if (
    node.callee.computed ||
    !t.isStringLiteral(node.callee.object) ||
    !t.isIdentifier(node.callee.property, { name: 'split' }) ||
    !t.isStringLiteral(firstArgument, { value: '' })
  ) {
    return undefined;
  }

  return node.callee.object.value;
}

function extractDigestMetadata(node: t.Node):
  | {
      algorithm: 'MD5' | 'SHA256';
      digestSlice: PassphraseStrategy['digestSlice'];
      digestArgument: t.Node;
    }
  | undefined {
  if (!t.isCallExpression(node) || !t.isMemberExpression(node.callee)) {
    return undefined;
  }

  const digestSliceProperty =
    !node.callee.computed && t.isIdentifier(node.callee.property)
      ? node.callee.property.name
      : undefined;
  if (digestSliceProperty !== 'substring' && digestSliceProperty !== 'slice') {
    return undefined;
  }

  const [startArgument, endArgument] = node.arguments;
  if (!t.isNumericLiteral(startArgument) || !t.isNumericLiteral(endArgument)) {
    return undefined;
  }

  const toStringCall = node.callee.object;
  if (!t.isCallExpression(toStringCall) || !t.isMemberExpression(toStringCall.callee)) {
    return undefined;
  }

  if (
    toStringCall.callee.computed ||
    !t.isIdentifier(toStringCall.callee.property, { name: 'toString' })
  ) {
    return undefined;
  }

  const digestCall = toStringCall.callee.object;
  if (!t.isCallExpression(digestCall) || !t.isMemberExpression(digestCall.callee)) {
    return undefined;
  }

  const algorithm =
    !digestCall.callee.computed && t.isIdentifier(digestCall.callee.property)
      ? digestCall.callee.property.name === 'MD5' || digestCall.callee.property.name === 'SHA256'
        ? digestCall.callee.property.name
        : undefined
      : undefined;
  if (!algorithm) {
    return undefined;
  }

  const [digestArgument] = digestCall.arguments;
  if (!digestArgument || !t.isExpression(digestArgument)) {
    return undefined;
  }

  return {
    algorithm,
    digestSlice: {
      start: startArgument.value,
      end: endArgument.value,
    },
    digestArgument,
  };
}

function isUtcDateTemplate(scope: NodePath<t.Function>['scope'], node: t.Node): boolean {
  const resolvedNode = resolveBoundExpression(scope, node);
  if (!t.isTemplateLiteral(resolvedNode) || resolvedNode.expressions.length !== 3) {
    return false;
  }

  const [yearExpression, monthExpression, dayExpression] = resolvedNode.expressions;
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
