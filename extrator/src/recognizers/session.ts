import traverseImport, { type NodePath } from '@babel/traverse';
import type { File } from '@babel/types';
import * as t from '@babel/types';

const traverse = ((traverseImport as { default?: unknown }).default ??
  traverseImport) as typeof import('@babel/traverse').default;

import type { SessionCookieGenerator } from '../manifest.js';
import { flattenPlusExpression } from '../evaluate.js';

export interface SessionRecognition {
  cookieName?: string;
  generator?: SessionCookieGenerator;
}

export function recognizeSessionSignals(
  ast: File,
  verifyFunctionName?: string
): SessionRecognition {
  const preferredCookieName = verifyFunctionName
    ? recognizeCookieNameFromVerifyFunction(ast, verifyFunctionName)
    : undefined;
  const recognizedCookie = recognizeCookieWriteAndGenerator(ast, preferredCookieName);

  return {
    cookieName: recognizedCookie?.cookieName ?? preferredCookieName,
    generator: recognizedCookie?.generator,
  };
}

function recognizeCookieNameFromVerifyFunction(
  ast: File,
  verifyFunctionName: string
): string | undefined {
  let cookieName: string | undefined;

  traverse(ast, {
    VariableDeclarator(path) {
      if (!t.isIdentifier(path.node.id, { name: verifyFunctionName })) {
        return;
      }

      const initPath = path.get('init');
      if (!initPath || Array.isArray(initPath) || !initPath.isFunction()) {
        return;
      }

      cookieName = extractCookieNameFromFunction(initPath);
      path.stop();
    },
    FunctionDeclaration(path) {
      if (!t.isIdentifier(path.node.id, { name: verifyFunctionName })) {
        return;
      }

      cookieName = extractCookieNameFromFunction(path);
      path.stop();
    },
  });

  return cookieName;
}

function extractCookieNameFromFunction(path: NodePath<t.Function>): string | undefined {
  const bodyPath = path.get('body');
  if (Array.isArray(bodyPath)) {
    return undefined;
  }

  let valuePath: NodePath<t.Node> | undefined;
  if (bodyPath.isBlockStatement()) {
    for (const statementPath of bodyPath.get('body')) {
      if (!statementPath.isReturnStatement()) {
        continue;
      }

      const argumentPath = statementPath.get('argument');
      if (!argumentPath || Array.isArray(argumentPath)) {
        continue;
      }

      valuePath = argumentPath as NodePath<t.Node>;
      break;
    }
  } else {
    valuePath = bodyPath as NodePath<t.Node>;
  }

  if (!valuePath) {
    return undefined;
  }

  const candidateCall = valuePath.isLogicalExpression() ? valuePath.get('left') : valuePath;
  if (!candidateCall.isCallExpression()) {
    return undefined;
  }

  const firstArgument = candidateCall.get('arguments.0');
  return firstArgument && !Array.isArray(firstArgument) && firstArgument.isStringLiteral()
    ? firstArgument.node.value
    : undefined;
}

function recognizeCookieWriteAndGenerator(
  ast: File,
  preferredCookieName?: string
):
  | {
      cookieName: string;
      generator: SessionCookieGenerator;
    }
  | undefined {
  let recognition:
    | {
        cookieName: string;
        generator: SessionCookieGenerator;
      }
    | undefined;

  traverse(ast, {
    Function(functionPath) {
      const cookieWrite = findCookieWrite(functionPath, preferredCookieName);
      if (!cookieWrite) {
        return;
      }

      const generator = findCookieGenerator(functionPath, cookieWrite.variableName);
      if (!generator) {
        return;
      }

      recognition = {
        cookieName: cookieWrite.cookieName,
        generator,
      };
      functionPath.stop();
    },
  });

  return recognition;
}

function findCookieWrite(
  functionPath: NodePath<t.Function>,
  preferredCookieName?: string
):
  | {
      cookieName: string;
      variableName: string;
    }
  | undefined {
  let result:
    | {
        cookieName: string;
        variableName: string;
      }
    | undefined;

  functionPath.traverse({
    Function(innerPath) {
      if (innerPath !== functionPath) {
        innerPath.skip();
      }
    },
    AssignmentExpression(path) {
      if (!isDocumentCookieAssignment(path.node.left)) {
        return;
      }

      const cookieWrite = extractCookieWrite(path.get('right'), preferredCookieName);
      if (!cookieWrite) {
        return;
      }

      result = cookieWrite;
      path.stop();
    },
  });

  return result;
}

