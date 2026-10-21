export function Login({ onToken }: { onToken: (token: string) => void }) {
  const save = (token: string) => {
    localStorage.setItem("SESSION", token);
    onToken(token);
  };
  return <button onClick={() => save("x")}>Sign in</button>;
}
