package allwright

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"time"

	enginev1 "allwright.dev/gen/allwright/engine/v1"
)

type MobileAndroidConnectOptions struct {
	Device           string
	AdbEndpoint      string
	PreserveAppState bool
	Timeout          uint32
}

type MobileAndroidLaunchOptions struct {
	APKPath          string
	AppID            string
	LaunchActivity   string
	StopBeforeLaunch bool
	Timeout          uint32
}

type mobileSelectorFlavor string

const (
	mobileSelectorFlavorCSS   mobileSelectorFlavor = "css"
	mobileSelectorFlavorXPath mobileSelectorFlavor = "xpath"
	mobileSelectorFlavorUIA   mobileSelectorFlavor = "uia"
)

var uiAutomatorSelectorKeys = map[string]struct{}{
	"text": {}, "textcontains": {}, "textmatches": {}, "textstartswith": {},
	"classname": {}, "classnamematches": {},
	"description": {}, "desc": {}, "descriptioncontains": {}, "desccontains": {},
	"descriptionmatches": {}, "descmatches": {}, "descriptionstartswith": {}, "descstartswith": {},
	"checkable": {}, "checked": {}, "clickable": {}, "longclickable": {}, "scrollable": {},
	"enabled": {}, "focusable": {}, "focused": {}, "selected": {},
	"packagename": {}, "package": {}, "packagenamematches": {},
	"resourceid": {}, "resourceidmatches": {},
	"index": {}, "instance": {},
}

type AndroidApp struct {
	runtime          *runtimeClient
	stream           tabSessionStream
	surfaceSessionID string
	sessionID        string
	attached         bool
	closed           bool
}

type AndroidLocator struct {
	page     *AndroidApp
	selector string
}

type AndroidDevice struct {
	runtime          *runtimeClient
	stream           browserSessionStream
	sessionID        string
	surfaceSessionID string
	app              *AndroidApp
	closed           bool
}

type AndroidSurface struct{}

type mobileNamespace struct {
	Android AndroidSurface
}

var Mobile = mobileNamespace{
	Android: AndroidSurface{},
}

func (p *AndroidApp) SessionID() string {
	if p == nil {
		return ""
	}
	return p.sessionID
}

func (p *AndroidApp) Locator(selector string) *AndroidLocator {
	if p == nil {
		return nil
	}
	return &AndroidLocator{
		page:     p,
		selector: normalizeMobileSelectorForTransport(selector),
	}
}

func (p *AndroidApp) ensureStream(ctx context.Context) error {
	if p.stream != nil {
		return nil
	}
	if p.runtime == nil {
		return fmt.Errorf("android app runtime is nil")
	}
	stream, err := p.runtime.engine.ContextSession(ctx)
	if err != nil {
		return fmt.Errorf("open Android app session stream: %w", err)
	}
	p.stream = stream
	return nil
}

func (p *AndroidApp) Click(ctx context.Context, selector string, options ...CommandOptions) (*ClickResult, error) {
	if p == nil {
		return nil, fmt.Errorf("android app is nil")
	}
	if err := p.ensureStream(ctx); err != nil {
		return nil, err
	}
	if p.closed {
		return nil, fmt.Errorf("android tab session %s is closed", p.sessionID)
	}

	commandOptions := firstCommandOptions(options)
	selector = normalizeMobileSelectorForTransport(selector)
	if err := p.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: p.surfaceSessionID,
		ContextSessionId: p.sessionID,
		Command: &enginev1.ContextSessionCommand_ClickElement{
			ClickElement: &enginev1.ClickElementCommand{
				CssSelector:  selector,
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send Android ClickElementCommand: %w", err)
	}

	for {
		event, err := p.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive Android tab session event during click: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			p.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_ElementClicked:
			return &ClickResult{
				Selector:      payload.ElementClicked.GetCssSelector(),
				Note:          payload.ElementClicked.GetNote(),
				BidiSessionID: payload.ElementClicked.GetBidiSessionId(),
			}, nil
		case *enginev1.ContextSessionEvent_Closed:
			p.closed = true
			return nil, fmt.Errorf("android app session %s closed while clicking", p.sessionID)
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("android app session error while clicking: %s", payload.Error.GetMessage())
		}
	}
}

