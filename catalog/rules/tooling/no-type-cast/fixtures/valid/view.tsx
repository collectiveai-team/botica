export function Field({ event }: { event: Event }) {
  const input = event.target as HTMLInputElement;
  return <span>{input.value}</span>;
}
