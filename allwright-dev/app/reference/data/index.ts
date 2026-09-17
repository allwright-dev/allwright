import type { LanguageId, LanguageReference } from "../types";
import { goReference } from "./go";
import { javaReference } from "./java";
import { pythonReference } from "./python";
import { rustReference } from "./rust";
import { typescriptReference } from "./typescript";

// Order here is the order every language picker on the reference pages
// renders in — TypeScript first since it's the flagship client used
// everywhere else on this site (quickstart, most blog posts).
export const references: LanguageReference[] = [
  typescriptReference,
  pythonReference,
  javaReference,
  rustReference,
  goReference,
];

export function getReference(id: LanguageId): LanguageReference | undefined {
  return references.find((reference) => reference.id === id);
}

export function getLanguageIds(): LanguageId[] {
  return references.map((reference) => reference.id);
}
