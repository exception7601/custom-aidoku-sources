import traverseImport, { type NodePath } from '@babel/traverse';
import type { File } from '@babel/types';
import * as t from '@babel/types';

const traverse = ((traverseImport as { default?: unknown }).default ??
  traverseImport) as typeof import('@babel/traverse').default;

import type { DynamicSignatureStrategy, SignatureRule } from '../manifest.js';
import { evaluateStatic } from '../evaluate.js';

export interface RequestRecognition {
  signatureHeader?: string;
  signatureRules: SignatureRule[];
  signatureStrategy?: DynamicSignatureStrategy;
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
      recognition.signatureStrategy = extractDynamicSignatureStrategy(operation.valuePath);
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

function extractDynamicSignatureStrategy(
  valuePath: NodePath<t.Node>
): DynamicSignatureStrategy | undefined {
  const signatureCallPath = unwrapValuePath(valuePath);
  if (!signatureCallPath.isCallExpression()) {
    return undefined;
  }

  const calleePath = signatureCallPath.get('callee');
  if (!calleePath.isIdentifier({ name: 'btoa' })) {
    return undefined;
  }

  const payloadPath = signatureCallPath.get('arguments.0');
  if (!payloadPath || Array.isArray(payloadPath) || !payloadPath.isTemplateLiteral()) {
    return undefined;
  }

  const [timestampExpressionPath, digestExpressionPath] = payloadPath.get('expressions');
  if (
    !timestampExpressionPath ||
    !digestExpressionPath ||
    Array.isArray(timestampExpressionPath) ||
    Array.isArray(digestExpressionPath)
  ) {
    return undefined;
  }

  const timestampDivisor = extractTimestampDivisor(timestampExpressionPath);
  if (!timestampDivisor) {
    return undefined;
  }

  const digestBindingPath = unwrapValuePath(digestExpressionPath);
  if (!digestBindingPath.isCallExpression()) {
    return undefined;
  }

  const digestCalleePath = digestBindingPath.get('callee');
  if (!digestCalleePath.isMemberExpression()) {
    return undefined;
  }

  const digestPropertyPath = digestCalleePath.get('property');
  if (Array.isArray(digestPropertyPath)) {
    return undefined;
  }

  const digestPropertyName = !digestCalleePath.node.computed
    ? digestPropertyPath.isIdentifier()
      ? digestPropertyPath.node.name
      : undefined
    : evaluateStatic(digestPropertyPath);
  if (digestPropertyName !== 'toString') {
    return undefined;
  }

  const shaCallPath = digestCalleePath.get('object');
  if (Array.isArray(shaCallPath) || !shaCallPath.isCallExpression()) {
    return undefined;
  }

  const shaCalleePath = shaCallPath.get('callee');
  if (!shaCalleePath.isMemberExpression()) {
    return undefined;
  }

  const shaPropertyPath = shaCalleePath.get('property');
  if (Array.isArray(shaPropertyPath)) {
    return undefined;
  }

  const shaPropertyName = !shaCalleePath.node.computed
    ? shaPropertyPath.isIdentifier()
      ? shaPropertyPath.node.name
      : undefined
    : evaluateStatic(shaPropertyPath);
  if (shaPropertyName !== 'SHA256') {
    return undefined;
  }

  const payloadArgumentPath = shaCallPath.get('arguments.0');
  if (!payloadArgumentPath || Array.isArray(payloadArgumentPath)) {
    return undefined;
  }

  const payloadBindingPath = unwrapValuePath(payloadArgumentPath);
  if (!payloadBindingPath.isTemplateLiteral()) {
    return undefined;
  }

  const payloadExpressions = payloadBindingPath.get('expressions');
  const routeKindPath = payloadExpressions[1];
  const saltPath = payloadExpressions[2];
  if (!routeKindPath || !saltPath || Array.isArray(routeKindPath) || Array.isArray(saltPath)) {
    return undefined;
  }

  const routeSelector = extractRouteSelector(routeKindPath);
  const salt = extractStaticString(saltPath);
  if (!routeSelector || !salt) {
    return undefined;
  }

  return {
    kind: 'time-sha256-base64',
    timestampDivisor,
    salt,
    routeSelector,
  };
}

function extractTimestampDivisor(path: NodePath<t.Node>): number | undefined {
  const bindingPath = unwrapValuePath(path);
  if (!bindingPath.isCallExpression()) {
    return undefined;
  }

  const calleePath = bindingPath.get('callee');
  if (!calleePath.isMemberExpression()) {
    return undefined;
  }

  const objectPath = calleePath.get('object');
  const propertyPath = calleePath.get('property');
  if (Array.isArray(objectPath) || Array.isArray(propertyPath)) {
    return undefined;
  }

  const propertyName = !calleePath.node.computed
    ? propertyPath.isIdentifier()
      ? propertyPath.node.name
      : undefined
    : evaluateStatic(propertyPath);
  if (propertyName !== 'floor') {
    return undefined;
  }

  if (!objectPath.isIdentifier({ name: 'Math' })) {
    return undefined;
  }

  const firstArgument = bindingPath.get('arguments.0');
  if (!firstArgument || Array.isArray(firstArgument) || !firstArgument.isBinaryExpression()) {
    return undefined;
  }

  if (firstArgument.node.operator !== '/') {
    return undefined;
  }

  const leftPath = firstArgument.get('left');
  const rightPath = firstArgument.get('right');
  if (Array.isArray(leftPath) || Array.isArray(rightPath)) {
    return undefined;
  }

  if (!isDateNowCall(leftPath)) {
    return undefined;
  }

  const divisorValue = evaluateStatic(rightPath);
  return typeof divisorValue === 'number' ? divisorValue : undefined;
}

function extractRouteSelector(path: NodePath<t.Node>): DynamicSignatureStrategy['routeSelector'] | undefined {
  const bindingPath = unwrapValuePath(path);
  if (!bindingPath.isConditionalExpression()) {
    return undefined;
  }

  const whenMatched = evaluateStatic(bindingPath.get('consequent'));
  const otherwise = evaluateStatic(bindingPath.get('alternate'));
  const when = extractUrlCondition(bindingPath.get('test'));
  if (
    typeof whenMatched !== 'string' ||
    typeof otherwise !== 'string' ||
    when === undefined
  ) {
    return undefined;
  }

  return {
    whenUrlContains: when.urlContains,
    whenMatched,
    otherwise,
  };
}

function extractStaticString(path: NodePath<t.Node>): string | undefined {
  const value = evaluateStatic(unwrapValuePath(path));
  return typeof value === 'string' ? value : undefined;
}

function isDateNowCall(path: NodePath<t.Node>): boolean {
  if (!path.isCallExpression()) {
    return false;
  }

  const calleePath = path.get('callee');
  if (!calleePath.isMemberExpression()) {
    return false;
  }

  const objectPath = calleePath.get('object');
  const propertyPath = calleePath.get('property');
  if (Array.isArray(objectPath) || Array.isArray(propertyPath)) {
    return false;
  }

  return (
    objectPath.isIdentifier({ name: 'Date' }) &&
    !calleePath.node.computed &&
    propertyPath.isIdentifier({ name: 'now' })
  );
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
