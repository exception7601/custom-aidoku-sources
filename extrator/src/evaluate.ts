import type { NodePath, Scope } from '@babel/traverse';
import * as t from '@babel/types';

export type StaticPrimitive = string | number | boolean | null;
export type StaticValue = StaticPrimitive | StaticValue[] | StaticFunction;

type AnyNodePath = NodePath<t.Node | null | undefined>;

interface StaticFunction {
  kind: 'function';
  path: NodePath<t.Function>;
  scope: Scope;
}

function isStaticFunction(value: StaticValue | undefined): value is StaticFunction {
  return (
    typeof value === 'object' &&
    value !== null &&
    !Array.isArray(value) &&
    value.kind === 'function'
  );
}

export function flattenPlusExpression(node: t.Node): t.Node[] {
  if (t.isBinaryExpression(node, { operator: '+' })) {
    return [...flattenPlusExpression(node.left), ...flattenPlusExpression(node.right)];
  }

  return [node];
}

export function getIdentifierName(node: t.Node | null | undefined): string | undefined {
  return t.isIdentifier(node) ? node.name : undefined;
}

export function evaluateStatic(
  path: AnyNodePath,
  context: ReadonlyMap<string, StaticValue> = new Map()
): StaticValue | undefined {
  const node = path.node;
  if (!node) {
    return undefined;
  }

  if (t.isStringLiteral(node)) {
    return node.value;
  }

  if (t.isNumericLiteral(node)) {
    return node.value;
  }

  if (t.isBooleanLiteral(node)) {
    return node.value;
  }

  if (t.isNullLiteral(node)) {
    return null;
  }

  if (t.isIdentifier(node)) {
    if (context.has(node.name)) {
      return context.get(node.name);
    }

    return evaluateBinding(path.scope, node.name, context);
  }

  if (t.isMemberExpression(node)) {
    return evaluateMemberExpression(path as NodePath<t.MemberExpression>, context);
  }

  if (t.isArrayExpression(node)) {
    const output: StaticValue[] = [];

    for (const [index, element] of node.elements.entries()) {
      if (element === null) {
        return undefined;
      }

      const elementPath = path.get(`elements.${index}`);
      if (!elementPath || Array.isArray(elementPath)) {
        return undefined;
      }

      const value = evaluateStatic(elementPath as AnyNodePath, context);
      if (value === undefined) {
        return undefined;
      }

      output.push(value);
    }

    return output;
  }

  if (t.isTemplateLiteral(node)) {
    let output = '';

    for (const [index, quasi] of node.quasis.entries()) {
      output += quasi.value.cooked ?? '';

      if (index >= node.expressions.length) {
        continue;
      }

      const expressionPath = path.get(`expressions.${index}`);
      if (!expressionPath || Array.isArray(expressionPath)) {
        return undefined;
      }

      const expressionValue = evaluateStatic(expressionPath as AnyNodePath, context);
      if (typeof expressionValue !== 'string' && typeof expressionValue !== 'number') {
        return undefined;
      }

      output += String(expressionValue);
    }

    return output;
  }

  if (t.isBinaryExpression(node, { operator: '+' })) {
    const left = evaluateStatic(path.get('left') as AnyNodePath, context);
    const right = evaluateStatic(path.get('right') as AnyNodePath, context);
    if (left === undefined || right === undefined) {
      return undefined;
    }

    if (
      (typeof left === 'string' || typeof left === 'number') &&
      (typeof right === 'string' || typeof right === 'number')
    ) {
      return String(left) + String(right);
    }

    return undefined;
  }

  if (t.isLogicalExpression(node)) {
    const left = evaluateStatic(path.get('left') as AnyNodePath, context);
    if (left === undefined) {
      return undefined;
    }

    if (node.operator === '||') {
      return left || evaluateStatic(path.get('right') as AnyNodePath, context);
    }

    if (node.operator === '&&') {
      return left && evaluateStatic(path.get('right') as AnyNodePath, context);
    }
  }

  if (t.isConditionalExpression(node)) {
    const test = evaluateStatic(path.get('test') as AnyNodePath, context);
    if (typeof test !== 'boolean') {
      return undefined;
    }

    return test
      ? evaluateStatic(path.get('consequent') as AnyNodePath, context)
      : evaluateStatic(path.get('alternate') as AnyNodePath, context);
  }

  if (
    t.isArrowFunctionExpression(node) ||
    t.isFunctionExpression(node) ||
    t.isFunctionDeclaration(node)
  ) {
    return {
      kind: 'function',
      path: path as NodePath<t.Function>,
      scope: path.scope,
    };
  }

  if (t.isCallExpression(node)) {
    return evaluateCallExpression(path as NodePath<t.CallExpression>, context);
  }

  return undefined;
}

