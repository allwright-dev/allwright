package allwright

import (
	"context"
	"fmt"

	enginev1 "allwright.dev/gen/allwright/engine/v1"
)

func (p *NativeApp) capture(ctx context.Context, kind enginev1.CaptureKind, selector, attribute string, options CommandOptions) (*enginev1.CaptureResolvedEvent, error) {
	if p == nil {
		return nil, fmt.Errorf("native app is nil")
	}
	if err := p.ensureStream(ctx); err != nil {
		return nil, err
	}
	if p.closed {
		return nil, fmt.Errorf("native app session is closed")
	}
	if err := p.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: p.surfaceSessionID,
		ContextSessionId: p.sessionID,
		Command: &enginev1.ContextSessionCommand_Capture{Capture: &enginev1.CaptureCommand{
			Kind: kind, CssSelector: normalizeMobileSelectorForTransport(selector), AttributeName: attribute,
			RetryOptions: retryOptionsProto(options.Timeout),
		}},
	}); err != nil {
		return nil, err
	}
	for {
		event, err := p.stream.Recv()
		if err != nil {
			return nil, err
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			p.attached = true
		case *enginev1.ContextSessionEvent_CaptureResolved:
			return payload.CaptureResolved, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("native capture: %s", payload.Error.GetMessage())
		case *enginev1.ContextSessionEvent_Closed:
			p.closed = true
			return nil, fmt.Errorf("native app session closed while capturing")
		}
	}
}

func (p *NativeApp) InputValue(ctx context.Context, selector string, options ...CommandOptions) (string, error) {
	r, err := p.capture(ctx, enginev1.CaptureKind_CAPTURE_KIND_INPUT_VALUE, selector, "", firstCommandOptions(options))
	if err != nil { return "", err }
	return r.GetValue(), nil
}

func (p *NativeApp) IsChecked(ctx context.Context, selector string, options ...CommandOptions) (bool, error) {
	r, err := p.capture(ctx, enginev1.CaptureKind_CAPTURE_KIND_CHECKED, selector, "", firstCommandOptions(options))
	if err != nil { return false, err }
	return r.GetChecked(), nil
}

func (p *NativeApp) GetAttribute(ctx context.Context, selector, name string, options ...CommandOptions) (*string, error) {
	r, err := p.capture(ctx, enginev1.CaptureKind_CAPTURE_KIND_ATTRIBUTE, selector, name, firstCommandOptions(options))
	if err != nil { return nil, err }
	return r.Value, nil
}

func (p *NativeApp) BoundingBox(ctx context.Context, selector string, options ...CommandOptions) (*BoundingBox, error) {
	r, err := p.capture(ctx, enginev1.CaptureKind_CAPTURE_KIND_BOUNDING_BOX, selector, "", firstCommandOptions(options))
	if err != nil || r.BoundingBox == nil { return nil, err }
	b := r.BoundingBox
	return &BoundingBox{X: b.X, Y: b.Y, Width: b.Width, Height: b.Height}, nil
}

func (l *NativeLocator) InputValue(ctx context.Context, options ...CommandOptions) (string, error) {
	if l == nil || l.page == nil { return "", fmt.Errorf("native locator page is nil") }
	return l.page.InputValue(ctx, l.selector, options...)
}

func (l *NativeLocator) IsChecked(ctx context.Context, options ...CommandOptions) (bool, error) {
	if l == nil || l.page == nil { return false, fmt.Errorf("native locator page is nil") }
	return l.page.IsChecked(ctx, l.selector, options...)
}

func (l *NativeLocator) GetAttribute(ctx context.Context, name string, options ...CommandOptions) (*string, error) {
	if l == nil || l.page == nil { return nil, fmt.Errorf("native locator page is nil") }
	return l.page.GetAttribute(ctx, l.selector, name, options...)
}

func (l *NativeLocator) BoundingBox(ctx context.Context, options ...CommandOptions) (*BoundingBox, error) {
	if l == nil || l.page == nil { return nil, fmt.Errorf("native locator page is nil") }
	return l.page.BoundingBox(ctx, l.selector, options...)
}
