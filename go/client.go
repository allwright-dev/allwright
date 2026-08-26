package allwright

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"

	enginev1 "allwright.dev/gen/allwright/engine/v1"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"gopkg.in/yaml.v3"
)

const (
	defaultServerAddr = "127.0.0.1:50051"
	serverAddrEnvVar  = "ALLWRIGHT_SERVER_ADDR"
)

var configFilenames = []string{
	"allwright.config.yaml",
	"allwright.config.yml",
	"allwright.config.json",
	".allwright/config.yaml",
	".allwright/config.yml",
	".allwright/config.json",
}

var runtimeState struct {
	mu                 sync.Mutex
	client             *runtimeClient
	serverAddrOverride string
}

type runtimeClient struct {
	conn   *grpc.ClientConn
	engine enginev1.EngineServiceClient
}

type BrowserType struct {
	browserKind enginev1.BrowserKind
}

type LaunchOptions struct {
	BrowserBinary string
	Timeout       time.Duration
}

type RetryConfig struct {
	TimeoutMs  uint32 `json:"timeoutMs,omitempty" yaml:"timeoutMs,omitempty"`
	IntervalMs uint32 `json:"intervalMs,omitempty" yaml:"intervalMs,omitempty"`
}

type configServer struct {
	Addr string `json:"addr,omitempty" yaml:"addr,omitempty"`
}

type configBrowser struct {
	Name          string        `json:"name,omitempty" yaml:"name,omitempty"`
	Binary        string        `json:"binary,omitempty" yaml:"binary,omitempty"`
	LaunchOptions *configLaunch `json:"launchOptions,omitempty" yaml:"launchOptions,omitempty"`
}

type configLaunch struct {
	BrowserBinary string `json:"browserBinary,omitempty" yaml:"browserBinary,omitempty"`
	TimeoutMs     uint32 `json:"timeoutMs,omitempty" yaml:"timeoutMs,omitempty"`
}

type suiteConfig struct {
	Server  *configServer  `json:"server,omitempty" yaml:"server,omitempty"`
	Browser *configBrowser `json:"browser,omitempty" yaml:"browser,omitempty"`
	Expect  *RetryConfig   `json:"expect,omitempty" yaml:"expect,omitempty"`
}

type AllwrightConfig struct {
	SchemaVersion uint32                 `json:"schemaVersion,omitempty" yaml:"schemaVersion,omitempty"`
	Server        *configServer          `json:"server,omitempty" yaml:"server,omitempty"`
	Browser       *configBrowser         `json:"browser,omitempty" yaml:"browser,omitempty"`
	Expect        *RetryConfig           `json:"expect,omitempty" yaml:"expect,omitempty"`
	Suites        map[string]suiteConfig `json:"suites,omitempty" yaml:"suites,omitempty"`
}

type ResolveConfigOptions struct {
	Cwd        string
	ConfigFile string
	Suite      string
}

type ResolvedConfig struct {
	ConfigFilePath string
	SuiteName      string
	ServerAddr     string
	BrowserName    string
	BrowserBinary  string
	LaunchOptions  LaunchOptions
	Expect         RetryConfig
}

