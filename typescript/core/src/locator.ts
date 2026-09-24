import { WebLocatorBuilders, semanticSelector, type LocatorFilterOptions } from "./web-locators.js";
import { chainSelectorForTransport } from "./selectors.js";
import type {
  BoundingBox, CapturedOption, CaptureResult,
  ClickResult,
  CommandOptions,
  CountResult,
  ElementResult,
  FillResult,
  HighlightOptions,
  HighlightResult,
  Locator,
  LocatorInfo,
  Page,
  PressOptions,
  PressResult,
  TextResult,
  WaitForSelectorOptions,
  WaitForSelectorResult,
} from "./types.js";

export class LocatorImpl extends WebLocatorBuilders implements Locator {
  readonly page: Page;
  readonly selector: string;

  constructor(input: LocatorInfo) {
    super();
    this.page = input.page;
    this.selector = input.selector;
  }

  frame(options: CommandOptions = {}): Promise<Page> { return this.page.frame(this.selector, options); }
  Frame(options: CommandOptions = {}): Promise<Page> { return this.frame(options); }

  async click(options: CommandOptions = {}): Promise<ClickResult> {
    return this.page.click(this.selector, options);
  }

  async count(options: CommandOptions = {}): Promise<CountResult> {
    return this.page.count(this.selector, options);
  }

  async highlight(options: HighlightOptions = {}): Promise<HighlightResult> {
    return this.page.highlight(this.selector, options);
  }

  async focus(options: CommandOptions = {}): Promise<ElementResult> {
    return this.page.focus(this.selector, options);
  }

  async fill(value: string, options: CommandOptions = {}): Promise<FillResult> {
    return this.page.fill(this.selector, value, options);
  }

  async hover(options: CommandOptions = {}): Promise<ElementResult> {
    return this.page.hover(this.selector, options);
  }

  async press(key: string, options: PressOptions = {}): Promise<PressResult> {
    return this.page.press(this.selector, key, options);
  }

  async inputValue(options: CommandOptions = {}): Promise<string> {
    return this.page.inputValue(this.selector, options);
  }
  async selectedOptions(options: CommandOptions = {}): Promise<CapturedOption[]> {
    return this.page.selectedOptions(this.selector, options);
  }
  async selectedText(options: CommandOptions = {}): Promise<string | null> {
    return this.page.selectedText(this.selector, options);
  }
  async isChecked(options: CommandOptions = {}): Promise<boolean> {
    return this.page.isChecked(this.selector, options);
  }
  async getAttribute(name: string, options: CommandOptions = {}): Promise<string | null> {
    return this.page.getAttribute(this.selector, name, options);
  }
  async boundingBox(options: CommandOptions = {}): Promise<BoundingBox | null> {
    return this.page.boundingBox(this.selector, options);
  }

  async textContent(options: CommandOptions = {}): Promise<TextResult> {
    return this.page.textContent(this.selector, options);
  }

  async innerText(options: CommandOptions = {}): Promise<TextResult> {
    return this.page.innerText(this.selector, options);
  }

  async waitFor(options: WaitForSelectorOptions = {}): Promise<WaitForSelectorResult> {
    return this.page.waitForSelector(this.selector, options);
  }

  not(other: Locator): Locator {
    if (other.page !== this.page) throw new Error('Excluded locators must belong to the same page');
    return this.locator(semanticSelector({ kind: 'exclude', selector: other.selector }));
  }

  filter(options: LocatorFilterOptions = {}): Locator {
    for (const inner of [options.has, options.hasNot]) {
      if (inner && inner.page !== this.page) throw new Error('Filter locators must belong to the same page');
    }
    return this.locator(semanticSelector({ ...options, kind: 'filter', has: options.has?.selector, hasNot: options.hasNot?.selector }));
  }

  nth(index: number): Locator {
    if (!Number.isInteger(index)) throw new Error('Locator index must be an integer');
    return this.locator(semanticSelector({ kind: 'nth', index }));
  }
  first(): Locator { return this.nth(0); }
  last(): Locator { return this.nth(-1); }

  locator(selector: string, options?: LocatorFilterOptions): Locator {
    const result = new LocatorImpl({
      page: this.page,
      selector: chainSelectorForTransport(this.selector, selector),
    });
    return options ? result.filter(options) : result;
  }
}
