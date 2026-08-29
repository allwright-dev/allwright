package allwright

import (
	"context"
	"fmt"

	enginev1 "allwright.dev/gen/allwright/engine/v1"
)

func newBrowserFromLaunch(
	runtime *runtimeClient,
	stream browserSessionStream,
	sessionID string,
	browserName string,
	launchNote string,
	cdpWebSocketURL string,
	userDataDir string,
	initialPageSessionID string,
) *Browser {
	browser := &Browser{
		runtime:         runtime,
		stream:          stream,
		sessionID:       sessionID,
		browserName:     browserName,
		launchNote:      launchNote,
		cdpWebSocketURL: cdpWebSocketURL,
		userDataDir:     userDataDir,
	}
	browser.initialTab = &Tab{
		runtime:          runtime,
		browserSessionID: browser.sessionID,
		sessionID:        initialPageSessionID,
	}
	return browser
}

func (b *Browser) SessionID() string {
	if b == nil {
		return ""
	}
	return b.sessionID
}

func (b *Browser) BrowserName() string {
	if b == nil {
		return ""
	}
	return b.browserName
}

func (b *Browser) LaunchNote() string {
	if b == nil {
		return ""
	}
	return b.launchNote
}

func (b *Browser) CdpWebSocketURL() string {
	if b == nil {
		return ""
	}
	return b.cdpWebSocketURL
}

func (b *Browser) UserDataDir() string {
	if b == nil {
		return ""
	}
	return b.userDataDir
}

func (b *Browser) InitialTab() *Tab {
	if b == nil {
		return nil
	}
	return b.initialTab
}

func (b *Browser) Page() *Page {
	return b.InitialTab()
}

func (b *Browser) InitialPage() *Page {
	return b.InitialTab()
}

func (b *Browser) NewTab(ctx context.Context, options ...CommandOptions) (*Tab, error) {
	if b == nil {
		return nil, fmt.Errorf("browser is nil")
	}
	b.mu.Lock()
	defer b.mu.Unlock()

	if b.closed {
		return nil, fmt.Errorf("browser session %s is closed", b.sessionID)
	}

	commandOptions := firstCommandOptions(options)

	if err := b.stream.Send(&enginev1.SurfaceSessionCommand{
		Command: &enginev1.SurfaceSessionCommand_OpenContext{
			OpenContext: &enginev1.OpenContextCommand{
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send OpenContextCommand: %w", err)
	}

	for {
		event, err := b.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive browser session event after opening tab: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.SurfaceSessionEvent_ContextOpened:
			return &Tab{
				runtime:          b.runtime,
				browserSessionID: b.sessionID,
				sessionID:        payload.ContextOpened.GetContextSessionId(),
			}, nil
		case *enginev1.SurfaceSessionEvent_Error:
			return nil, fmt.Errorf("browser session error while opening tab: %s", payload.Error.GetMessage())
		}
	}
}

func (b *Browser) NewPage(ctx context.Context, options ...CommandOptions) (*Page, error) {
	if b == nil {
		return nil, fmt.Errorf("browser is nil")
	}
	return b.NewTab(ctx, options...)
}

func (b *Browser) Ping(ctx context.Context, message string) (string, error) {
	if b == nil {
		return "", fmt.Errorf("browser is nil")
	}
	b.mu.Lock()
	defer b.mu.Unlock()

	if b.closed {
		return "", fmt.Errorf("browser session %s is closed", b.sessionID)
	}

	if err := b.stream.Send(&enginev1.SurfaceSessionCommand{
		Command: &enginev1.SurfaceSessionCommand_Ping{
			Ping: &enginev1.SessionPingCommand{
				Message: message,
			},
		},
	}); err != nil {
		return "", fmt.Errorf("send SessionPingCommand: %w", err)
	}

	for {
		event, err := b.stream.Recv()
		if err != nil {
			return "", fmt.Errorf("receive browser session event after ping: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.SurfaceSessionEvent_Pong:
			return payload.Pong.GetMessage(), nil
		case *enginev1.SurfaceSessionEvent_Error:
			return "", fmt.Errorf("browser session error while pinging: %s", payload.Error.GetMessage())
		}
	}
}

func (b *Browser) Close(ctx context.Context) error {
	if b == nil {
		return nil
	}
	b.mu.Lock()
	defer b.mu.Unlock()

	if b.closed {
		return nil
	}

	if err := b.stream.Send(&enginev1.SurfaceSessionCommand{
		Command: &enginev1.SurfaceSessionCommand_Close{
			Close: &enginev1.CloseSurfaceSessionCommand{},
		},
	}); err != nil {
		return fmt.Errorf("send CloseSurfaceSessionCommand: %w", err)
	}

	for {
		event, err := b.stream.Recv()
		if err != nil {
			return fmt.Errorf("receive browser session event while closing: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.SurfaceSessionEvent_Closed:
			b.closed = true
			if err := b.stream.CloseSend(); err != nil {
				return fmt.Errorf("close browser session send side: %w", err)
			}
			_ = payload
			return nil
		case *enginev1.SurfaceSessionEvent_Error:
			return fmt.Errorf("browser session error while closing: %s", payload.Error.GetMessage())
		}
	}
}
