import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

export function fixturePath(name: string): string {
  return resolve(import.meta.dirname, 'fixtures', name);
}

export async function readFixture(name: string): Promise<string> {
  return readFile(fixturePath(name), 'utf8');
}
