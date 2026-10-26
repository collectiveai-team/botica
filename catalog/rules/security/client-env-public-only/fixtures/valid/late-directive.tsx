import { report } from "./report";

"use client";

export function Report() {
  return <pre>{report(process.env.REPORT_TOKEN)}</pre>;
}
