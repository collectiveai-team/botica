---
title: Style with shadcn/ui + Tailwind; merge classes with cn()
section: fe
scope: specific
applies-to: nextjs
status: current
tags: nextjs, shadcn, tailwind, styling, cn, lucide
---

## Style with shadcn/ui + Tailwind; merge classes with cn()

Use shadcn/ui (full Radix primitive set), Tailwind CSS, and the `cn()` helper
(`clsx` + `tailwind-merge`). Customize components by editing the component file
directly — never wrap them with style overrides. Use `cn()` everywhere you need conditional
or merged class strings.

**Prefer:**

```tsx
import { cn } from "@/lib/utils"

// Merge base + variant + caller-supplied classes
function StatusBadge({ active, className }: { active: boolean; className?: string }) {
  return (
    <span className={cn(
      "rounded-full px-2 py-0.5 text-xs font-medium",
      active ? "bg-green-100 text-green-800" : "bg-gray-100 text-gray-600",
      className,
    )}>
      {active ? "Active" : "Inactive"}
    </span>
  )
}
```

**Avoid:**

```tsx
// Don't override shadcn components with wrappers — edit the source component instead
function MyButton(props: ButtonProps) {
  return <Button {...props} style={{ borderRadius: 0 }} /> // bypasses Tailwind merge
}

// Don't concatenate raw strings — twMerge won't deduplicate conflicting utilities
className={"px-4 " + (large ? "px-8" : "")}
```

Reference: shadcn/ui docs (ui.shadcn.com); `fe-app-router` for component co-location.
