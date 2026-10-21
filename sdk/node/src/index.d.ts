export const PROTOCOL_VERSION: number;

/** One finding: `start` and `end` are byte offsets in the UTF-8 text of the file. */
export interface Finding {
  start: number;
  end: number;
  message: string;
}

/** A finding of a rule of project scope, in the file at `path`. */
export interface ProjectFinding extends Finding {
  path: string;
}

/** The options that the configuration of the project gives a rule, by name. */
export type Options = Record<string, unknown>;

export type Check = (path: string, text: string, options: Options) => Iterable<Finding>;

export type ProjectCheck = (root: string, paths: string[], options: Options) => Iterable<ProjectFinding>;

export interface RuleOptions {
  /** `"error"`, `"warning"` or `"info"`; `"warning"` when omitted. */
  severity?: string;
  /** `"project"` for a rule that gets the root and the paths of the project; `"file"` when omitted. */
  scope?: "file" | "project";
}

export function rule(
  id: string,
  languages: string[],
  check: Check | ProjectCheck,
  options?: RuleOptions,
): Check | ProjectCheck;

/** Converts a string index (UTF-16 code units) into a byte offset of the UTF-8 text. */
export function byteOffsets(text: string): (index: number) => number;

/**
 * Answers `initialize`, `lint/files`, `lint/project` and `shutdown` on stdin and stdout, and
 * returns after `shutdown`.
 */
export function serve(): Promise<void>;
