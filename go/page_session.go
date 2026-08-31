package allwright

import (
	"context"
	"fmt"

	enginev1 "allwright.dev/gen/allwright/engine/v1"
)

func (t *Tab) Hover(ctx context.Context, cssSelector string, options ...CommandOptions) (*ElementResult, error) {
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
		Command: &enginev1.ContextSessionCommand_HoverElement{
			HoverElement: &enginev1.HoverElementCommand{
				CssSelector:  cssSelector,
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send HoverElementCommand: %w", err)
	}

	for {
		event, err := t.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive tab session event during hover: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_ElementHovered:
			return &ElementResult{
				Selector: payload.ElementHovered.GetCssSelector(),
				Note:     payload.ElementHovered.GetNote(),
			}, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while hovering: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) Press(ctx context.Context, cssSelector string, key string, options ...PressOptions) (*PressResult, error) {
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

	pressOptions := firstPressOptions(options)
	cssSelector = normalizeSelectorForTransport(cssSelector)

	if err := t.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: t.browserSessionID,
		ContextSessionId: t.sessionID,
		Command: &enginev1.ContextSessionCommand_PressKey{
			PressKey: &enginev1.PressKeyCommand{
				CssSelector:  cssSelector,
				Key:          key,
				Text:         optionalString(pressOptions.Text),
				RetryOptions: retryOptionsProto(pressOptions.Timeout),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send PressKeyCommand: %w", err)
	}

	for {
		event, err := t.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive tab session event during press: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_KeyPressed:
			return &PressResult{
				Selector: payload.KeyPressed.GetCssSelector(),
				Key:      payload.KeyPressed.GetKey(),
				Note:     payload.KeyPressed.GetNote(),
			}, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while pressing key: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) TextContent(ctx context.Context, cssSelector string, options ...CommandOptions) (*TextResult, error) {
	if t == nil {
		return nil, fmt.Errorf("tab is nil")
	}
	return t.readText(ctx, cssSelector, true, firstCommandOptions(options))
}

func (t *Tab) InnerText(ctx context.Context, cssSelector string, options ...CommandOptions) (*TextResult, error) {
	if t == nil {
		return nil, fmt.Errorf("tab is nil")
	}
	return t.readText(ctx, cssSelector, false, firstCommandOptions(options))
}

func (t *Tab) WaitForSelector(ctx context.Context, cssSelector string, options ...WaitForSelectorOptions) (*WaitForSelectorResult, error) {
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

	waitOptions := firstWaitForSelectorOptions(options)
	cssSelector = normalizeSelectorForTransport(cssSelector)

	if err := t.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: t.browserSessionID,
		ContextSessionId: t.sessionID,
		Command: &enginev1.ContextSessionCommand_WaitForSelector{
			WaitForSelector: &enginev1.WaitForSelectorCommand{
				CssSelector:  cssSelector,
				Visible:      waitOptions.Visible,
				RetryOptions: retryOptionsProto(waitOptions.Timeout),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send WaitForSelectorCommand: %w", err)
	}

	for {
		event, err := t.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive tab session event during waitForSelector: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_SelectorWaitSatisfied:
			return &WaitForSelectorResult{
				Selector: payload.SelectorWaitSatisfied.GetCssSelector(),
				Visible:  payload.SelectorWaitSatisfied.GetVisible(),
				Note:     payload.SelectorWaitSatisfied.GetNote(),
			}, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while waiting for selector: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) Screenshot(ctx context.Context, options ...CommandOptions) (*ScreenshotResult, error) {
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
		Command: &enginev1.ContextSessionCommand_Screenshot{
			Screenshot: &enginev1.ScreenshotCommand{
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send ScreenshotCommand: %w", err)
	}

	for {
		event, err := t.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive tab session event during screenshot: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_ScreenshotCaptured:
			return &ScreenshotResult{
				PNGData: payload.ScreenshotCaptured.GetPngData(),
				Note:    payload.ScreenshotCaptured.GetNote(),
			}, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while capturing screenshot: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) Ping(ctx context.Context, message string) (string, error) {
	if t == nil {
		return "", fmt.Errorf("tab is nil")
	}
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return "", err
	}
	if t.closed {
		return "", fmt.Errorf("tab session %s is closed", t.sessionID)
	}

	if err := t.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: t.browserSessionID,
		ContextSessionId: t.sessionID,
		Command: &enginev1.ContextSessionCommand_Ping{
			Ping: &enginev1.ContextSessionPingCommand{
				Message: message,
			},
		},
	}); err != nil {
		return "", fmt.Errorf("send ContextSessionPingCommand: %w", err)
	}

	for {
		event, err := t.stream.Recv()
		if err != nil {
			return "", fmt.Errorf("receive tab session event during ping: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_Pong:
			return payload.Pong.GetMessage(), nil
		case *enginev1.ContextSessionEvent_Error:
			return "", fmt.Errorf("tab session error while pinging: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) Close(ctx context.Context) error {
	if t == nil {
		return nil
	}
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return err
	}
	if t.closed {
		return nil
	}

	if err := t.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: t.browserSessionID,
		ContextSessionId: t.sessionID,
		Command: &enginev1.ContextSessionCommand_Close{
			Close: &enginev1.CloseContextSessionCommand{},
		},
	}); err != nil {
		return fmt.Errorf("send CloseContextSessionCommand: %w", err)
	}

	for {
		event, err := t.stream.Recv()
		if err != nil {
			return fmt.Errorf("receive tab session event while closing: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_Closed:
			t.closed = true
			if err := t.stream.CloseSend(); err != nil {
				return fmt.Errorf("close tab session send side: %w", err)
			}
			_ = payload
			return nil
		case *enginev1.ContextSessionEvent_Error:
			return fmt.Errorf("tab session error while closing: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) ensureStream(ctx context.Context) error {
	if t == nil {
		return fmt.Errorf("tab is nil")
	}
	if t.runtime == nil {
		return fmt.Errorf("tab runtime is nil")
	}
	if t.stream != nil {
		return nil
	}

	stream, err := t.runtime.engine.ContextSession(ctx)
	if err != nil {
		return fmt.Errorf("open tab session stream: %w", err)
	}
	t.stream = stream
	return nil
}

func (t *Tab) readText(ctx context.Context, cssSelector string, textContent bool, options CommandOptions) (*TextResult, error) {
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
	cssSelector = normalizeSelectorForTransport(cssSelector)

	var sendErr error
	if textContent {
		sendErr = t.stream.Send(&enginev1.ContextSessionCommand{
			SurfaceSessionId: t.browserSessionID,
			ContextSessionId: t.sessionID,
			Command: &enginev1.ContextSessionCommand_GetTextContent{
				GetTextContent: &enginev1.GetTextContentCommand{
					CssSelector:  cssSelector,
					RetryOptions: retryOptionsProto(options.Timeout),
				},
			},
		})
	} else {
		sendErr = t.stream.Send(&enginev1.ContextSessionCommand{
			SurfaceSessionId: t.browserSessionID,
			ContextSessionId: t.sessionID,
			Command: &enginev1.ContextSessionCommand_GetInnerText{
				GetInnerText: &enginev1.GetInnerTextCommand{
					CssSelector:  cssSelector,
					RetryOptions: retryOptionsProto(options.Timeout),
				},
			},
		})
	}

	if sendErr != nil {
		return nil, fmt.Errorf("send text command: %w", sendErr)
	}

	for {
		event, err := t.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive tab session event during text read: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_TextContentResolved:
			return &TextResult{
				Selector: payload.TextContentResolved.GetCssSelector(),
				Text:     payload.TextContentResolved.GetText(),
				Note:     payload.TextContentResolved.GetNote(),
			}, nil
		case *enginev1.ContextSessionEvent_InnerTextResolved:
			return &TextResult{
				Selector: payload.InnerTextResolved.GetCssSelector(),
				Text:     payload.InnerTextResolved.GetText(),
				Note:     payload.InnerTextResolved.GetNote(),
			}, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while reading text: %s", payload.Error.GetMessage())
		}
	}
}
