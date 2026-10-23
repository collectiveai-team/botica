export function wrap(text: string, width: number): string[] {
  return text.match(new RegExp(`.{1,${width}}`, "g")) ?? [];
}