function findCookieGenerator(
  functionPath: NodePath<t.Function>,
  variableName: string
): SessionCookieGenerator | undefined {
  let generator: SessionCookieGenerator | undefined;

  functionPath.traverse({
    Function(innerPath) {
      if (innerPath !== functionPath) {
        innerPath.skip();
      }
    },
    AssignmentExpression(path) {
      if (!t.isIdentifier(path.node.left, { name: variableName })) {
        return;
      }

      const segments = extractRandomSegments(path.get('right'));
      if (!segments) {
        return;
      }

      generator = {
        kind: 'random-base36-concat',
        segments,
      };
      path.stop();
    },
    VariableDeclarator(path) {
      if (!t.isIdentifier(path.node.id, { name: variableName })) {
        return;
      }

      const initPath = path.get('init');
      if (!initPath || Array.isArray(initPath)) {
        return;
      }

      const segments = extractRandomSegments(initPath as NodePath<t.Node>);
      if (!segments) {
        return;
      }

      generator = {
        kind: 'random-base36-concat',
        segments,
      };
      path.stop();
    },
  });

  return generator;
}

function isDocumentCookieAssignment(node: t.Node): boolean {
  return (
    t.isMemberExpression(node) &&
    t.isIdentifier(node.object, { name: 'document' }) &&
    !node.computed &&
    t.isIdentifier(node.property, { name: 'cookie' })
  );
}

function extractCookieWrite(
  valuePath: NodePath<t.Node>,
  preferredCookieName?: string
):
  | {
      cookieName: string;
      variableName: string;
    }
  | undefined {
  if (!valuePath.isTemplateLiteral()) {
    return undefined;
  }

  const [firstQuasi] = valuePath.node.quasis;
  const [firstExpression] = valuePath.get('expressions');
  if (!firstQuasi || !firstExpression || Array.isArray(firstExpression)) {
    return undefined;
  }

  const prefix = firstQuasi.value.cooked ?? '';
  const match = prefix.match(/^([^=]+)=/u);
  if (!match || !firstExpression.isIdentifier()) {
    return undefined;
  }

  const cookieName = match[1];
  if (!cookieName || (preferredCookieName && cookieName !== preferredCookieName)) {
    return undefined;
  }

  return {
    cookieName,
    variableName: firstExpression.node.name,
  };
}

function extractRandomSegments(
  valuePath: NodePath<t.Node>
): SessionCookieGenerator['segments'] | undefined {
  const parts = flattenPlusExpression(valuePath.node);
  const segments = parts
    .map((part) => parseRandomSegment(part))
    .filter(
      (segment): segment is SessionCookieGenerator['segments'][number] => segment !== undefined
    );

  return segments.length === parts.length ? segments : undefined;
}

function parseRandomSegment(node: t.Node): SessionCookieGenerator['segments'][number] | undefined {
  if (!t.isCallExpression(node) || !t.isMemberExpression(node.callee)) {
    return undefined;
  }

  const substringProperty =
    !node.callee.computed && t.isIdentifier(node.callee.property)
      ? node.callee.property.name
      : undefined;
  if (substringProperty !== 'substring') {
    return undefined;
  }

  const [startNode, endNode] = node.arguments;
  if (!t.isNumericLiteral(startNode) || !t.isNumericLiteral(endNode)) {
    return undefined;
  }

  const toStringCall = node.callee.object;
  if (!t.isCallExpression(toStringCall) || !t.isMemberExpression(toStringCall.callee)) {
    return undefined;
  }

  const toStringProperty =
    !toStringCall.callee.computed && t.isIdentifier(toStringCall.callee.property)
      ? toStringCall.callee.property.name
      : undefined;
  if (toStringProperty !== 'toString') {
    return undefined;
  }

  const [radixNode] = toStringCall.arguments;
  if (!t.isNumericLiteral(radixNode)) {
    return undefined;
  }

  const randomCall = toStringCall.callee.object;
  if (!t.isCallExpression(randomCall) || !t.isMemberExpression(randomCall.callee)) {
    return undefined;
  }

  if (
    !t.isIdentifier(randomCall.callee.object, { name: 'Math' }) ||
    randomCall.callee.computed ||
    !t.isIdentifier(randomCall.callee.property, { name: 'random' })
  ) {
    return undefined;
  }

  return {
    radix: radixNode.value,
    start: startNode.value,
    end: endNode.value,
  };
}
