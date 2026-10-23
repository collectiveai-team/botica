export function first<T extends any>(items: readonly T[]): T | undefined {
  return items[0];
}
