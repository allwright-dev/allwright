package allwright

import (
	"context"
	"fmt"
	"os"
	"strings"
	"sync"

	enginev1 "allwright.dev/gen/allwright/engine/v1"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

const (
	defaultServerAddr = "127.0.0.1:50051"
	serverAddrEnvVar  = "ALLWRIGHT_SERVER_ADDR"
)

var runtimeState struct {
	mu     sync.Mutex
	client *runtimeClient
}

type runtimeClient struct {
	conn   *grpc.ClientConn
	engine enginev1.EngineServiceClient
}

type LaunchOptions struct {
	ChromeBinary string
}

type NavigateResult struct {
	URL             string
	Note            string
	BidiSessionID   string
	MapperTargetID  string
	MapperSessionID string
	PackageVersion  string
}

type ClickResult struct {
	Selector      string
	Note          string
	BidiSessionID string
}

type Browser struct {
	mu              sync.Mutex
	runtime         *runtimeClient
	stream          browserSessionStream
	sessionID       string
	browserName     string
	launchNote      string
	cdpWebSocketURL string
	userDataDir     string
	initialTab      *Tab
	closed          bool
}

type Tab struct {
	mu                sync.Mutex
	runtime           *runtimeClient
	stream            tabSessionStream
	browserSessionID  string
	sessionID         string
	attached          bool
	closed            bool
	lastBidiSessionID string
}

type browserSessionStream interface {
	Send(*enginev1.BrowserSessionCommand) error
	Recv() (*enginev1.BrowserSessionEvent, error)
	CloseSend() error
}

type tabSessionStream interface {
	Send(*enginev1.TabSessionCommand) error
	Recv() (*enginev1.TabSessionEvent, error)
	CloseSend() error
}

func Ping(ctx context.Context) (string, error) {
	runtime, err := getRuntime(ctx)
	if err != nil {
		return "", err
	}

	response, err := runtime.engine.Ping(ctx, &enginev1.PingRequest{})
	if err != nil {
		return "", fmt.Errorf("ping engine server: %w", err)
	}
	return response.GetMessage(), nil
}

func LaunchChrome(ctx context.Context, options LaunchOptions) (*Browser, error) {
	runtime, err := getRuntime(ctx)
	if err != nil {
		return nil, err
	}

	stream, err := runtime.engine.BrowserSession(ctx)
	if err != nil {
		return nil, fmt.Errorf("open browser session stream: %w", err)
	}

	command := &enginev1.BrowserSessionCommand{
		Command: &enginev1.BrowserSessionCommand_LaunchChrome{
			LaunchChrome: &enginev1.LaunchChromeCommand{
				ChromeBinary: optionalString(options.ChromeBinary),
			},
		},
	}
	if err := stream.Send(command); err != nil {
		return nil, fmt.Errorf("send LaunchChromeCommand: %w", err)
	}

	for {
		event, err := stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive browser session event after launch: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.BrowserSessionEvent_ChromeLaunched:
			browser := &Browser{
				runtime:         runtime,
				stream:          stream,
				sessionID:       event.GetSessionId(),
				browserName:     payload.ChromeLaunched.GetBrowser(),
				launchNote:      payload.ChromeLaunched.GetNote(),
				cdpWebSocketURL: payload.ChromeLaunched.GetCdpWebsocketUrl(),
				userDataDir:     payload.ChromeLaunched.GetUserDataDir(),
			}
			browser.initialTab = &Tab{
				runtime:          runtime,
				browserSessionID: browser.sessionID,
				sessionID:        payload.ChromeLaunched.GetInitialTabSessionId(),
			}
			return browser, nil
		case *enginev1.BrowserSessionEvent_Error:
			return nil, fmt.Errorf("browser session error during launch: %s", payload.Error.GetMessage())
		}
	}
}

func Shutdown() error {
	runtimeState.mu.Lock()
	defer runtimeState.mu.Unlock()

	if runtimeState.client == nil {
		return nil
	}

	err := runtimeState.client.conn.Close()
	runtimeState.client = nil
	return err
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

func (b *Browser) NewTab(ctx context.Context) (*Tab, error) {
	b.mu.Lock()
	defer b.mu.Unlock()

	if b.closed {
		return nil, fmt.Errorf("browser session %s is closed", b.sessionID)
	}

	if err := b.stream.Send(&enginev1.BrowserSessionCommand{
		Command: &enginev1.BrowserSessionCommand_OpenTab{
			OpenTab: &enginev1.OpenTabCommand{},
		},
	}); err != nil {
		return nil, fmt.Errorf("send OpenTabCommand: %w", err)
	}

	for {
		event, err := b.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive browser session event after opening tab: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.BrowserSessionEvent_TabOpened:
			return &Tab{
				runtime:          b.runtime,
				browserSessionID: b.sessionID,
				sessionID:        payload.TabOpened.GetTabSessionId(),
			}, nil
		case *enginev1.BrowserSessionEvent_Error:
			return nil, fmt.Errorf("browser session error while opening tab: %s", payload.Error.GetMessage())
		}
	}
}

