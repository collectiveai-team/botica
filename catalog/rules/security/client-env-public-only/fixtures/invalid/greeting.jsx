"use client";

export default function Greeting() {
  return <p>{process.env.GREETING_TEXT}</p>;
}
