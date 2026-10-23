import type { Plugin } from "./plugins";
import { registry } from "./plugins";

export function plugin(name: string): Plugin {
  return registry[name] as unknown as Plugin; // botica: ignore[no-type-cast] -- the types of the registry say `object`
}
