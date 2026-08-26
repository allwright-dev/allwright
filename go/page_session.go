package allwright

import (
	"context"
	"fmt"

	enginev1 "allwright.dev/gen/allwright/engine/v1"
)

func (t *Tab) Hover(ctx context.Context, cssSelector string, options ...CommandOptions) (*ElementResult, error) {
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

	if err := t.stream.Send(&enginev1.TabSessionCommand{
		BrowserSessionId: t.browserSessionID,
		TabSessionId:     t.sessionID,
		Command: &enginev1.TabSessionCommand_HoverElement{
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
		case *enginev1.TabSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.TabSessionEvent_ElementHovered:
			return &ElementResult{
				Selector: payload.ElementHovered.GetCssSelector(),
				Note:     payload.ElementHovered.GetNote(),
			}, nil
		case *enginev1.TabSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while hovering: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) Press(ctx context.Context, cssSelector string, key string, options ...PressOptions) (*PressResult, error) {
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

	if err := t.stream.Send(&enginev1.TabSessionCommand{
		BrowserSessionId: t.browserSessionID,
		TabSessionId:     t.sessionID,
		Command: &enginev1.TabSessionCommand_PressKey{
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
		case *enginev1.TabSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.TabSessionEvent_KeyPressed:
			return &PressResult{
				Selector: payload.KeyPressed.GetCssSelector(),
				Key:      payload.KeyPressed.GetKey(),
				Note:     payload.KeyPressed.GetNote(),
			}, nil
		case *enginev1.TabSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while pressing key: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) TextContent(ctx context.Context, cssSelector string, options ...CommandOptions) (*TextResult, error) {
	return t.readText(ctx, cssSelector, true, firstCommandOptions(options))
}

func (t *Tab) InnerText(ctx context.Context, cssSelector string, options ...CommandOptions) (*TextResult, error) {
	return t.readText(ctx, cssSelector, false, firstCommandOptions(options))
}

func (t *Tab) WaitForSelector(ctx context.Context, cssSelector string, options ...WaitForSelectorOptions) (*WaitForSelectorResult, error) {
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

	if err := t.stream.Send(&enginev1.TabSessionCommand{
		BrowserSessionId: t.browserSessionID,
		TabSessionId:     t.sessionID,
		Command: &enginev1.TabSessionCommand_WaitForSelector{
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
		case *enginev1.TabSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.TabSessionEvent_SelectorWaitSatisfied:
			return &WaitForSelectorResult{
				Selector: payload.SelectorWaitSatisfied.GetCssSelector(),
				Visible:  payload.SelectorWaitSatisfied.GetVisible(),
				Note:     payload.SelectorWaitSatisfied.GetNote(),
			}, nil
		case *enginev1.TabSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while waiting for selector: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) Ping(ctx context.Context, message string) (string, error) {
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return "", err
	}
	if t.closed {
		return "", fmt.Errorf("tab session %s is closed", t.sessionID)
	}

	if err := t.stream.Send(&enginev1.TabSessionCommand{
		BrowserSessionId: t.browserSessionID,
		TabSessionId:     t.sessionID,
		Command: &enginev1.TabSessionCommand_Ping{
			Ping: &enginev1.TabSessionPingCommand{
				Message: message,
			},
		},
	}); err != nil {
		return "", fmt.Errorf("send TabSessionPingCommand: %w", err)
	}

	for {
		event, err := t.stream.Recv()
		if err != nil {
			return "", fmt.Errorf("receive tab session event during ping: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.TabSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.TabSessionEvent_Pong:
			return payload.Pong.GetMessage(), nil
		case *enginev1.TabSessionEvent_Error:
			return "", fmt.Errorf("tab session error while pinging: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) Close(ctx context.Context) error {
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return err
	}
	if t.closed {
		return nil
	}

	if err := t.stream.Send(&enginev1.TabSessionCommand{
		BrowserSessionId: t.browserSessionID,
		TabSessionId:     t.sessionID,
		Command: &enginev1.TabSessionCommand_Close{
			Close: &enginev1.CloseTabSessionCommand{},
		},
	}); err != nil {
		return fmt.Errorf("send CloseTabSessionCommand: %w", err)
	}

	for {
		event, err := t.stream.Recv()
		if err != nil {
			return fmt.Errorf("receive tab session event while closing: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.TabSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.TabSessionEvent_Closed:
			t.closed = true
			if err := t.stream.CloseSend(); err != nil {
				return fmt.Errorf("close tab session send side: %w", err)
			}
			_ = payload
			return nil
		case *enginev1.TabSessionEvent_Error:
			return fmt.Errorf("tab session error while closing: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) ensureStream(ctx context.Context) error {
	if t.stream != nil {
		return nil
	}

	stream, err := t.runtime.engine.TabSession(ctx)
	if err != nil {
		return fmt.Errorf("open tab session stream: %w", err)
	}
	t.stream = stream
	return nil
}

func (t *Tab) readText(ctx context.Context, cssSelector string, textContent bool, options CommandOptions) (*TextResult, error) {
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
		sendErr = t.stream.Send(&enginev1.TabSessionCommand{
			BrowserSessionId: t.browserSessionID,
			TabSessionId:     t.sessionID,
			Command: &enginev1.TabSessionCommand_GetTextContent{
				GetTextContent: &enginev1.GetTextContentCommand{
					CssSelector:  cssSelector,
					RetryOptions: retryOptionsProto(options.Timeout),
				},
			},
		})
	} else {
		sendErr = t.stream.Send(&enginev1.TabSessionCommand{
			BrowserSessionId: t.browserSessionID,
			TabSessionId:     t.sessionID,
			Command: &enginev1.TabSessionCommand_GetInnerText{
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
		case *enginev1.TabSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.TabSessionEvent_TextContentResolved:
			return &TextResult{
				Selector: payload.TextContentResolved.GetCssSelector(),
				Text:     payload.TextContentResolved.GetText(),
				Note:     payload.TextContentResolved.GetNote(),
			}, nil
		case *enginev1.TabSessionEvent_InnerTextResolved:
			return &TextResult{
				Selector: payload.InnerTextResolved.GetCssSelector(),
				Text:     payload.InnerTextResolved.GetText(),
				Note:     payload.InnerTextResolved.GetNote(),
			}, nil
		case *enginev1.TabSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while reading text: %s", payload.Error.GetMessage())
		}
	}
}
