export function run(): void {
  try {
    work();
  } catch (error) {
    console.log(error);
  }
  console.error("failed");
}
