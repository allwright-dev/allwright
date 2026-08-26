import { chainSelectorForTransport } from "./selectors.js";
import type {
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

export class LocatorImpl implements Locator {
  readonly page: Page;
  readonly selector: string;

  constructor(input: LocatorInfo) {
    this.page = input.page;
    this.selector = input.selector;
  }

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

  async textContent(options: CommandOptions = {}): Promise<TextResult> {
    return this.page.textContent(this.selector, options);
  }

  async innerText(options: CommandOptions = {}): Promise<TextResult> {
    return this.page.innerText(this.selector, options);
  }

  async waitFor(options: WaitForSelectorOptions = {}): Promise<WaitForSelectorResult> {
    return this.page.waitForSelector(this.selector, options);
  }

  locator(selector: string): Locator {
    return new LocatorImpl({
      page: this.page,
      selector: chainSelectorForTransport(this.selector, selector),
    });
  }
}
