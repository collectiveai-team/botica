export function run(): string {
  return "done";
}

if (import.meta.main) {
  console.log(run());
}
