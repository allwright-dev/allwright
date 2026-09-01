package allwright

import (
	"sync"
	"time"

	enginev1 "allwright.dev/gen/allwright/engine/v1"
)

type runtimeClient struct {
	conn   grpcClientConn
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

type configWeb struct {
	Browser *configBrowser `json:"browser,omitempty" yaml:"browser,omitempty"`
}

type configLaunch struct {
	BrowserBinary string `json:"browserBinary,omitempty" yaml:"browserBinary,omitempty"`
	TimeoutMs     uint32 `json:"timeoutMs,omitempty" yaml:"timeoutMs,omitempty"`
}

type configApp struct {
	ID       string `json:"id,omitempty" yaml:"id,omitempty"`
	Binary   string `json:"binary,omitempty" yaml:"binary,omitempty"`
	Activity string `json:"activity,omitempty" yaml:"activity,omitempty"`
}

type configMobileTarget struct {
	Device string     `json:"device,omitempty" yaml:"device,omitempty"`
	App    *configApp `json:"app,omitempty" yaml:"app,omitempty"`
}

type configMobile struct {
	Android *configMobileTarget `json:"android,omitempty" yaml:"android,omitempty"`
	IOS     *configMobileTarget `json:"ios,omitempty" yaml:"ios,omitempty"`
}

type configDesktopTarget struct {
	App *configApp `json:"app,omitempty" yaml:"app,omitempty"`
}

type configDesktop struct {
	Mac     *configDesktopTarget `json:"mac,omitempty" yaml:"mac,omitempty"`
	Windows *configDesktopTarget `json:"windows,omitempty" yaml:"windows,omitempty"`
	Linux   *configDesktopTarget `json:"linux,omitempty" yaml:"linux,omitempty"`
}

type suiteConfig struct {
	Server  *configServer  `json:"server,omitempty" yaml:"server,omitempty"`
	Web     *configWeb     `json:"web,omitempty" yaml:"web,omitempty"`
	Mobile  *configMobile  `json:"mobile,omitempty" yaml:"mobile,omitempty"`
	Desktop *configDesktop `json:"desktop,omitempty" yaml:"desktop,omitempty"`
	Expect  *RetryConfig   `json:"expect,omitempty" yaml:"expect,omitempty"`
}

type AllwrightConfig struct {
	SchemaVersion uint32                 `json:"schemaVersion,omitempty" yaml:"schemaVersion,omitempty"`
	Server        *configServer          `json:"server,omitempty" yaml:"server,omitempty"`
	Web           *configWeb             `json:"web,omitempty" yaml:"web,omitempty"`
	Mobile        *configMobile          `json:"mobile,omitempty" yaml:"mobile,omitempty"`
	Desktop       *configDesktop         `json:"desktop,omitempty" yaml:"desktop,omitempty"`
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
	Web            *configWeb
	Mobile         *configMobile
	Desktop        *configDesktop
}

type CommandOptions struct {
	Timeout time.Duration
}

type ScreenshotOptions struct {
	Timeout  time.Duration
	FullPage bool
	Path     string
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

type ScreenshotResult struct {
	PNGData []byte
	Note    string
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
	Send(*enginev1.SurfaceSessionCommand) error
	Recv() (*enginev1.SurfaceSessionEvent, error)
	CloseSend() error
}

type tabSessionStream interface {
	Send(*enginev1.ContextSessionCommand) error
	Recv() (*enginev1.ContextSessionEvent, error)
	CloseSend() error
}

type grpcClientConn interface {
	Close() error
}
