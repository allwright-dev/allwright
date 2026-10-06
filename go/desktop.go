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

type MacApp = NativeApp
type MacLocator = NativeLocator
type WindowsApp = NativeApp
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
			d.app = &MacApp{runtime: d.runtime, surfaceSessionID: d.surfaceSessionID, sessionID: payload.DesktopAppLaunched.GetAppSessionId()}
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
				app: &MacApp{runtime: runtime, surfaceSessionID: event.GetSessionId(), sessionID: payload.DesktopConnected.GetInitialAppSessionId()},
			}, nil
		case *enginev1.SurfaceSessionEvent_Error:
			return nil, fmt.Errorf("%s desktop connect error: %s", label, payload.Error.GetMessage())
		}
	}
}