function evaluateMemberExpression(
  path: NodePath<t.MemberExpression>,
  _context: ReadonlyMap<string, StaticValue>
): StaticValue | undefined {
  if (!path.node.computed && t.isIdentifier(path.node.object, { name: 'Math' })) {
    if (t.isIdentifier(path.node.property, { name: 'PI' })) {
      return Math.PI;
    }
  }

  return undefined;
}

function evaluateBinding(
  scope: Scope,
  name: string,
  context: ReadonlyMap<string, StaticValue>
): StaticValue | undefined {
  const binding = scope.getBinding(name);
  if (!binding) {
    return undefined;
  }

  if (binding.path.isVariableDeclarator()) {
    const initPath = binding.path.get('init');
    if (!initPath || Array.isArray(initPath)) {
      return undefined;
    }

    return evaluateStatic(initPath as AnyNodePath, context);
  }

  if (binding.path.isFunctionDeclaration()) {
    return {
      kind: 'function',
      path: binding.path,
      scope: binding.path.scope,
    };
  }

  return undefined;
}

function evaluateCallExpression(
  path: NodePath<t.CallExpression>,
  context: ReadonlyMap<string, StaticValue>
): StaticValue | undefined {
  const calleePath = path.get('callee');

  if (calleePath.isIdentifier({ name: 'String' })) {
    const firstArgument = path.get('arguments.0');
    if (!firstArgument || Array.isArray(firstArgument)) {
      return undefined;
    }

    const value = evaluateStatic(firstArgument as AnyNodePath, context);
    if (value === undefined || isStaticFunction(value)) {
      return undefined;
    }

    return String(value);
  }

  if (calleePath.isIdentifier({ name: 'btoa' })) {
    const firstArgument = path.get('arguments.0');
    if (!firstArgument || Array.isArray(firstArgument)) {
      return undefined;
    }

    const value = evaluateStatic(firstArgument as AnyNodePath, context);
    return typeof value === 'string' ? Buffer.from(value).toString('base64') : undefined;
  }

  if (calleePath.isMemberExpression()) {
    const memberValue = evaluateMemberCall(path, calleePath, context);
    if (memberValue !== undefined) {
      return memberValue;
    }
  }

  const calleeValue = evaluateStatic(calleePath as AnyNodePath, context);
  if (!isStaticFunction(calleeValue)) {
    return undefined;
  }

  return invokeFunction(calleeValue, path, context);
}

function evaluateMemberCall(
  path: NodePath<t.CallExpression>,
  calleePath: NodePath<t.MemberExpression>,
  context: ReadonlyMap<string, StaticValue>
): StaticValue | undefined {
  const propertyName = resolvePropertyName(calleePath, context);
  if (!propertyName) {
    return undefined;
  }

  if (
    calleePath.get('object').isIdentifier({ name: 'String' }) &&
    propertyName === 'fromCharCode'
  ) {
    const codes = path
      .get('arguments')
      .map((argumentPath) => evaluateStatic(argumentPath as AnyNodePath, context));

    if (!codes.every((value): value is number => typeof value === 'number')) {
      return undefined;
    }

    return String.fromCharCode(...codes);
  }

  const objectValue = evaluateStatic(calleePath.get('object') as AnyNodePath, context);

  switch (propertyName) {
    case 'split':
      return evaluateSplit(path, objectValue, context);
    case 'join':
      return evaluateJoin(path, objectValue, context);
    case 'substring':
      return evaluateSubstring(path, objectValue, context);
    case 'slice':
      return evaluateSlice(path, objectValue, context);
    case 'padStart':
      return evaluatePadStart(path, objectValue, context);
    case 'toUpperCase':
      return typeof objectValue === 'string' ? objectValue.toUpperCase() : undefined;
    case 'toLowerCase':
      return typeof objectValue === 'string' ? objectValue.toLowerCase() : undefined;
    case 'includes':
      return evaluateIncludes(path, objectValue, context);
    case 'reduce':
      return evaluateReduce(path, objectValue, context);
    case 'toString':
      return evaluateToString(path, objectValue, context);
    default:
      return undefined;
  }
}

