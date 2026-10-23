export type Payload = Record<string, any>;

export function total(items: any[]): number {
  return items.length;
}

export async function fetchUser(id: string): Promise<any> {
  const response = await fetch(`/users/${id}`);
  return response.json();
}

export function keys<T extends Record<string, any>>(value: T): string[] {
  return Object.keys(value);
}
