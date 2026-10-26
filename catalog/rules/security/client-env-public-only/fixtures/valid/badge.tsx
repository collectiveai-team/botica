"use client";

export function Badge({ plan }: { plan: string }) {
  const verbose = process.env.NODE_ENV === "development";
  return <span data-site={process.env.NEXT_PUBLIC_SITE_NAME}>{verbose ? `plan: ${plan}` : plan}</span>;
}