type CommandOptions struct {
	Timeout time.Duration
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

type CountResult struct {
	Selector string
	Count    uint32
	Note     string
}

type HighlightOptions struct {
	Timeout  time.Duration
	Duration time.Duration
}

type HighlightResult struct {
	Selector string
	Count    uint32
	Note     string
}

type ElementResult struct {
	Selector string
	Note     string
}

type FillResult struct {
	Selector string
	Value    string
	Note     string
}

type PressOptions struct {
	Timeout time.Duration
	Text    string
}

type PressResult struct {
	Selector string
	Key      string
	Note     string
}

type TextResult struct {
	Selector string
	Text     string
	Note     string
}

type WaitForSelectorOptions struct {
	Timeout time.Duration
	Visible *bool
}

type WaitForSelectorResult struct {
	Selector string
	Visible  bool
	Note     string
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

type Page = Tab

type Locator struct {
	page     *Tab
	selector string
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

var (
	Chromium = BrowserType{browserKind: enginev1.BrowserKind_BROWSER_KIND_CHROMIUM}
	Firefox  = BrowserType{browserKind: enginev1.BrowserKind_BROWSER_KIND_FIREFOX}
)

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
	return LaunchBrowser(ctx, enginev1.BrowserKind_BROWSER_KIND_CHROMIUM, options)
}

func LaunchFirefox(ctx context.Context, options LaunchOptions) (*Browser, error) {
	return LaunchBrowser(ctx, enginev1.BrowserKind_BROWSER_KIND_FIREFOX, options)
}

func LaunchBrowser(ctx context.Context, browserKind enginev1.BrowserKind, options LaunchOptions) (*Browser, error) {
	runtime, err := getRuntime(ctx)
	if err != nil {
		return nil, err
	}

	stream, err := runtime.engine.BrowserSession(ctx)
	if err != nil {
		return nil, fmt.Errorf("open browser session stream: %w", err)
	}

	command := &enginev1.BrowserSessionCommand{
		Command: &enginev1.BrowserSessionCommand_LaunchBrowser{
			LaunchBrowser: &enginev1.LaunchBrowserCommand{
				BrowserKind:   browserKind,
				BrowserBinary: optionalString(options.BrowserBinary),
				RetryOptions:  retryOptionsProto(options.Timeout),
			},
		},
	}
	if err := stream.Send(command); err != nil {
		return nil, fmt.Errorf("send LaunchBrowserCommand: %w", err)
	}

	for {
		event, err := stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive browser session event after launch: %w", err)
		}

		switch payload := event.GetEvent().(type) {
		case *enginev1.BrowserSessionEvent_BrowserLaunched:
			browser := &Browser{
				runtime:         runtime,
				stream:          stream,
				sessionID:       event.GetSessionId(),
				browserName:     payload.BrowserLaunched.GetBrowser(),
				launchNote:      payload.BrowserLaunched.GetNote(),
				cdpWebSocketURL: "",
				userDataDir:     payload.BrowserLaunched.GetUserDataDir(),
			}
			browser.initialTab = &Tab{
				runtime:          runtime,
				browserSessionID: browser.sessionID,
				sessionID:        payload.BrowserLaunched.GetInitialTabSessionId(),
			}
			return browser, nil
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

func SetServerAddr(serverAddr string) error {
	runtimeState.mu.Lock()
	defer runtimeState.mu.Unlock()

	runtimeState.serverAddrOverride = strings.TrimSpace(serverAddr)
	if runtimeState.client == nil {
		return nil
	}

	err := runtimeState.client.conn.Close()
	runtimeState.client = nil
	return err
}

func FindConfigFile(startDir string) (string, error) {
	currentDir := startDir
	if strings.TrimSpace(currentDir) == "" {
		var err error
		currentDir, err = os.Getwd()
		if err != nil {
			return "", fmt.Errorf("get working directory: %w", err)
		}
	}
	currentDir, _ = filepath.Abs(currentDir)

	for {
		for _, filename := range configFilenames {
			candidate := filepath.Join(currentDir, filename)
			info, err := os.Stat(candidate)
			if err == nil && !info.IsDir() {
				return candidate, nil
			}
		}

		parentDir := filepath.Dir(currentDir)
		if parentDir == currentDir {
			return "", nil
		}
		currentDir = parentDir
	}
}

func LoadConfigFile(path string) (*AllwrightConfig, error) {
	resolved, err := filepath.Abs(path)
	if err != nil {
		return nil, fmt.Errorf("resolve config path: %w", err)
	}

	raw, err := os.ReadFile(resolved)
	if err != nil {
		return nil, fmt.Errorf("read config file %s: %w", resolved, err)
	}

	config := &AllwrightConfig{}
	switch strings.ToLower(filepath.Ext(resolved)) {
	case ".json":
		if err := json.Unmarshal(raw, config); err != nil {
			return nil, fmt.Errorf("parse config file %s as JSON: %w", resolved, err)
		}
	case ".yaml", ".yml":
		if err := yaml.Unmarshal(raw, config); err != nil {
			return nil, fmt.Errorf("parse config file %s as YAML: %w", resolved, err)
		}
	default:
		return nil, fmt.Errorf("unsupported allwright config file extension %q for %s", filepath.Ext(resolved), resolved)
	}

	if err := validateConfig(config, resolved); err != nil {
		return nil, err
	}
	return config, nil
}

func ResolveConfig(options ResolveConfigOptions) (*ResolvedConfig, error) {
	configFile := strings.TrimSpace(options.ConfigFile)
	if configFile == "" {
		resolved, err := FindConfigFile(options.Cwd)
		if err != nil {
			return nil, err
		}
		configFile = resolved
	}

	config := &AllwrightConfig{}
	if configFile != "" {
		loaded, err := LoadConfigFile(configFile)
		if err != nil {
			return nil, err
		}
		config = loaded
	}

	suiteName := strings.TrimSpace(options.Suite)
	var suite *suiteConfig
	if suiteName != "" {
		selected, ok := config.Suites[suiteName]
		if !ok {
			source := configFile
			if source == "" {
				source = "the resolved config file"
			}
			return nil, fmt.Errorf("allwright config suite %q was not found in %s", suiteName, source)
		}
		suite = &selected
	}

	browserName := "chromium"
	if config.Browser != nil && strings.TrimSpace(config.Browser.Name) != "" {
		browserName = strings.TrimSpace(config.Browser.Name)
	}
	if suite != nil && suite.Browser != nil && strings.TrimSpace(suite.Browser.Name) != "" {
		browserName = strings.TrimSpace(suite.Browser.Name)
	}

	browserBinary := firstNonEmpty(
		configBrowserBinaryFromSuite(suite),
		configBrowserBinary(config.Browser),
	)
	serverAddr := firstNonEmpty(
		configServerAddrFromSuite(suite),
		configServerAddr(config.Server),
	)
	launchOptions := mergeLaunchOptions(
		launchOptionsFromConfig(config.Browser),
		launchOptionsFromSuite(suite),
	)
	if browserBinary != "" {
		launchOptions.BrowserBinary = browserBinary
	}
	expect := mergeRetryConfig(config.Expect, suiteExpect(suite))

	return &ResolvedConfig{
		ConfigFilePath: configFile,
		SuiteName:      suiteName,
		ServerAddr:     serverAddr,
		BrowserName:    browserName,
		BrowserBinary:  browserBinary,
		LaunchOptions:  launchOptions,
		Expect:         expect,
	}, nil
}

func LaunchConfiguredBrowser(ctx context.Context, config *ResolvedConfig) (*Browser, error) {
	if config == nil {
		return nil, fmt.Errorf("resolved config is required")
	}
	if strings.TrimSpace(config.ServerAddr) != "" {
		if err := SetServerAddr(config.ServerAddr); err != nil {
			return nil, err
		}
	}

	switch strings.ToLower(strings.TrimSpace(config.BrowserName)) {
	case "", "chromium":
		return LaunchChrome(ctx, config.LaunchOptions)
	case "firefox":
		return LaunchFirefox(ctx, config.LaunchOptions)
	default:
		return nil, fmt.Errorf("unsupported browser.name %q; use \"chromium\" or \"firefox\"", config.BrowserName)
	}
}

func (bt BrowserType) Launch(ctx context.Context, options LaunchOptions) (*Browser, error) {
	return LaunchBrowser(ctx, bt.browserKind, options)
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
	b.mu.Lock()
	defer b.mu.Unlock()

	if b.closed {
		return nil, fmt.Errorf("browser session %s is closed", b.sessionID)
	}

	commandOptions := firstCommandOptions(options)

	if err := b.stream.Send(&enginev1.BrowserSessionCommand{
		Command: &enginev1.BrowserSessionCommand_OpenTab{
			OpenTab: &enginev1.OpenTabCommand{
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
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

func (b *Browser) NewPage(ctx context.Context, options ...CommandOptions) (*Page, error) {
	return b.NewTab(ctx, options...)
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

func (t *Tab) Locator(selector string) *Locator {
	if t == nil {
		return nil
	}
	return &Locator{
		page:     t,
		selector: selector,
	}
}

func (t *Tab) Goto(ctx context.Context, url string, options ...CommandOptions) (*NavigateResult, error) {
	return t.Navigate(ctx, url, options...)
}

func (t *Tab) Navigate(ctx context.Context, url string, options ...CommandOptions) (*NavigateResult, error) {
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return nil, err
	}
	if t.closed {
		return nil, fmt.Errorf("tab session %s is closed", t.sessionID)
	}

	commandOptions := firstCommandOptions(options)

	if err := t.stream.Send(&enginev1.TabSessionCommand{
		BrowserSessionId: t.browserSessionID,
		TabSessionId:     t.sessionID,
		Command: &enginev1.TabSessionCommand_Navigate{
			Navigate: &enginev1.NavigateTabCommand{
				Url:          url,
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
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

func (t *Tab) Click(ctx context.Context, cssSelector string, options ...CommandOptions) (*ClickResult, error) {
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return nil, err
	}
	if t.closed {
		return nil, fmt.Errorf("tab session %s is closed", t.sessionID)
	}

	commandOptions := firstCommandOptions(options)

	if err := t.stream.Send(&enginev1.TabSessionCommand{
		BrowserSessionId: t.browserSessionID,
		TabSessionId:     t.sessionID,
		Command: &enginev1.TabSessionCommand_ClickElement{
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

func (t *Tab) Count(ctx context.Context, cssSelector string, options ...CommandOptions) (*CountResult, error) {
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return nil, err
	}
	if t.closed {
		return nil, fmt.Errorf("tab session %s is closed", t.sessionID)
	}

	commandOptions := firstCommandOptions(options)

	if err := t.stream.Send(&enginev1.TabSessionCommand{
		BrowserSessionId: t.browserSessionID,
		TabSessionId:     t.sessionID,
		Command: &enginev1.TabSessionCommand_CountElements{
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
		case *enginev1.TabSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.TabSessionEvent_ElementCounted:
			return &CountResult{
				Selector: payload.ElementCounted.GetCssSelector(),
				Count:    payload.ElementCounted.GetCount(),
				Note:     payload.ElementCounted.GetNote(),
			}, nil
		case *enginev1.TabSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while counting elements: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) Highlight(ctx context.Context, cssSelector string, options ...HighlightOptions) (*HighlightResult, error) {
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return nil, err
	}
	if t.closed {
		return nil, fmt.Errorf("tab session %s is closed", t.sessionID)
	}

	highlightOptions := firstHighlightOptions(options)

	if err := t.stream.Send(&enginev1.TabSessionCommand{
		BrowserSessionId: t.browserSessionID,
		TabSessionId:     t.sessionID,
		Command: &enginev1.TabSessionCommand_HighlightElements{
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
		case *enginev1.TabSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.TabSessionEvent_ElementsHighlighted:
			return &HighlightResult{
				Selector: payload.ElementsHighlighted.GetCssSelector(),
				Count:    payload.ElementsHighlighted.GetCount(),
				Note:     payload.ElementsHighlighted.GetNote(),
			}, nil
		case *enginev1.TabSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while highlighting elements: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) Focus(ctx context.Context, cssSelector string, options ...CommandOptions) (*ElementResult, error) {
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return nil, err
	}
	if t.closed {
		return nil, fmt.Errorf("tab session %s is closed", t.sessionID)
	}

	commandOptions := firstCommandOptions(options)

	if err := t.stream.Send(&enginev1.TabSessionCommand{
		BrowserSessionId: t.browserSessionID,
		TabSessionId:     t.sessionID,
		Command: &enginev1.TabSessionCommand_FocusElement{
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
		case *enginev1.TabSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.TabSessionEvent_ElementFocused:
			return &ElementResult{
				Selector: payload.ElementFocused.GetCssSelector(),
				Note:     payload.ElementFocused.GetNote(),
			}, nil
		case *enginev1.TabSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while focusing: %s", payload.Error.GetMessage())
		}
	}
}

func (t *Tab) Fill(ctx context.Context, cssSelector string, value string, options ...CommandOptions) (*FillResult, error) {
	t.mu.Lock()
	defer t.mu.Unlock()

	if err := t.ensureStream(ctx); err != nil {
		return nil, err
	}
	if t.closed {
		return nil, fmt.Errorf("tab session %s is closed", t.sessionID)
	}

	commandOptions := firstCommandOptions(options)

	if err := t.stream.Send(&enginev1.TabSessionCommand{
		BrowserSessionId: t.browserSessionID,
		TabSessionId:     t.sessionID,
		Command: &enginev1.TabSessionCommand_FillElement{
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
		case *enginev1.TabSessionEvent_Attached:
			t.attached = true
			_ = payload
		case *enginev1.TabSessionEvent_ElementFilled:
			return &FillResult{
				Selector: payload.ElementFilled.GetCssSelector(),
				Value:    payload.ElementFilled.GetValue(),
				Note:     payload.ElementFilled.GetNote(),
			}, nil
		case *enginev1.TabSessionEvent_Error:
			return nil, fmt.Errorf("tab session error while filling: %s", payload.Error.GetMessage())
		}
	}
}

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

func retryOptionsProto(timeout time.Duration) *enginev1.CommandRetryOptions {
	if timeout <= 0 {
		return nil
	}

	timeoutMS := timeout.Milliseconds()
	if timeoutMS <= 0 {
		timeoutMS = 1
	}

	return &enginev1.CommandRetryOptions{
		TimeoutMs:       optionalUint32(uint32(timeoutMS)),
		RetryIntervalMs: nil,
	}
}

func optionalUint32(value uint32) *uint32 {
	return &value
}

func firstCommandOptions(options []CommandOptions) CommandOptions {
	if len(options) > 0 {
		return options[0]
	}
	return CommandOptions{}
}

func firstHighlightOptions(options []HighlightOptions) HighlightOptions {
	if len(options) > 0 {
		return options[0]
	}
	return HighlightOptions{}
}

func firstPressOptions(options []PressOptions) PressOptions {
	if len(options) > 0 {
		return options[0]
	}
	return PressOptions{}
}

func firstWaitForSelectorOptions(options []WaitForSelectorOptions) WaitForSelectorOptions {
	if len(options) > 0 {
		return options[0]
	}
	return WaitForSelectorOptions{}
}

func durationProto(value time.Duration) *uint32 {
	if value <= 0 {
		return nil
	}

	durationMS := value.Milliseconds()
	if durationMS <= 0 {
		durationMS = 1
	}

	return optionalUint32(uint32(durationMS))
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
		selector: strings.TrimSpace(l.selector + " " + selector),
	}
}

func (l *Locator) Click(ctx context.Context, options ...CommandOptions) (*ClickResult, error) {
	return l.page.Click(ctx, l.selector, options...)
}

func (l *Locator) Count(ctx context.Context, options ...CommandOptions) (*CountResult, error) {
	return l.page.Count(ctx, l.selector, options...)
}

func (l *Locator) Highlight(ctx context.Context, options ...HighlightOptions) (*HighlightResult, error) {
	return l.page.Highlight(ctx, l.selector, options...)
}

func (l *Locator) Focus(ctx context.Context, options ...CommandOptions) (*ElementResult, error) {
	return l.page.Focus(ctx, l.selector, options...)
}

func (l *Locator) Fill(ctx context.Context, value string, options ...CommandOptions) (*FillResult, error) {
	return l.page.Fill(ctx, l.selector, value, options...)
}

func (l *Locator) Hover(ctx context.Context, options ...CommandOptions) (*ElementResult, error) {
	return l.page.Hover(ctx, l.selector, options...)
}

func (l *Locator) Press(ctx context.Context, key string, options ...PressOptions) (*PressResult, error) {
	return l.page.Press(ctx, l.selector, key, options...)
}

func (l *Locator) TextContent(ctx context.Context, options ...CommandOptions) (*TextResult, error) {
	return l.page.TextContent(ctx, l.selector, options...)
}

func (l *Locator) InnerText(ctx context.Context, options ...CommandOptions) (*TextResult, error) {
	return l.page.InnerText(ctx, l.selector, options...)
}

func (l *Locator) WaitFor(ctx context.Context, options ...WaitForSelectorOptions) (*WaitForSelectorResult, error) {
	return l.page.WaitForSelector(ctx, l.selector, options...)
}

func getRuntime(ctx context.Context) (*runtimeClient, error) {
	runtimeState.mu.Lock()
	defer runtimeState.mu.Unlock()

	if runtimeState.client != nil {
		return runtimeState.client, nil
	}

	serverAddr := resolveServerAddr()

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

func resolveServerAddr() string {
	if strings.TrimSpace(runtimeState.serverAddrOverride) != "" {
		return runtimeState.serverAddrOverride
	}

	serverAddr := strings.TrimSpace(os.Getenv(serverAddrEnvVar))
	if serverAddr == "" {
		return defaultServerAddr
	}
	return serverAddr
}

func validateConfig(config *AllwrightConfig, source string) error {
	if config == nil {
		return nil
	}
	if config.SchemaVersion != 0 && config.SchemaVersion != 1 {
		return fmt.Errorf("allwright config %s has unsupported schemaVersion %d; expected 1", source, config.SchemaVersion)
	}

	if err := validateBrowserName(source, configBrowserName(config.Browser)); err != nil {
		return err
	}
	for suiteName, suite := range config.Suites {
		if err := validateBrowserName(source, configBrowserName(suite.Browser)); err != nil {
			return fmt.Errorf("suite %q: %w", suiteName, err)
		}
	}
	return nil
}

func validateBrowserName(source string, browserName string) error {
	switch strings.ToLower(strings.TrimSpace(browserName)) {
	case "", "chromium", "firefox":
		return nil
	default:
		return fmt.Errorf("allwright config %s has unsupported browser.name %q; use \"chromium\" or \"firefox\"", source, browserName)
	}
}

func configBrowserName(browser *configBrowser) string {
	if browser == nil {
		return ""
	}
	return strings.TrimSpace(browser.Name)
}

func configBrowserBinary(browser *configBrowser) string {
	if browser == nil {
		return ""
	}
	return strings.TrimSpace(browser.Binary)
}

func configBrowserBinaryFromSuite(suite *suiteConfig) string {
	if suite == nil {
		return ""
	}
	return configBrowserBinary(suite.Browser)
}

func configServerAddr(server *configServer) string {
	if server == nil {
		return ""
	}
	return strings.TrimSpace(server.Addr)
}

func configServerAddrFromSuite(suite *suiteConfig) string {
	if suite == nil {
		return ""
	}
	return configServerAddr(suite.Server)
}

func launchOptionsFromConfig(browser *configBrowser) LaunchOptions {
	if browser == nil || browser.LaunchOptions == nil {
		return LaunchOptions{}
	}
	options := LaunchOptions{}
	if strings.TrimSpace(browser.LaunchOptions.BrowserBinary) != "" {
		options.BrowserBinary = strings.TrimSpace(browser.LaunchOptions.BrowserBinary)
	}
	if browser.LaunchOptions.TimeoutMs > 0 {
		options.Timeout = time.Duration(browser.LaunchOptions.TimeoutMs) * time.Millisecond
	}
	return options
}

func launchOptionsFromSuite(suite *suiteConfig) LaunchOptions {
	if suite == nil {
		return LaunchOptions{}
	}
	return launchOptionsFromConfig(suite.Browser)
}

func mergeLaunchOptions(base LaunchOptions, override LaunchOptions) LaunchOptions {
	merged := base
	if strings.TrimSpace(override.BrowserBinary) != "" {
		merged.BrowserBinary = strings.TrimSpace(override.BrowserBinary)
	}
	if override.Timeout > 0 {
		merged.Timeout = override.Timeout
	}
	return merged
}

func suiteExpect(suite *suiteConfig) *RetryConfig {
	if suite == nil {
		return nil
	}
	return suite.Expect
}

func mergeRetryConfig(base *RetryConfig, override *RetryConfig) RetryConfig {
	merged := RetryConfig{}
	if base != nil {
		merged = *base
	}
	if override != nil {
		if override.TimeoutMs > 0 {
			merged.TimeoutMs = override.TimeoutMs
		}
		if override.IntervalMs > 0 {
			merged.IntervalMs = override.IntervalMs
		}
	}
	return merged
}

func firstNonEmpty(values ...string) string {
	for _, value := range values {
		trimmed := strings.TrimSpace(value)
		if trimmed != "" {
			return trimmed
		}
	}
	return ""
}
