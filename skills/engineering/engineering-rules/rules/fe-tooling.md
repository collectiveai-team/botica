---
title: TypeScript strict everywhere; use the repo's package manager; never ship with build errors suppressed
section: fe
scope: specific
applies-to: nextjs
status: current
tags: nextjs, typescript, tooling, tsconfig, eslint, npm, pnpm
---

## TypeScript strict everywhere; use the repo's package manager; never ship with build errors suppressed

All Next.js frontends must run `"strict": true` in `tsconfig.json`. Match the package
manager lockfile already committed in the repo — do not mix npm and pnpm. Forms use
`react-hook-form` + `@hookform/resolvers` + `zod`. Never suppress ESLint or TypeScript
errors in `next.config.mjs`; this is a scaffolding shortcut that must be removed before
the app ships.

**Prefer:**

```ts
// tsconfig.json — strict is non-negotiable
{ "compilerOptions": { "strict": true, "target": "ES2022" } }

// next.config.mjs — keep checks enabled
const nextConfig = {
  experimental: { serverActions: { allowedOrigins: ["localhost:3000"] } },
  // no eslint.ignoreDuringBuilds, no typescript.ignoreBuildErrors
}
export default nextConfig

// Forms: react-hook-form + zod + zodResolver
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
const form = useForm({ resolver: zodResolver(MySchema) })
```

**Avoid:**

```js
// next.config.mjs — scaffolding default; never commit this to a production branch
const nextConfig = {
  eslint: { ignoreDuringBuilds: true },       // hides real lint errors
  typescript: { ignoreBuildErrors: true },    // hides real type errors
}
```

```bash
# Wrong package manager — follow the repo's lockfile
npm install    # in a repo that uses pnpm-lock.yaml
pnpm install   # in a repo that uses package-lock.json
```

Reference: `fe-shadcn-tailwind` for `components/ui/form.tsx` usage.
