package allwright

import (
	"context"
	"fmt"

	enginev1 "allwright.dev/gen/allwright/engine/v1"
)

type DesktopMacConnectOptions struct {
	AgentEndpoint string
	Timeout       uint32
}

type DesktopMacLaunchOptions struct {
	AppID            string
	TerminateRunning bool
	Timeout          uint32
}

type DesktopWindowsConnectOptions = DesktopMacConnectOptions
type DesktopWindowsLaunchOptions = DesktopMacLaunchOptions

type DesktopApp struct{ *NativeApp }

type DesktopWindow struct {
	*NativeApp
	windowID string
	title    string
	main     bool
}

type MacApp = DesktopApp
type MacLocator = NativeLocator
type WindowsApp = DesktopApp
type WindowsLocator = NativeLocator
type WindowsDesktop = MacDesktop

type MacDesktop struct {
	runtime          *runtimeClient
	stream           browserSessionStream
	sessionID        string
	surfaceSessionID string
	app              *MacApp
	closed           bool
}

type MacSurface struct{}
type WindowsSurface struct{}

type desktopNamespace struct {
	Mac     MacSurface
	Windows WindowsSurface
}

var Desktop = desktopNamespace{Mac: MacSurface{}, Windows: WindowsSurface{}}

func (d *MacDesktop) SessionID() string {
	if d == nil {
		return ""
	}
	return d.sessionID
}

func (d *MacDesktop) App() *MacApp {
	if d == nil {
		return nil
	}
	return d.app
}

func (d *MacDesktop) InitialApp() *MacApp { return d.App() }

