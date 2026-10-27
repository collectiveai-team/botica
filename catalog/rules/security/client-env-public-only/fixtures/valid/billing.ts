import "server-only";

export async function getPlan(): Promise<string> {
  const response = await fetch("https://billing.example.com/plan", {
    headers: { authorization: `Bearer ${process.env.BILLING_KEY}` },
  });
  return response.text();
}
