import traverseImport, { type NodePath } from '@babel/traverse';
import type { File } from '@babel/types';
import * as t from '@babel/types';

const traverse = ((traverseImport as { default?: unknown }).default ??
  traverseImport) as typeof import('@babel/traverse').default;

import type { SignatureRule } from '../manifest.js';
import { evaluateStatic } from '../evaluate.js';

export interface RequestRecognition {
  signatureHeader?: string;
  signatureRules: SignatureRule[];
  verifyHeader?: string;
  verifyFunctionName?: string;
  dataKeyHeader?: string;
}

interface HeaderOperation {
  headerName: string;
  valuePath: NodePath<t.Node>;
}

export function recognizeRequestSignals(ast: File): RequestRecognition {
  const operations: HeaderOperation[] = [];
  let dataKeyHeader: string | undefined;

  traverse(ast, {
    Function(functionPath) {
      const headerNames = new Set<string>();

      functionPath.traverse({
        Function(innerPath) {
          if (innerPath !== functionPath) {
            innerPath.skip();
          }
        },
        VariableDeclarator(variablePath) {
          const identifier = variablePath.node.id;
          if (!t.isIdentifier(identifier)) {
            return;
          }

          const initPath = variablePath.get('init');
          if (!initPath || Array.isArray(initPath) || !initPath.isNewExpression()) {
            return;
          }

          if (!initPath.get('callee').isIdentifier({ name: 'Headers' })) {
            return;
          }

          headerNames.add(identifier.name);
        },
        CallExpression(callPath) {
          const calleePath = callPath.get('callee');
          if (!calleePath.isMemberExpression()) {
            return;
          }

          const objectPath = calleePath.get('object');
          if (!objectPath.isIdentifier() || !headerNames.has(objectPath.node.name)) {
            return;
          }

          const methodName = resolveMethodName(calleePath);
          if (methodName !== 'append' && methodName !== 'set') {
            return;
          }

          const [headerNamePath, valuePath] = callPath.get('arguments');
          if (
            !headerNamePath ||
            !valuePath ||
            Array.isArray(headerNamePath) ||
            Array.isArray(valuePath)
          ) {
            return;
          }

          const headerName = evaluateStatic(headerNamePath);
          if (typeof headerName !== 'string') {
            return;
          }

          operations.push({
            headerName,
            valuePath: valuePath as NodePath<t.Node>,
          });
        },
      });
    },
    CallExpression(callPath) {
      const calleePath = callPath.get('callee');
      if (!calleePath.isMemberExpression()) {
        return;
      }

      const propertyPath = calleePath.get('property');
      if (Array.isArray(propertyPath)) {
        return;
      }

      const propertyName = !calleePath.node.computed
        ? propertyPath.isIdentifier()
          ? propertyPath.node.name
          : undefined
        : evaluateStatic(propertyPath);

      if (propertyName !== 'get') {
        return;
      }

      const firstArgument = callPath.get('arguments.0');
      if (!firstArgument || Array.isArray(firstArgument)) {
        return;
      }

      const headerName = evaluateStatic(firstArgument);
      if (typeof headerName === 'string' && headerName.includes('datakey')) {
        dataKeyHeader = headerName;
      }
    },
  });

  const recognition: RequestRecognition = {
    signatureRules: [],
    dataKeyHeader,
  };

  for (const operation of operations) {
    if (operation.headerName.includes('signature')) {
      recognition.signatureHeader = operation.headerName;
      recognition.signatureRules = extractSignatureRules(operation.valuePath);
      continue;
    }

    if (operation.headerName.includes('verify')) {
      recognition.verifyHeader = operation.headerName;
      recognition.verifyFunctionName = extractVerifyFunctionName(operation.valuePath);
    }
  }

  return recognition;
}

function resolveMethodName(path: NodePath<t.MemberExpression>): string | undefined {
  if (!path.node.computed && t.isIdentifier(path.node.property)) {
    return path.node.property.name;
  }

  const propertyPath = path.get('property');
  if (Array.isArray(propertyPath)) {
    return undefined;
  }

  const propertyValue = evaluateStatic(propertyPath);
  return typeof propertyValue === 'string' ? propertyValue : undefined;
}

function extractSignatureRules(valuePath: NodePath<t.Node>): SignatureRule[] {
  const unwrapped = unwrapValuePath(valuePath);

  if (unwrapped.isConditionalExpression()) {
    const consequent = evaluateStatic(unwrapped.get('consequent'));
    const alternate = evaluateStatic(unwrapped.get('alternate'));
    const condition = extractUrlCondition(unwrapped.get('test'));

    if (
      typeof consequent === 'string' &&
      typeof alternate === 'string' &&
      condition !== undefined
    ) {
      return [
        {
          when: condition,
          value: consequent,
        },
        {
          default: true,
          value: alternate,
        },
      ];
    }
  }

  const directValue = evaluateStatic(unwrapped);
  return typeof directValue === 'string'
    ? [
        {
          default: true,
          value: directValue,
        },
      ]
    : [];
}

function extractVerifyFunctionName(valuePath: NodePath<t.Node>): string | undefined {
  const unwrapped = unwrapValuePath(valuePath);
  if (!unwrapped.isCallExpression()) {
    return undefined;
  }

  const calleePath = unwrapped.get('callee');
  return calleePath.isIdentifier() ? calleePath.node.name : undefined;
}

function extractUrlCondition(testPath: NodePath<t.Node>):
  | {
      urlContains: string;
    }
  | undefined {
  const unwrapped = unwrapValuePath(testPath);
  if (!unwrapped.isCallExpression()) {
    return undefined;
  }

  const calleePath = unwrapped.get('callee');
  if (!calleePath.isMemberExpression()) {
    return undefined;
  }

  const propertyPath = calleePath.get('property');
  if (Array.isArray(propertyPath)) {
    return undefined;
  }

  const propertyName = !calleePath.node.computed
    ? propertyPath.isIdentifier()
      ? propertyPath.node.name
      : undefined
    : evaluateStatic(propertyPath);

  if (propertyName !== 'includes') {
    return undefined;
  }

  const firstArgument = unwrapped.get('arguments.0');
  if (!firstArgument || Array.isArray(firstArgument)) {
    return undefined;
  }

  const includedValue = evaluateStatic(firstArgument);
  return typeof includedValue === 'string' ? { urlContains: includedValue } : undefined;
}

function unwrapValuePath(path: NodePath<t.Node>): NodePath<t.Node> {
  if (!path.isIdentifier()) {
    return path;
  }

  const binding = path.scope.getBinding(path.node.name);
  if (!binding || !binding.path.isVariableDeclarator()) {
    return path;
  }

  const initPath = binding.path.get('init');
  if (!initPath || Array.isArray(initPath)) {
    return path;
  }

  return initPath as NodePath<t.Node>;
}
