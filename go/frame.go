package allwright

import (
	enginev1 "allwright.dev/gen/allwright/engine/v1"
	"context"
	"fmt"
)

// Frame resolves this iframe to a page after its document loads and settles.
func (l *Locator) Frame(ctx context.Context, options ...CommandOptions) (*Page, error) {
	if l == nil || l.page == nil {
		return nil, fmt.Errorf("locator page is nil")
	}
	t := l.page
	t.mu.Lock()
	defer t.mu.Unlock()
	if err := t.ensureStream(ctx); err != nil {
		return nil, err
	}
	if t.closed {
		return nil, fmt.Errorf("page is closed")
	}
	if err := t.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: t.browserSessionID, ContextSessionId: t.sessionID,
		Command: &enginev1.ContextSessionCommand_ResolveFrame{ResolveFrame: &enginev1.ResolveFrameCommand{
			CssSelector: l.selector, RetryOptions: retryOptionsProto(firstCommandOptions(options).Timeout),
		}},
	}); err != nil {
		return nil, err
	}
	for {
		event, err := t.stream.Recv()
		if err != nil {
			return nil, err
		}
		switch payload := event.Event.(type) {
		case *enginev1.ContextSessionEvent_FrameResolved:
			return &Tab{runtime: t.runtime, browserSessionID: t.browserSessionID, sessionID: payload.FrameResolved.GetContextSessionId()}, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("resolve frame: %s", payload.Error.GetMessage())
		case *enginev1.ContextSessionEvent_Closed:
			t.closed = true
			return nil, fmt.Errorf("page closed while resolving frame")
		}
	}
}