func (p *AndroidApp) Fill(ctx context.Context, selector string, value string, options ...CommandOptions) (*FillResult, error) {
	if p == nil {
		return nil, fmt.Errorf("android app is nil")
	}
	if err := p.ensureStream(ctx); err != nil {
		return nil, err
	}
	if p.closed {
		return nil, fmt.Errorf("android tab session %s is closed", p.sessionID)
	}

	commandOptions := firstCommandOptions(options)
	selector = normalizeMobileSelectorForTransport(selector)
	if err := p.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: p.surfaceSessionID,
		ContextSessionId: p.sessionID,
		Command: &enginev1.ContextSessionCommand_FillElement{
			FillElement: &enginev1.FillElementCommand{
				CssSelector:  selector,
				Value:        value,
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send Android FillElementCommand: %w", err)
	}

	for {
		event, err := p.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive Android tab session event during fill: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			p.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_ElementFilled:
			return &FillResult{
				Selector: payload.ElementFilled.GetCssSelector(),
				Value:    payload.ElementFilled.GetValue(),
				Note:     payload.ElementFilled.GetNote(),
			}, nil
		case *enginev1.ContextSessionEvent_Closed:
			p.closed = true
			return nil, fmt.Errorf("android app session %s closed while filling", p.sessionID)
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("android app session error while filling: %s", payload.Error.GetMessage())
		}
	}
}

func (p *AndroidApp) Screenshot(ctx context.Context, options ...ScreenshotOptions) (*ScreenshotResult, error) {
	if p == nil {
		return nil, fmt.Errorf("android app is nil")
	}
	if err := p.ensureStream(ctx); err != nil {
		return nil, err
	}
	if p.closed {
		return nil, fmt.Errorf("android tab session %s is closed", p.sessionID)
	}

	screenshotOptions := firstScreenshotOptions(options)
	if err := p.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: p.surfaceSessionID,
		ContextSessionId: p.sessionID,
		Command: &enginev1.ContextSessionCommand_Screenshot{
			Screenshot: &enginev1.ScreenshotCommand{
				RetryOptions: retryOptionsProto(screenshotOptions.Timeout),
				FullPage:     optionalBool(screenshotOptions.FullPage),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send Android ScreenshotCommand: %w", err)
	}

	for {
		event, err := p.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive Android tab session event during screenshot: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			p.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_ScreenshotCaptured:
			screenshot := &ScreenshotResult{
				PNGData: payload.ScreenshotCaptured.GetPngData(),
				Note:    payload.ScreenshotCaptured.GetNote(),
			}
			if screenshotOptions.Path != "" {
				if err := os.WriteFile(screenshotOptions.Path, screenshot.PNGData, 0o644); err != nil {
					return nil, fmt.Errorf("write screenshot to %q: %w", screenshotOptions.Path, err)
				}
			}
			return screenshot, nil
		case *enginev1.ContextSessionEvent_Closed:
			p.closed = true
			return nil, fmt.Errorf("android app session %s closed while capturing screenshot", p.sessionID)
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("android app session error while capturing screenshot: %s", payload.Error.GetMessage())
		}
	}
}

func (d *AndroidDevice) SessionID() string {
	if d == nil {
		return ""
	}
	return d.sessionID
}

func (d *AndroidDevice) App() *AndroidApp {
	if d == nil {
		return nil
	}
	return d.app
}

func (d *AndroidDevice) InitialApp() *AndroidApp {
	return d.App()
}

