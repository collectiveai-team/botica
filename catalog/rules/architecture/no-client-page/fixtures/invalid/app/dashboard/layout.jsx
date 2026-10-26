// The frame of the dashboard.
'use client'

import { useState } from "react";

export default function Layout({ children }) {
  const [open, setOpen] = useState(true);
  return (
    <div>
      <button onClick={() => setOpen(!open)}>Menu</button>
      {open && <nav />}
      {children}
    </div>
  );
}
