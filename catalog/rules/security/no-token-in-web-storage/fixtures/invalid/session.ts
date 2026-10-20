export function keep(token: string, refresh: string): void {
  localStorage.setItem("accessToken", token);
  window.sessionStorage.setItem("refresh_token", refresh);
  sessionStorage.authToken = token;
  globalThis.localStorage["jwt"] = token;
}