func (d *MacDesktop) Launch(ctx context.Context, options DesktopMacLaunchOptions) (*MacApp, error) {
	if d == nil {
		return nil, fmt.Errorf("mac desktop is nil")
	}
	if d.closed {
		return nil, fmt.Errorf("mac desktop session %s is closed", d.sessionID)
	}
	if options.AppID == "" {
		return nil, fmt.Errorf("desktop mac launch requires AppID")
	}
	if err := d.stream.Send(&enginev1.SurfaceSessionCommand{
		Command: &enginev1.SurfaceSessionCommand_LaunchDesktopApp{
			LaunchDesktopApp: &enginev1.LaunchDesktopAppCommand{
				AppId:            options.AppID,
				TerminateRunning: options.TerminateRunning,
				RetryOptions:     retryOptionsProto(durationFromOptionalUint32(options.Timeout)),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send LaunchDesktopAppCommand: %w", err)
	}

	for {
		event, err := d.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive surface event while launching mac app: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.SurfaceSessionEvent_DesktopAppLaunched:
			d.app = newDesktopApp(d.runtime, d.surfaceSessionID, payload.DesktopAppLaunched.GetAppSessionId())
			return d.app, nil
		case *enginev1.SurfaceSessionEvent_Closed:
			d.closed = true
			return nil, fmt.Errorf("mac desktop session %s closed while launching app", d.sessionID)
		case *enginev1.SurfaceSessionEvent_Error:
			return nil, fmt.Errorf("mac desktop launch error: %s", payload.Error.GetMessage())
		}
	}
}

func (MacSurface) Connect(ctx context.Context, options DesktopMacConnectOptions) (*MacDesktop, error) {
	return connectDesktop(ctx, options, enginev1.DesktopPlatform_DESKTOP_PLATFORM_MAC, "mac")
}

func (WindowsSurface) Connect(ctx context.Context, options DesktopWindowsConnectOptions) (*WindowsDesktop, error) {
	return connectDesktop(ctx, options, enginev1.DesktopPlatform_DESKTOP_PLATFORM_WINDOWS, "Windows")
}

func connectDesktop(ctx context.Context, options DesktopMacConnectOptions, platform enginev1.DesktopPlatform, label string) (*MacDesktop, error) {
	runtime, err := getRuntime(ctx)
	if err != nil {
		return nil, err
	}
	stream, err := runtime.engine.SurfaceSession(ctx)
	if err != nil {
		return nil, fmt.Errorf("open %s desktop surface stream: %w", label, err)
	}
	if err := stream.Send(&enginev1.SurfaceSessionCommand{
		Command: &enginev1.SurfaceSessionCommand_ConnectDesktop{
			ConnectDesktop: &enginev1.ConnectDesktopCommand{
				Platform:      platform,
				AgentEndpoint: optionalString(options.AgentEndpoint),
				RetryOptions:  retryOptionsProto(durationFromOptionalUint32(options.Timeout)),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send ConnectDesktopCommand: %w", err)
	}

	for {
		event, err := stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive %s desktop connect event: %w", label, err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.SurfaceSessionEvent_DesktopConnected:
			sessionID := payload.DesktopConnected.GetDesktopSessionId()
			if sessionID == "" {
				sessionID = event.GetSessionId()
			}
			return &MacDesktop{
				runtime: runtime, stream: stream, sessionID: sessionID, surfaceSessionID: event.GetSessionId(),
				app: newDesktopApp(runtime, event.GetSessionId(), payload.DesktopConnected.GetInitialAppSessionId()),
			}, nil
		case *enginev1.SurfaceSessionEvent_Error:
			return nil, fmt.Errorf("%s desktop connect error: %s", label, payload.Error.GetMessage())
		}
	}
}

func newDesktopApp(runtime *runtimeClient, surfaceSessionID, sessionID string) *DesktopApp {
	return &DesktopApp{NativeApp: &NativeApp{runtime: runtime, surfaceSessionID: surfaceSessionID, sessionID: sessionID}}
}

func (app *DesktopApp) Windows(ctx context.Context) ([]*DesktopWindow, error) {
	if app == nil || app.NativeApp == nil {
		return nil, fmt.Errorf("desktop app is nil")
	}
	if err := app.ensureStream(ctx); err != nil {
		return nil, err
	}
	if err := app.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: app.surfaceSessionID, ContextSessionId: app.sessionID,
		Command: &enginev1.ContextSessionCommand_ListDesktopWindows{ListDesktopWindows: &enginev1.ListDesktopWindowsCommand{}},
	}); err != nil {
		return nil, fmt.Errorf("send ListDesktopWindowsCommand: %w", err)
	}
	for {
		event, err := app.stream.Recv()
		if err != nil {
			return nil, err
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_DesktopWindowsResolved:
			windows := make([]*DesktopWindow, 0, len(payload.DesktopWindowsResolved.GetWindows()))
			for _, info := range payload.DesktopWindowsResolved.GetWindows() {
				window, err := app.windowFromInfo(info)
				if err != nil {
					return nil, err
				}
				windows = append(windows, window)
			}
			return windows, nil
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("list desktop windows: %s", payload.Error.GetMessage())
		}
	}
}

func (app *DesktopApp) MainWindow(ctx context.Context) (*DesktopWindow, error) {
	windows, err := app.Windows(ctx)
	if err != nil {
		return nil, err
	}
	for _, window := range windows {
		if window.main {
			return window, nil
		}
	}
	return nil, fmt.Errorf("desktop application does not currently expose a main window")
}

func (window *DesktopWindow) WindowID() string {
	if window == nil {
		return ""
	}
	return window.windowID
}
func (window *DesktopWindow) Title() string {
	if window == nil {
		return ""
	}
	return window.title
}
func (window *DesktopWindow) IsMain() bool { return window != nil && window.main }

func (window *DesktopWindow) Close(ctx context.Context) error {
	if window == nil || window.NativeApp == nil {
		return fmt.Errorf("desktop window is nil")
	}
	if err := window.ensureStream(ctx); err != nil {
		return err
	}
	if err := window.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: window.surfaceSessionID, ContextSessionId: window.sessionID,
		Command: &enginev1.ContextSessionCommand_CloseDesktopWindow{CloseDesktopWindow: &enginev1.CloseDesktopWindowCommand{}},
	}); err != nil {
		return err
	}
	for {
		event, err := window.stream.Recv()
		if err != nil {
			return err
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_DesktopWindowClosed:
			if payload.DesktopWindowClosed.GetWindowId() == window.windowID {
				window.closed = true
				return nil
			}
		case *enginev1.ContextSessionEvent_Error:
			return fmt.Errorf("close desktop window: %s", payload.Error.GetMessage())
		}
	}
}

func (app *DesktopApp) windowFromInfo(info *enginev1.DesktopWindowInfo) (*DesktopWindow, error) {
	if info == nil || info.GetContextSessionId() == "" || info.GetWindowId() == "" {
		return nil, fmt.Errorf("desktop window result is missing its context or window id")
	}
	return &DesktopWindow{NativeApp: &NativeApp{runtime: app.runtime, surfaceSessionID: app.surfaceSessionID,
		sessionID: info.GetContextSessionId()}, windowID: info.GetWindowId(), title: info.GetTitle(), main: info.GetIsMain()}, nil
}
