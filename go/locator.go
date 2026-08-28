package allwright

import (
	"context"
	"fmt"
)

func (t *Tab) Locator(selector string) *Locator {
	if t == nil {
		return nil
	}
	return &Locator{
		page:     t,
		selector: normalizeSelectorForTransport(selector),
	}
}

func (l *Locator) Page() *Page {
	if l == nil {
		return nil
	}
	return l.page
}

func (l *Locator) Selector() string {
	if l == nil {
		return ""
	}
	return l.selector
}

func (l *Locator) Locator(selector string) *Locator {
	if l == nil {
		return nil
	}
	return &Locator{
		page:     l.page,
		selector: chainSelectorForTransport(l.selector, selector),
	}
}

func (l *Locator) Click(ctx context.Context, options ...CommandOptions) (*ClickResult, error) {
	if l == nil || l.page == nil {
		return nil, fmt.Errorf("locator page is nil")
	}
	return l.page.Click(ctx, l.selector, options...)
}

func (l *Locator) Count(ctx context.Context, options ...CommandOptions) (*CountResult, error) {
	if l == nil || l.page == nil {
		return nil, fmt.Errorf("locator page is nil")
	}
	return l.page.Count(ctx, l.selector, options...)
}

func (l *Locator) Highlight(ctx context.Context, options ...HighlightOptions) (*HighlightResult, error) {
	if l == nil || l.page == nil {
		return nil, fmt.Errorf("locator page is nil")
	}
	return l.page.Highlight(ctx, l.selector, options...)
}

func (l *Locator) Focus(ctx context.Context, options ...CommandOptions) (*ElementResult, error) {
	if l == nil || l.page == nil {
		return nil, fmt.Errorf("locator page is nil")
	}
	return l.page.Focus(ctx, l.selector, options...)
}

func (l *Locator) Fill(ctx context.Context, value string, options ...CommandOptions) (*FillResult, error) {
	if l == nil || l.page == nil {
		return nil, fmt.Errorf("locator page is nil")
	}
	return l.page.Fill(ctx, l.selector, value, options...)
}

func (l *Locator) Hover(ctx context.Context, options ...CommandOptions) (*ElementResult, error) {
	if l == nil || l.page == nil {
		return nil, fmt.Errorf("locator page is nil")
	}
	return l.page.Hover(ctx, l.selector, options...)
}

func (l *Locator) Press(ctx context.Context, key string, options ...PressOptions) (*PressResult, error) {
	if l == nil || l.page == nil {
		return nil, fmt.Errorf("locator page is nil")
	}
	return l.page.Press(ctx, l.selector, key, options...)
}

func (l *Locator) TextContent(ctx context.Context, options ...CommandOptions) (*TextResult, error) {
	if l == nil || l.page == nil {
		return nil, fmt.Errorf("locator page is nil")
	}
	return l.page.TextContent(ctx, l.selector, options...)
}

func (l *Locator) InnerText(ctx context.Context, options ...CommandOptions) (*TextResult, error) {
	if l == nil || l.page == nil {
		return nil, fmt.Errorf("locator page is nil")
	}
	return l.page.InnerText(ctx, l.selector, options...)
}

func (l *Locator) WaitFor(ctx context.Context, options ...WaitForSelectorOptions) (*WaitForSelectorResult, error) {
	if l == nil || l.page == nil {
		return nil, fmt.Errorf("locator page is nil")
	}
	return l.page.WaitForSelector(ctx, l.selector, options...)
}