func (b *Browser) Ping(ctx context.Context, message string) (string, error) {
	b.mu.Lock()
	defer b.mu.Unlock()

	if b.closed {
		return "", fmt.Errorf("browser session %s is closed", b.sessionID)
	}

	if err := b.stream.Send(&enginev1.BrowserSessionCommand{
		Command: &enginev1.BrowserSessionCommand_Ping{
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
		case *enginev1.BrowserSessionEvent_Pong:
			return payload.Pong.GetMessage(), nil
		case *enginev1.BrowserSessionEvent_Error:
			return "", fmt.Errorf("browser session error while pinging: %s", payload.Error.GetMessage())
		}
	}
}

func (b *Browser) Close(ctx context.Context) error {
	b.mu.Lock()
	defer b.mu.Unlock()

	if b.closed {
		return nil
	}

	if err := b.stream.Send(&enginev1.BrowserSessionCommand{
		Command: &enginev1.BrowserSessionCommand_Close{
			Close: &enginev1.CloseBrowserSessionCommand{},
		},
	}); err != nil {
		return fmt.Errorf("send CloseBrowserSessionCommand: %w", err)
	}

	for {
		event, err := b.stream.Recv()
		if err != nil {
			return fmt.Errorf("receive browser session event while closing: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.BrowserSessionEvent_Closed:
			b.closed = true
			if err := b.stream.CloseSend(); err != nil {
				return fmt.Errorf("close browser session send side: %w", err)
			}
			_ = payload
			return nil
		case *enginev1.BrowserSessionEvent_Error:
			return fmt.Errorf("browser session error while closing: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) SessionID() string {
	if t == nil {
		return ""
	}
	return t.sessionID
}

func (t *Tab) Navigate(ctx context.Context, url string) (*NavigateResult, error) {
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return nil, err
	}
	if t.closed {
		return nil, fmt.Errorf("tab session %s is closed", t.sessionID)
	}

	if err := t.stream.Send(&enginev1.TabSessionCommand{
		BrowserSessionId: t.browserSessionID,
		TabSessionId:     t.sessionID,
		Command: &enginev1.TabSessionCommand_Navigate{
			Navigate: &enginev1.NavigateTabCommand{
				Url: url,
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send NavigateTabCommand: %w", err)
	}

	result := &NavigateResult{}
	for {
		event, err := t.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive tab session event during navigate: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.TabSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.TabSessionEvent_Navigated:
			result.URL = payload.Navigated.GetUrl()
			result.Note = payload.Navigated.GetNote()
		case *enginev1.TabSessionEvent_ChromiumBidiInjection:
			result.BidiSessionID = payload.ChromiumBidiInjection.GetBidiSessionId()
			result.MapperTargetID = payload.ChromiumBidiInjection.GetMapperTargetId()
			result.MapperSessionID = payload.ChromiumBidiInjection.GetMapperSessionId()
			result.PackageVersion = payload.ChromiumBidiInjection.GetPackageVersion()
			t.lastBidiSessionID = result.BidiSessionID
			return result, nil
		case *enginev1.TabSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while navigating: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) Click(ctx context.Context, cssSelector string) (*ClickResult, error) {
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return nil, err
	}
	if t.closed {
		return nil, fmt.Errorf("tab session %s is closed", t.sessionID)
	}

	if err := t.stream.Send(&enginev1.TabSessionCommand{
		BrowserSessionId: t.browserSessionID,
		TabSessionId:     t.sessionID,
		Command: &enginev1.TabSessionCommand_ClickElement{
			ClickElement: &enginev1.ClickElementCommand{
				CssSelector: cssSelector,
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
		case *enginev1.TabSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.TabSessionEvent_ElementClicked:
			t.lastBidiSessionID = payload.ElementClicked.GetBidiSessionId()
			return &ClickResult{
				Selector:      payload.ElementClicked.GetCssSelector(),
				Note:          payload.ElementClicked.GetNote(),
				BidiSessionID: payload.ElementClicked.GetBidiSessionId(),
			}, nil
		case *enginev1.TabSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while clicking: %s", payload.Error.GetMessage())
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

func getRuntime(ctx context.Context) (*runtimeClient, error) {
	runtimeState.mu.Lock()
	defer runtimeState.mu.Unlock()

	if runtimeState.client != nil {
		return runtimeState.client, nil
	}

	serverAddr := strings.TrimSpace(os.Getenv(serverAddrEnvVar))
	if serverAddr == "" {
		serverAddr = defaultServerAddr
	}

	conn, err := grpc.NewClient(serverAddr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return nil, fmt.Errorf("dial engine server at %s: %w", serverAddr, err)
	}

	runtimeState.client = &runtimeClient{
		conn:   conn,
		engine: enginev1.NewEngineServiceClient(conn),
	}
	return runtimeState.client, nil
}

func optionalString(value string) *string {
	trimmed := strings.TrimSpace(value)
	if trimmed == "" {
		return nil
	}
	return &trimmed
}
