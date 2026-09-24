package allwright

import (
	"context"
	"fmt"

	enginev1 "allwright.dev/gen/allwright/engine/v1"
)

type CapturedOption struct {
	Value string
	Label string
	Index uint32
}
type BoundingBox struct {
	X      float64
	Y      float64
	Width  float64
	Height float64
}

func (t *Tab) capture(ctx context.Context, kind enginev1.CaptureKind, selector, attribute string, options CommandOptions) (*enginev1.CaptureResolvedEvent, error) {
	if t == nil {
		return nil, fmt.Errorf("tab is nil")
	}
	t.mu.Lock()
	defer t.mu.Unlock()
	if err := t.ensureStream(ctx); err != nil {
		return nil, err
	}
	if t.closed {
		return nil, fmt.Errorf("tab session is closed")
	}
	if selector != "" {
		selector = normalizeSelectorForTransport(selector)
	}
	err := t.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: t.browserSessionID, ContextSessionId: t.sessionID,
		Command: &enginev1.ContextSessionCommand_Capture{Capture: &enginev1.CaptureCommand{
			Kind: kind, CssSelector: selector, AttributeName: attribute, RetryOptions: retryOptionsProto(options.Timeout),
		}},
	})
	if err != nil {
		return nil, err
	}
	for {
		event, err := t.stream.Recv()
		if err != nil {
			return nil, err
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			t.attached = true
		case *enginev1.ContextSessionEvent_CaptureResolved:
			return payload.CaptureResolved, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("capture: %s", payload.Error.GetMessage())
		case *enginev1.ContextSessionEvent_Closed:
			t.closed = true
			return nil, fmt.Errorf("tab session closed while capturing")
		}
	}
}

func (t *Tab) URL(ctx context.Context, options ...CommandOptions) (string, error) {
	r, err := t.capture(ctx, enginev1.CaptureKind_CAPTURE_KIND_URL, "", "", firstCommandOptions(options))
	if err != nil {
		return "", err
	}
	return r.GetValue(), nil
}

func (t *Tab) InputValue(ctx context.Context, selector string, options ...CommandOptions) (string, error) {
	r, err := t.capture(ctx, enginev1.CaptureKind_CAPTURE_KIND_INPUT_VALUE, selector, "", firstCommandOptions(options))
	if err != nil {
		return "", err
	}
	return r.GetValue(), nil
}

func (l *Locator) InputValue(ctx context.Context, options ...CommandOptions) (string, error) {
	if l == nil {
		return "", fmt.Errorf("locator is nil")
	}
	return l.page.InputValue(ctx, l.selector, options...)
}

func (t *Tab) SelectedOptions(ctx context.Context, selector string, options ...CommandOptions) ([]CapturedOption, error) {
	r, err := t.capture(ctx, enginev1.CaptureKind_CAPTURE_KIND_SELECTED_OPTIONS, selector, "", firstCommandOptions(options))
	if err != nil {
		return nil, err
	}
	result := make([]CapturedOption, 0, len(r.SelectedOptions))
	for _, o := range r.SelectedOptions {
		result = append(result, CapturedOption{Value: o.Value, Label: o.Label, Index: o.Index})
	}
	return result, nil
}

func (l *Locator) SelectedOptions(ctx context.Context, options ...CommandOptions) ([]CapturedOption, error) {
	if l == nil {
		return nil, fmt.Errorf("locator is nil")
	}
	return l.page.SelectedOptions(ctx, l.selector, options...)
}

func (t *Tab) SelectedText(ctx context.Context, selector string, options ...CommandOptions) (*string, error) {
	r, err := t.capture(ctx, enginev1.CaptureKind_CAPTURE_KIND_SELECTED_TEXT, selector, "", firstCommandOptions(options))
	if err != nil {
		return nil, err
	}
	return r.Value, nil
}

func (l *Locator) SelectedText(ctx context.Context, options ...CommandOptions) (*string, error) {
	if l == nil {
		return nil, fmt.Errorf("locator is nil")
	}
	return l.page.SelectedText(ctx, l.selector, options...)
}

func (t *Tab) IsChecked(ctx context.Context, selector string, options ...CommandOptions) (bool, error) {
	r, err := t.capture(ctx, enginev1.CaptureKind_CAPTURE_KIND_CHECKED, selector, "", firstCommandOptions(options))
	if err != nil {
		return false, err
	}
	return r.GetChecked(), nil
}

func (l *Locator) IsChecked(ctx context.Context, options ...CommandOptions) (bool, error) {
	if l == nil {
		return false, fmt.Errorf("locator is nil")
	}
	return l.page.IsChecked(ctx, l.selector, options...)
}

func (t *Tab) GetAttribute(ctx context.Context, selector string, name string, options ...CommandOptions) (*string, error) {
	r, err := t.capture(ctx, enginev1.CaptureKind_CAPTURE_KIND_ATTRIBUTE, selector, name, firstCommandOptions(options))
	if err != nil {
		return nil, err
	}
	return r.Value, nil
}

func (l *Locator) GetAttribute(ctx context.Context, name string, options ...CommandOptions) (*string, error) {
	if l == nil {
		return nil, fmt.Errorf("locator is nil")
	}
	return l.page.GetAttribute(ctx, l.selector, name, options...)
}

func (t *Tab) BoundingBox(ctx context.Context, selector string, options ...CommandOptions) (*BoundingBox, error) {
	r, err := t.capture(ctx, enginev1.CaptureKind_CAPTURE_KIND_BOUNDING_BOX, selector, "", firstCommandOptions(options))
	if err != nil {
		return nil, err
	}
	if r.BoundingBox == nil {
		return nil, nil
	}
	b := r.BoundingBox
	return &BoundingBox{X: b.X, Y: b.Y, Width: b.Width, Height: b.Height}, nil
}

func (l *Locator) BoundingBox(ctx context.Context, options ...CommandOptions) (*BoundingBox, error) {
	if l == nil {
		return nil, fmt.Errorf("locator is nil")
	}
	return l.page.BoundingBox(ctx, l.selector, options...)
}
