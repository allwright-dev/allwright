package allwright

import (
	"context"
	"fmt"

	enginev1 "allwright.dev/gen/allwright/engine/v1"
)

func (t *Tab) SessionID() string {
	if t == nil {
		return ""
	}
	return t.sessionID
}

func (t *Tab) Goto(ctx context.Context, url string, options ...CommandOptions) (*NavigateResult, error) {
	if t == nil {
		return nil, fmt.Errorf("tab is nil")
	}
	return t.Navigate(ctx, url, options...)
}

func (t *Tab) Navigate(ctx context.Context, url string, options ...CommandOptions) (*NavigateResult, error) {
	if t == nil {
		return nil, fmt.Errorf("tab is nil")
	}
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return nil, err
	}
	if t.closed {
		return nil, fmt.Errorf("tab session %s is closed", t.sessionID)
	}

	commandOptions := firstCommandOptions(options)

	if err := t.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: t.browserSessionID,
		ContextSessionId: t.sessionID,
		Command: &enginev1.ContextSessionCommand_Navigate{
			Navigate: &enginev1.NavigatePageCommand{
				Url:          url,
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send NavigatePageCommand: %w", err)
	}

	result := &NavigateResult{}
	for {
		event, err := t.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive tab session event during navigate: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_Navigated:
			result.URL = payload.Navigated.GetUrl()
			result.Note = payload.Navigated.GetNote()
		case *enginev1.ContextSessionEvent_ChromiumBidiInjection:
			result.BidiSessionID = payload.ChromiumBidiInjection.GetBidiSessionId()
			result.MapperTargetID = payload.ChromiumBidiInjection.GetMapperTargetId()
			result.MapperSessionID = payload.ChromiumBidiInjection.GetMapperSessionId()
			result.PackageVersion = payload.ChromiumBidiInjection.GetPackageVersion()
			t.lastBidiSessionID = result.BidiSessionID
			return result, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while navigating: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) Click(ctx context.Context, cssSelector string, options ...CommandOptions) (*ClickResult, error) {
	if t == nil {
		return nil, fmt.Errorf("tab is nil")
	}
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return nil, err
	}
	if t.closed {
		return nil, fmt.Errorf("tab session %s is closed", t.sessionID)
	}

	commandOptions := firstCommandOptions(options)
	cssSelector = normalizeSelectorForTransport(cssSelector)

	if err := t.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: t.browserSessionID,
		ContextSessionId: t.sessionID,
		Command: &enginev1.ContextSessionCommand_ClickElement{
			ClickElement: &enginev1.ClickElementCommand{
				CssSelector:  cssSelector,
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send ClickElementCommand: %w", err)
	}

	for {
		event, err := t.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive tab session event during click: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_ElementClicked:
			t.lastBidiSessionID = payload.ElementClicked.GetBidiSessionId()
			return &ClickResult{
				Selector:      payload.ElementClicked.GetCssSelector(),
				Note:          payload.ElementClicked.GetNote(),
				BidiSessionID: payload.ElementClicked.GetBidiSessionId(),
			}, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while clicking: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) Count(ctx context.Context, cssSelector string, options ...CommandOptions) (*CountResult, error) {
	if t == nil {
		return nil, fmt.Errorf("tab is nil")
	}
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return nil, err
	}
	if t.closed {
		return nil, fmt.Errorf("tab session %s is closed", t.sessionID)
	}

	commandOptions := firstCommandOptions(options)
	cssSelector = normalizeSelectorForTransport(cssSelector)

	if err := t.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: t.browserSessionID,
		ContextSessionId: t.sessionID,
		Command: &enginev1.ContextSessionCommand_CountElements{
			CountElements: &enginev1.CountElementsCommand{
				CssSelector:  cssSelector,
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send CountElementsCommand: %w", err)
	}

	for {
		event, err := t.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive tab session event during count: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_ElementCounted:
			return &CountResult{
				Selector: payload.ElementCounted.GetCssSelector(),
				Count:    payload.ElementCounted.GetCount(),
				Note:     payload.ElementCounted.GetNote(),
			}, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while counting elements: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) Highlight(ctx context.Context, cssSelector string, options ...HighlightOptions) (*HighlightResult, error) {
	if t == nil {
		return nil, fmt.Errorf("tab is nil")
	}
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return nil, err
	}
	if t.closed {
		return nil, fmt.Errorf("tab session %s is closed", t.sessionID)
	}

	highlightOptions := firstHighlightOptions(options)
	cssSelector = normalizeSelectorForTransport(cssSelector)

	if err := t.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: t.browserSessionID,
		ContextSessionId: t.sessionID,
		Command: &enginev1.ContextSessionCommand_HighlightElements{
			HighlightElements: &enginev1.HighlightElementsCommand{
				CssSelector:  cssSelector,
				DurationMs:   durationProto(highlightOptions.Duration),
				RetryOptions: retryOptionsProto(highlightOptions.Timeout),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send HighlightElementsCommand: %w", err)
	}

	for {
		event, err := t.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive tab session event during highlight: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_ElementsHighlighted:
			return &HighlightResult{
				Selector: payload.ElementsHighlighted.GetCssSelector(),
				Count:    payload.ElementsHighlighted.GetCount(),
				Note:     payload.ElementsHighlighted.GetNote(),
			}, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while highlighting elements: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) Focus(ctx context.Context, cssSelector string, options ...CommandOptions) (*ElementResult, error) {
	if t == nil {
		return nil, fmt.Errorf("tab is nil")
	}
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return nil, err
	}
	if t.closed {
		return nil, fmt.Errorf("tab session %s is closed", t.sessionID)
	}

	commandOptions := firstCommandOptions(options)
	cssSelector = normalizeSelectorForTransport(cssSelector)

	if err := t.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: t.browserSessionID,
		ContextSessionId: t.sessionID,
		Command: &enginev1.ContextSessionCommand_FocusElement{
			FocusElement: &enginev1.FocusElementCommand{
				CssSelector:  cssSelector,
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send FocusElementCommand: %w", err)
	}

	for {
		event, err := t.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive tab session event during focus: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_ElementFocused:
			return &ElementResult{
				Selector: payload.ElementFocused.GetCssSelector(),
				Note:     payload.ElementFocused.GetNote(),
			}, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while focusing: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) Fill(ctx context.Context, cssSelector string, value string, options ...CommandOptions) (*FillResult, error) {
	if t == nil {
		return nil, fmt.Errorf("tab is nil")
	}
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return nil, err
	}
	if t.closed {
		return nil, fmt.Errorf("tab session %s is closed", t.sessionID)
	}

	commandOptions := firstCommandOptions(options)
	cssSelector = normalizeSelectorForTransport(cssSelector)

	if err := t.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: t.browserSessionID,
		ContextSessionId: t.sessionID,
		Command: &enginev1.ContextSessionCommand_FillElement{
			FillElement: &enginev1.FillElementCommand{
				CssSelector:  cssSelector,
				Value:        value,
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send FillElementCommand: %w", err)
	}

	for {
		event, err := t.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive tab session event during fill: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_ElementFilled:
			return &FillResult{
				Selector: payload.ElementFilled.GetCssSelector(),
				Value:    payload.ElementFilled.GetValue(),
				Note:     payload.ElementFilled.GetNote(),
			}, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while filling: %s", payload.Error.GetMessage())
		}
	}
}
