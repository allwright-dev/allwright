package allwright

import (
	"context"
	"fmt"
	"strings"

	enginev1 "allwright.dev/gen/allwright/engine/v1"
)

var (
	Chromium = BrowserType{browserKind: enginev1.BrowserKind_BROWSER_KIND_CHROMIUM}
	Firefox  = BrowserType{browserKind: enginev1.BrowserKind_BROWSER_KIND_FIREFOX}
)

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

	stream, err := runtime.engine.SurfaceSession(ctx)
	if err != nil {
		return nil, fmt.Errorf("open browser session stream: %w", err)
	}

	command := &enginev1.SurfaceSessionCommand{
		Command: &enginev1.SurfaceSessionCommand_LaunchBrowser{
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
		case *enginev1.SurfaceSessionEvent_BrowserLaunched:
			return newBrowserFromLaunch(
				runtime,
				stream,
				event.GetSessionId(),
				payload.BrowserLaunched.GetBrowser(),
				payload.BrowserLaunched.GetNote(),
				"",
				payload.BrowserLaunched.GetUserDataDir(),
				payload.BrowserLaunched.GetInitialPageSessionId(),
			), nil
		case *enginev1.SurfaceSessionEvent_ChromeLaunched:
			return newBrowserFromLaunch(
				runtime,
				stream,
				event.GetSessionId(),
				payload.ChromeLaunched.GetBrowser(),
				payload.ChromeLaunched.GetNote(),
				payload.ChromeLaunched.GetCdpWebsocketUrl(),
				payload.ChromeLaunched.GetUserDataDir(),
				payload.ChromeLaunched.GetInitialPageSessionId(),
			), nil
		case *enginev1.SurfaceSessionEvent_Error:
			return nil, fmt.Errorf("browser session error during launch: %s", payload.Error.GetMessage())
		}
	}
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

	if strings.TrimSpace(config.BrowserName) == "" && (config.Mobile != nil || config.Desktop != nil) {
		return nil, fmt.Errorf("resolved config does not define web.browser.name and includes only non-web surfaces")
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
