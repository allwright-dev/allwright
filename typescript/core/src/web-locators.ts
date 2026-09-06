import type { Locator } from './types.js';

export type TextMatcher = string | RegExp;
export interface TextOptions { exact?: boolean }
export interface RoleOptions extends TextOptions {
  name?: TextMatcher;
  checked?: boolean;
  disabled?: boolean;
  expanded?: boolean;
  includeHidden?: boolean;
  level?: number;
  pressed?: boolean;
  selected?: boolean;
}
export interface LocatorFilterOptions {
  has?: Locator;
  hasNot?: Locator;
  hasText?: TextMatcher;
  hasNotText?: TextMatcher;
  visible?: boolean;
}
export interface WebLocators {
  getByRole(role: string, options?: RoleOptions): Locator;
  getByText(text: TextMatcher, options?: TextOptions): Locator;
  getByLabel(text: TextMatcher, options?: TextOptions): Locator;
  getByPlaceholder(text: TextMatcher, options?: TextOptions): Locator;
  getByAltText(text: TextMatcher, options?: TextOptions): Locator;
  getByTitle(text: TextMatcher, options?: TextOptions): Locator;
  getByTestId(text: TextMatcher): Locator;
}
export function semanticSelector(spec: Record<string, unknown>): string {
  return `aw=${JSON.stringify(JSON.stringify(spec, (_key, value) =>
    value instanceof RegExp ? { regex: value.source, flags: value.flags } : value))}`;
}
/** Locator construction only; DOM matching belongs to the web plugin. */
export abstract class WebLocatorBuilders implements WebLocators {
  abstract locator(selector: string, options?: LocatorFilterOptions): Locator;
  getByRole(role: string, options: RoleOptions = {}): Locator {
    if (options.level !== undefined && (!Number.isInteger(options.level) || options.level < 1)) {
      throw new Error('Role level must be a positive integer');
    }
    return this.locator(semanticSelector({ ...options, kind: 'role', role }));
  }
  getByText(text: TextMatcher, options: TextOptions = {}): Locator { return this.locator(semanticSelector({ ...options, kind: 'text', text })); }
  getByLabel(text: TextMatcher, options: TextOptions = {}): Locator { return this.locator(semanticSelector({ ...options, kind: 'label', text })); }
  getByPlaceholder(text: TextMatcher, options: TextOptions = {}): Locator { return this.locator(semanticSelector({ ...options, kind: 'placeholder', text })); }
  getByAltText(text: TextMatcher, options: TextOptions = {}): Locator { return this.locator(semanticSelector({ ...options, kind: 'altText', text })); }
  getByTitle(text: TextMatcher, options: TextOptions = {}): Locator { return this.locator(semanticSelector({ ...options, kind: 'title', text })); }
  getByTestId(text: TextMatcher): Locator { return this.locator(semanticSelector({ kind: 'testId', text })); }
}