function resolvePropertyName(
  memberPath: NodePath<t.MemberExpression>,
  context: ReadonlyMap<string, StaticValue>
): string | undefined {
  if (!memberPath.node.computed && t.isIdentifier(memberPath.node.property)) {
    return memberPath.node.property.name;
  }

  const propertyPath = memberPath.get('property');
  if (!propertyPath || Array.isArray(propertyPath)) {
    return undefined;
  }

  const propertyValue = evaluateStatic(propertyPath as AnyNodePath, context);
  return typeof propertyValue === 'string' ? propertyValue : undefined;
}

function evaluateSplit(
  path: NodePath<t.CallExpression>,
  objectValue: StaticValue | undefined,
  context: ReadonlyMap<string, StaticValue>
): StaticValue | undefined {
  if (typeof objectValue !== 'string') {
    return undefined;
  }

  const delimiterPath = path.get('arguments.0');
  if (!delimiterPath || Array.isArray(delimiterPath)) {
    return undefined;
  }

  const delimiter = evaluateStatic(delimiterPath as AnyNodePath, context);
  return typeof delimiter === 'string' ? objectValue.split(delimiter) : undefined;
}

function evaluateJoin(
  path: NodePath<t.CallExpression>,
  objectValue: StaticValue | undefined,
  context: ReadonlyMap<string, StaticValue>
): StaticValue | undefined {
  if (!Array.isArray(objectValue)) {
    return undefined;
  }

  const delimiterPath = path.get('arguments.0');
  if (!delimiterPath || Array.isArray(delimiterPath)) {
    return undefined;
  }

  const delimiter = evaluateStatic(delimiterPath as AnyNodePath, context);
  if (typeof delimiter !== 'string') {
    return undefined;
  }

  if (!objectValue.every((value) => typeof value === 'string' || typeof value === 'number')) {
    return undefined;
  }

  return objectValue.join(delimiter);
}

function evaluateSubstring(
  path: NodePath<t.CallExpression>,
  objectValue: StaticValue | undefined,
  context: ReadonlyMap<string, StaticValue>
): StaticValue | undefined {
  if (typeof objectValue !== 'string') {
    return undefined;
  }

  const start = getNumericArgument(path, 0, context);
  const end = getNumericArgument(path, 1, context);
  if (start === undefined || end === undefined) {
    return undefined;
  }

  return objectValue.substring(start, end);
}

function evaluateSlice(
  path: NodePath<t.CallExpression>,
  objectValue: StaticValue | undefined,
  context: ReadonlyMap<string, StaticValue>
): StaticValue | undefined {
  const start = getNumericArgument(path, 0, context);
  const end = getNumericArgument(path, 1, context);
  if (start === undefined) {
    return undefined;
  }

  if (typeof objectValue === 'string') {
    return objectValue.slice(start, end);
  }

  if (Array.isArray(objectValue)) {
    return objectValue.slice(start, end);
  }

  return undefined;
}

function evaluatePadStart(
  path: NodePath<t.CallExpression>,
  objectValue: StaticValue | undefined,
  context: ReadonlyMap<string, StaticValue>
): StaticValue | undefined {
  if (typeof objectValue !== 'string') {
    return undefined;
  }

  const targetLength = getNumericArgument(path, 0, context);
  const padPath = path.get('arguments.1');
  if (targetLength === undefined || !padPath || Array.isArray(padPath)) {
    return undefined;
  }

  const padValue = evaluateStatic(padPath as AnyNodePath, context);
  return typeof padValue === 'string' ? objectValue.padStart(targetLength, padValue) : undefined;
}

function evaluateIncludes(
  path: NodePath<t.CallExpression>,
  objectValue: StaticValue | undefined,
  context: ReadonlyMap<string, StaticValue>
): StaticValue | undefined {
  const searchPath = path.get('arguments.0');
  if (!searchPath || Array.isArray(searchPath)) {
    return undefined;
  }

  const searchValue = evaluateStatic(searchPath as AnyNodePath, context);
  if (typeof searchValue !== 'string') {
    return undefined;
  }

  if (typeof objectValue === 'string') {
    return objectValue.includes(searchValue);
  }

  if (Array.isArray(objectValue)) {
    return objectValue.includes(searchValue);
  }

  return undefined;
}

