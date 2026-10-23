export function Row({ value }: { value: unknown }) {
  return <td>{(value as any).label}</td>;
}
