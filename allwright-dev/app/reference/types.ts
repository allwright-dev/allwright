// Shared shape for the hand-authored API reference. One LanguageReference per
// client (rust/go/java/python/typescript), each broken into the same set of
// modules so the module list and its order stay identical across languages —
// that's what makes the language tab switcher able to jump to "the same
// place" when you flip from one language to another.
//
// Content here is maintained by hand alongside the client libraries
// themselves (see allwright-dev/CLAUDE.md) rather than generated from doc
// comments — none of the five codebases carry doc comments dense enough to
// extract a reference from yet. Each member's `since` field is the release
// documented in app/changelog/changelog-data.ts where that capability (or
// its current shape) first shipped, so the badge stays consistent with the
// changelog rather than inventing a second version history.

export type ApiKind = "function" | "method" | "type" | "class" | "property";

export type ApiMember = {
  /** Display name, e.g. "click" or "ClickResult". */
  name: string;
  kind: ApiKind;
  /** Exact source-accurate signature, rendered as a highlighted code line (or a short multi-line block for a type shape). */
  signature: string;
  /** One or two plain-English sentences. */
  description: string;
  /** Version tag matching a changelog entry, e.g. "v0.1.5". */
  since: string;
  /** A short, realistic call site — omitted when the signature alone is already the clearest example (e.g. plain data types). */
  example?: string;
};

export type ApiModule = {
  /** Stable across all five languages — used for the in-page anchor and to keep the sidebar order identical when switching language tabs. */
  slug: "browser" | "page" | "locator" | "hooks" | "mobile" | "config" | "vitest";
  title: string;
  description: string;
  members: ApiMember[];
};

export type LanguageId = "typescript" | "python" | "java" | "rust" | "go";

export type LanguageReference = {
  id: LanguageId;
  label: string;
  /** shiki language id used to highlight every signature/example in this language's modules. */
  codeLang: string;
  packageName: string;
  installCommand: string;
  /** GitHub path to this client's source, for "view source" links. */
  sourceHref: string;
  modules: ApiModule[];
};