func (d *AndroidDevice) Launch(ctx context.Context, options MobileAndroidLaunchOptions) (*AndroidApp, error) {
	if d == nil {
		return nil, fmt.Errorf("android device is nil")
	}
	if d.closed {
		return nil, fmt.Errorf("android device session %s is closed", d.sessionID)
	}

	if err := d.stream.Send(&enginev1.SurfaceSessionCommand{
		Command: &enginev1.SurfaceSessionCommand_LaunchApp{
			LaunchApp: &enginev1.LaunchAppCommand{
				ApkPath:          optionalString(options.APKPath),
				AppId:            optionalString(options.AppID),
				LaunchActivity:   optionalString(options.LaunchActivity),
				StopBeforeLaunch: options.StopBeforeLaunch,
				RetryOptions:     retryOptionsProto(durationFromOptionalUint32(options.Timeout)),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send LaunchAppCommand: %w", err)
	}

	for {
		event, err := d.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive surface session event while launching Android app: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.SurfaceSessionEvent_AppLaunched:
			d.app = &AndroidApp{
				runtime:          d.runtime,
				surfaceSessionID: d.surfaceSessionID,
				sessionID:        payload.AppLaunched.GetAppSessionId(),
			}
			return d.app, nil
		case *enginev1.SurfaceSessionEvent_Closed:
			d.closed = true
			return nil, fmt.Errorf("android device session %s closed while launching app", d.sessionID)
		case *enginev1.SurfaceSessionEvent_Error:
			return nil, fmt.Errorf("android device session error while launching app: %s", payload.Error.GetMessage())
		}
	}
}

func (AndroidSurface) Connect(ctx context.Context, options MobileAndroidConnectOptions) (*AndroidDevice, error) {
	runtime, err := getRuntime(ctx)
	if err != nil {
		return nil, err
	}

	stream, err := runtime.engine.SurfaceSession(ctx)
	if err != nil {
		return nil, fmt.Errorf("open Android surface session stream: %w", err)
	}

	if err := stream.Send(&enginev1.SurfaceSessionCommand{
		Command: &enginev1.SurfaceSessionCommand_ConnectMobile{
			ConnectMobile: &enginev1.ConnectMobileCommand{
				Platform:         enginev1.MobilePlatform_MOBILE_PLATFORM_ANDROID,
				Device:           optionalString(options.Device),
				AdbEndpoint:      optionalString(options.AdbEndpoint),
				PreserveAppState: options.PreserveAppState,
				RetryOptions:     retryOptionsProto(durationFromOptionalUint32(options.Timeout)),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send ConnectMobileCommand: %w", err)
	}

	for {
		event, err := stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive browser session event during Android connect: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.SurfaceSessionEvent_MobileConnected:
			sessionID := payload.MobileConnected.GetDeviceSessionId()
			if sessionID == "" {
				sessionID = event.GetSessionId()
			}
			return &AndroidDevice{
				runtime:          runtime,
				stream:           stream,
				sessionID:        sessionID,
				surfaceSessionID: event.GetSessionId(),
				app: &AndroidApp{
					runtime:          runtime,
					surfaceSessionID: event.GetSessionId(),
					sessionID:        payload.MobileConnected.GetInitialAppSessionId(),
				},
			}, nil
		case *enginev1.SurfaceSessionEvent_Error:
			return nil, fmt.Errorf("device session error during Android connect: %s", payload.Error.GetMessage())
		}
	}
}

func (l *AndroidLocator) App() *AndroidApp {
	if l == nil {
		return nil
	}
	return l.page
}

func (l *AndroidLocator) Selector() string {
	if l == nil {
		return ""
	}
	return l.selector
}

func (l *AndroidLocator) Locator(selector string) *AndroidLocator {
	if l == nil {
		return nil
	}
	return &AndroidLocator{
		page:     l.page,
		selector: chainMobileSelectorForTransport(l.selector, selector),
	}
}

func (l *AndroidLocator) Click(ctx context.Context, options ...CommandOptions) (*ClickResult, error) {
	if l == nil || l.page == nil {
		return nil, fmt.Errorf("android locator page is nil")
	}
	return l.page.Click(ctx, l.selector, options...)
}

func (l *AndroidLocator) Fill(ctx context.Context, value string, options ...CommandOptions) (*FillResult, error) {
	if l == nil || l.page == nil {
		return nil, fmt.Errorf("android locator page is nil")
	}
	return l.page.Fill(ctx, l.selector, value, options...)
}

func parseExplicitMobileSelectorPrefix(selector string) (mobileSelectorFlavor, int, bool) {
	lowered := strings.ToLower(selector)
	if strings.HasPrefix(lowered, "xpath=") || strings.HasPrefix(lowered, "xpath:") {
		return mobileSelectorFlavorXPath, 6, true
	}
	if strings.HasPrefix(lowered, "css=") || strings.HasPrefix(lowered, "css:") {
		return mobileSelectorFlavorCSS, 4, true
	}
	if strings.HasPrefix(lowered, "uia=") || strings.HasPrefix(lowered, "uia:") {
		return mobileSelectorFlavorUIA, 4, true
	}
	return "", 0, false
}

func parseUiAutomatorSelectorPrefix(selector string) int {
	for index, char := range selector {
		if char != '=' && char != ':' {
			continue
		}
		key := strings.ToLower(strings.TrimSpace(selector[:index]))
		if _, ok := uiAutomatorSelectorKeys[key]; ok {
			return index + 1
		}
		return -1
	}
	return -1
}

func isNormalizedMobileTransportSelector(selector string) bool {
	trimmed := strings.TrimSpace(selector)
	if trimmed == "" {
		return false
	}

	for index := 0; index < len(trimmed); {
		_, prefixLen, ok := parseExplicitMobileSelectorPrefix(trimmed[index:])
		if !ok {
			return false
		}
		index += prefixLen

		remainder := trimmed[index:]
		jsonEnd := findJSONStringEnd(remainder)
		if jsonEnd < 0 {
			return false
		}
		index += jsonEnd
		if index == len(trimmed) {
			return true
		}

		whitespaceStart := index
		for index < len(trimmed) && (trimmed[index] == ' ' || trimmed[index] == '\t' || trimmed[index] == '\n' || trimmed[index] == '\r') {
			index++
		}
		if index == whitespaceStart {
			return false
		}
		if _, _, ok := parseExplicitMobileSelectorPrefix(trimmed[index:]); !ok {
			return false
		}
	}

	return true
}

func parseMobileSelectorForTransport(selector string) (mobileSelectorFlavor, string) {
	trimmed := strings.TrimSpace(selector)
	if flavor, prefixLen, ok := parseExplicitMobileSelectorPrefix(trimmed); ok {
		return flavor, decodeSelectorBody(trimmed[prefixLen:])
	}
	if prefixLen := parseUiAutomatorSelectorPrefix(trimmed); prefixLen > 0 {
		return mobileSelectorFlavorUIA, trimmed[:prefixLen-1] + "=" + trimmed[prefixLen:]
	}
	if strings.HasPrefix(trimmed, "//") ||
		strings.HasPrefix(trimmed, ".//") ||
		strings.HasPrefix(trimmed, "../") ||
		strings.HasPrefix(trimmed, "/") ||
		strings.HasPrefix(trimmed, "(") {
		return mobileSelectorFlavorXPath, trimmed
	}
	return mobileSelectorFlavorCSS, trimmed
}

func normalizeMobileSelectorForTransport(selector string) string {
	trimmed := strings.TrimSpace(selector)
	if trimmed == "" {
		return ""
	}
	if isNormalizedMobileTransportSelector(trimmed) {
		return trimmed
	}
	flavor, body := parseMobileSelectorForTransport(selector)
	encoded, err := json.Marshal(body)
	if err != nil {
		return fmt.Sprintf("%s=%q", flavor, body)
	}
	return fmt.Sprintf("%s=%s", flavor, encoded)
}

func chainMobileSelectorForTransport(parent string, child string) string {
	parentSelector := ""
	childSelector := ""
	if strings.TrimSpace(parent) != "" {
		parentSelector = normalizeMobileSelectorForTransport(parent)
	}
	if strings.TrimSpace(child) != "" {
		childSelector = normalizeMobileSelectorForTransport(child)
	}
	if parentSelector == "" {
		return childSelector
	}
	if childSelector == "" {
		return parentSelector
	}
	return parentSelector + " " + childSelector
}

func durationFromOptionalUint32(value uint32) time.Duration {
	if value == 0 {
		return 0
	}
	return time.Duration(value) * time.Millisecond
}