function evaluateReduce(
  path: NodePath<t.CallExpression>,
  objectValue: StaticValue | undefined,
  context: ReadonlyMap<string, StaticValue>
): StaticValue | undefined {
  if (!Array.isArray(objectValue)) {
    return undefined;
  }

  const callbackPath = path.get('arguments.0');
  const initialValuePath = path.get('arguments.1');
  if (
    !callbackPath ||
    Array.isArray(callbackPath) ||
    !initialValuePath ||
    Array.isArray(initialValuePath)
  ) {
    return undefined;
  }

  const callbackValue = evaluateStatic(callbackPath as AnyNodePath, context);
  let accumulator = evaluateStatic(initialValuePath as AnyNodePath, context);
  if (!isStaticFunction(callbackValue) || accumulator === undefined) {
    return undefined;
  }

  const params = callbackValue.path.node.params;
  if (params.length < 2 || !t.isIdentifier(params[0]) || !t.isIdentifier(params[1])) {
    return undefined;
  }

  for (const item of objectValue) {
    const localContext = new Map(context);
    localContext.set(params[0].name, accumulator);
    localContext.set(params[1].name, item);

    const bodyPath = callbackValue.path.get('body');
    if (Array.isArray(bodyPath)) {
      return undefined;
    }

    accumulator = bodyPath.isBlockStatement()
      ? evaluateReturnedValue(bodyPath, localContext)
      : evaluateStatic(bodyPath as AnyNodePath, localContext);

    if (accumulator === undefined) {
      return undefined;
    }
  }

  return accumulator;
}

function evaluateToString(
  path: NodePath<t.CallExpression>,
  objectValue: StaticValue | undefined,
  context: ReadonlyMap<string, StaticValue>
): StaticValue | undefined {
  if (typeof objectValue !== 'number') {
    return undefined;
  }

  const radixPath = path.get('arguments.0');
  if (!radixPath || Array.isArray(radixPath)) {
    return objectValue.toString();
  }

  const radixValue = evaluateStatic(radixPath as AnyNodePath, context);
  return typeof radixValue === 'number' ? objectValue.toString(radixValue) : undefined;
}

function invokeFunction(
  fn: StaticFunction,
  callPath: NodePath<t.CallExpression>,
  parentContext: ReadonlyMap<string, StaticValue>
): StaticValue | undefined {
  const nextContext = new Map(parentContext);

  for (const [index, param] of fn.path.node.params.entries()) {
    if (!t.isIdentifier(param)) {
      return undefined;
    }

    const argumentPath = callPath.get(`arguments.${index}`);
    if (!argumentPath || Array.isArray(argumentPath)) {
      continue;
    }

    const argumentValue = evaluateStatic(argumentPath as AnyNodePath, parentContext);
    if (argumentValue !== undefined) {
      nextContext.set(param.name, argumentValue);
    }
  }

  const bodyPath = fn.path.get('body');
  if (Array.isArray(bodyPath)) {
    return undefined;
  }

  return bodyPath.isBlockStatement()
    ? evaluateReturnedValue(bodyPath, nextContext)
    : evaluateStatic(bodyPath as AnyNodePath, nextContext);
}

function evaluateReturnedValue(
  blockPath: NodePath<t.BlockStatement>,
  context: ReadonlyMap<string, StaticValue>
): StaticValue | undefined {
  for (const statementPath of blockPath.get('body')) {
    if (!statementPath.isReturnStatement()) {
      continue;
    }

    const argumentPath = statementPath.get('argument');
    if (!argumentPath || Array.isArray(argumentPath)) {
      return undefined;
    }

    return evaluateStatic(argumentPath as AnyNodePath, context);
  }

  return undefined;
}

function getNumericArgument(
  path: NodePath<t.CallExpression>,
  index: number,
  context: ReadonlyMap<string, StaticValue>
): number | undefined {
  const argumentPath = path.get(`arguments.${index}`);
  if (!argumentPath || Array.isArray(argumentPath)) {
    return undefined;
  }

  const value = evaluateStatic(argumentPath as AnyNodePath, context);
  return typeof value === 'number' ? value : undefined;
}
