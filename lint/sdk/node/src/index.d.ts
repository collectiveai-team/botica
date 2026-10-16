export const PROTOCOL_VERSION: number;

/** One finding: `start` and `end` are byte offsets in the UTF-8 text of the file. */
export interface Finding {
  start: number;
  end: number;
  message: string;
}

export type Check = (path: string, text: string) => Iterable<Finding>;

export interface RuleOptions {
  /** `"error"`, `"warning"` or `"info"`; `"warning"` when omitted. */
  severity?: string;
}

export function rule(
  id: string,
  languages: string[],
  check: Check,
  options?: RuleOptions,
): Check;

/** Converts a string index (UTF-16 code units) into a byte offset of the UTF-8 text. */
export function byteOffsets(text: string): (index: number) => number;

/** Answers `initialize`, `lint/files` and `shutdown` on stdin and stdout, and returns after `shutdown`. */
export function serve(): Promise<void>;
