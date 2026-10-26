"use client";

export function PlanBadge() {
  const key = process.env.BILLING_KEY;
  return <span data-site={process.env.NEXT_PUBLIC_SITE_NAME}>{key}</span>;
}
