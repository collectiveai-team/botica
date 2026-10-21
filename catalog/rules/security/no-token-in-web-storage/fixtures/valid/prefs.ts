export function remember(theme: string, key: string, value: string): void {
  localStorage.setItem("theme", theme);
  sessionStorage.setItem(key, value);
  localStorage.lastPage = "/orders";
  const token = localStorage.getItem("theme");
  void token;
}

export async function orders(): Promise<Response> {
  return fetch("/api/orders", { credentials: "include" });
}
