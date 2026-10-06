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

type MobileIOSConnectOptions struct {
	Device           string
	AgentEndpoint    string
	PreserveAppState bool
	Timeout          uint32
}

type MobileIOSLaunchOptions struct {
	AppPath          string
	AppID            string
	StopBeforeLaunch bool
	Timeout          uint32
}

type mobileSelectorFlavor string

const (
	mobileSelectorFlavorCSS   mobileSelectorFlavor = "css"
	mobileSelectorFlavorXPath mobileSelectorFlavor = "xpath"
	mobileSelectorFlavorUIA   mobileSelectorFlavor = "uia"
	mobileSelectorFlavorAW    mobileSelectorFlavor = "aw"
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

type NativeApp struct {
	runtime          *runtimeClient
	stream           tabSessionStream
	surfaceSessionID string
	sessionID        string
	attached         bool
	closed           bool
}

type NativeLocator struct {
	page     *NativeApp
	selector string
}

type AndroidApp = NativeApp
type AndroidLocator = NativeLocator

type AndroidDevice struct {
	runtime          *runtimeClient
	stream           browserSessionStream
	sessionID        string
	surfaceSessionID string
	app              *NativeApp
	closed           bool
}

type AndroidSurface struct{}
type IOSSurface struct{}

type IOSDevice struct{ android *AndroidDevice }
type IOSApp = NativeApp
type IOSLocator = NativeLocator

type mobileNamespace struct {
	Android AndroidSurface
	IOS     IOSSurface
}

var Mobile = mobileNamespace{
	Android: AndroidSurface{},
	IOS:     IOSSurface{},
}

func (p *NativeApp) SessionID() string {
	if p == nil {
		return ""
	}
	return p.sessionID
}

func (p *NativeApp) Locator(selector string) *NativeLocator {
	if p == nil {
		return nil
	}
	return &NativeLocator{
		page:     p,
		selector: normalizeMobileSelectorForTransport(selector),
	}
}

func (p *NativeApp) GetByRole(role string, options ...RoleOptions) *NativeLocator {
	o := RoleOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return p.Locator(roleSelector(role, o))
}

func (p *NativeApp) GetByText(value any, options ...TextOptions) *NativeLocator {
	o := TextOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return p.Locator(semanticSelector(map[string]any{"kind": "text", "text": value, "exact": o.Exact}))
}

func (p *NativeApp) GetByLabel(value any, options ...TextOptions) *NativeLocator {
	o := TextOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return p.Locator(semanticSelector(map[string]any{"kind": "label", "text": value, "exact": o.Exact}))
}

func (p *NativeApp) GetByTestId(value any, _ ...TextOptions) *NativeLocator {
	return p.Locator(semanticSelector(map[string]any{"kind": "testId", "text": value}))
}

func (p *NativeApp) ensureStream(ctx context.Context) error {
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

func (p *NativeApp) Goto(ctx context.Context, url string, options ...CommandOptions) error {
	return p.Navigate(ctx, url, options...)
}

func (p *NativeApp) Navigate(ctx context.Context, url string, options ...CommandOptions) error {
	if p == nil {
		return fmt.Errorf("native app is nil")
	}
	if err := p.ensureStream(ctx); err != nil {
		return err
	}
	if p.closed {
		return fmt.Errorf("native app session %s is closed", p.sessionID)
	}
	commandOptions := firstCommandOptions(options)
	if err := p.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: p.surfaceSessionID,
		ContextSessionId: p.sessionID,
		Command: &enginev1.ContextSessionCommand_Navigate{
			Navigate: &enginev1.NavigatePageCommand{
				Url: url, RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return fmt.Errorf("send mobile NavigatePageCommand: %w", err)
	}
	for {
		event, err := p.stream.Recv()
		if err != nil {
			return fmt.Errorf("receive native app event during deep link: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			p.attached = true
		case *enginev1.ContextSessionEvent_Navigated:
			return nil
		case *enginev1.ContextSessionEvent_Closed:
			p.closed = true
			return fmt.Errorf("native app session %s closed while opening deep link", p.sessionID)
		case *enginev1.ContextSessionEvent_Error:
			return fmt.Errorf("native app deep-link error: %s", payload.Error.GetMessage())
		}
	}
}

func (p *NativeApp) Click(ctx context.Context, selector string, options ...CommandOptions) error {
	_, err := p.clickResult(ctx, selector, options...)
	return err
}

func (p *NativeApp) Count(ctx context.Context, selector string, options ...CommandOptions) (int, error) {
	result, err := p.countResult(ctx, selector, options...)
	if err != nil {
		return 0, err
	}
	return int(result.Count), nil
}

func (p *NativeApp) Focus(ctx context.Context, selector string, options ...CommandOptions) error {
	_, err := p.focusResult(ctx, selector, options...)
	return err
}

func (p *NativeApp) Fill(ctx context.Context, selector, value string, options ...CommandOptions) error {
	_, err := p.fillResult(ctx, selector, value, options...)
	return err
}

func (p *NativeApp) Press(ctx context.Context, selector, key string, options ...PressOptions) error {
	_, err := p.pressResult(ctx, selector, key, options...)
	return err
}

func (p *NativeApp) TextContent(ctx context.Context, selector string, options ...CommandOptions) (string, error) {
	result, err := p.textContentResult(ctx, selector, options...)
	if err != nil {
		return "", err
	}
	return result.Text, nil
}

func (p *NativeApp) InnerText(ctx context.Context, selector string, options ...CommandOptions) (string, error) {
	result, err := p.innerTextResult(ctx, selector, options...)
	if err != nil {
		return "", err
	}
	return result.Text, nil
}

func (p *NativeApp) WaitForSelector(ctx context.Context, selector string, options ...WaitForSelectorOptions) error {
	_, err := p.waitForSelectorResult(ctx, selector, options...)
	return err
}

func (p *NativeApp) Screenshot(ctx context.Context, options ...ScreenshotOptions) ([]byte, error) {
	result, err := p.screenshotResult(ctx, options...)
	if err != nil {
		return nil, err
	}
	return result.PNGData, nil
}

func (p *NativeApp) clickResult(ctx context.Context, selector string, options ...CommandOptions) (*ClickResult, error) {
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

func (p *NativeApp) fillResult(ctx context.Context, selector string, value string, options ...CommandOptions) (*FillResult, error) {
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

func (p *NativeApp) countResult(ctx context.Context, selector string, options ...CommandOptions) (*CountResult, error) {
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
		Command: &enginev1.ContextSessionCommand_CountElements{
			CountElements: &enginev1.CountElementsCommand{
				CssSelector:  selector,
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send Android CountElementsCommand: %w", err)
	}

	for {
		event, err := p.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive Android tab session event during count: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			p.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_ElementCounted:
			return &CountResult{
				Selector: payload.ElementCounted.GetCssSelector(),
				Count:    payload.ElementCounted.GetCount(),
				Note:     payload.ElementCounted.GetNote(),
			}, nil
		case *enginev1.ContextSessionEvent_Closed:
			p.closed = true
			return nil, fmt.Errorf("android app session %s closed while counting elements", p.sessionID)
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("android app session error while counting elements: %s", payload.Error.GetMessage())
		}
	}
}

func (p *NativeApp) focusResult(ctx context.Context, selector string, options ...CommandOptions) (*ElementResult, error) {
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
		Command: &enginev1.ContextSessionCommand_FocusElement{
			FocusElement: &enginev1.FocusElementCommand{
				CssSelector:  selector,
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send Android FocusElementCommand: %w", err)
	}

	for {
		event, err := p.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive Android tab session event during focus: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			p.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_ElementFocused:
			return &ElementResult{
				Selector: payload.ElementFocused.GetCssSelector(),
				Note:     payload.ElementFocused.GetNote(),
			}, nil
		case *enginev1.ContextSessionEvent_Closed:
			p.closed = true
			return nil, fmt.Errorf("android app session %s closed while focusing", p.sessionID)
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("android app session error while focusing: %s", payload.Error.GetMessage())
		}
	}
}

func (p *NativeApp) pressResult(ctx context.Context, selector string, key string, options ...PressOptions) (*PressResult, error) {
	if p == nil {
		return nil, fmt.Errorf("android app is nil")
	}
	if err := p.ensureStream(ctx); err != nil {
		return nil, err
	}
	if p.closed {
		return nil, fmt.Errorf("android tab session %s is closed", p.sessionID)
	}

	pressOptions := firstPressOptions(options)
	selector = normalizeMobileSelectorForTransport(selector)
	command := &enginev1.PressKeyCommand{
		CssSelector:  selector,
		Key:          key,
		RetryOptions: retryOptionsProto(pressOptions.Timeout),
	}
	if pressOptions.Text != "" {
		command.Text = optionalString(pressOptions.Text)
	}
	if err := p.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: p.surfaceSessionID,
		ContextSessionId: p.sessionID,
		Command: &enginev1.ContextSessionCommand_PressKey{
			PressKey: command,
		},
	}); err != nil {
		return nil, fmt.Errorf("send Android PressKeyCommand: %w", err)
	}

	for {
		event, err := p.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive Android tab session event during press: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			p.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_KeyPressed:
			return &PressResult{
				Selector: payload.KeyPressed.GetCssSelector(),
				Key:      payload.KeyPressed.GetKey(),
				Note:     payload.KeyPressed.GetNote(),
			}, nil
		case *enginev1.ContextSessionEvent_Closed:
			p.closed = true
			return nil, fmt.Errorf("android app session %s closed while pressing key", p.sessionID)
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("android app session error while pressing key: %s", payload.Error.GetMessage())
		}
	}
}

func (p *NativeApp) textContentResult(ctx context.Context, selector string, options ...CommandOptions) (*TextResult, error) {
	return p.readText(ctx, selector, true, options...)
}

func (p *NativeApp) innerTextResult(ctx context.Context, selector string, options ...CommandOptions) (*TextResult, error) {
	return p.readText(ctx, selector, false, options...)
}

func (p *NativeApp) waitForSelectorResult(ctx context.Context, selector string, options ...WaitForSelectorOptions) (*WaitForSelectorResult, error) {
	if p == nil {
		return nil, fmt.Errorf("android app is nil")
	}
	if err := p.ensureStream(ctx); err != nil {
		return nil, err
	}
	if p.closed {
		return nil, fmt.Errorf("android tab session %s is closed", p.sessionID)
	}

	waitOptions := firstWaitForSelectorOptions(options)
	selector = normalizeMobileSelectorForTransport(selector)
	command := &enginev1.WaitForSelectorCommand{
		CssSelector:  selector,
		RetryOptions: retryOptionsProto(waitOptions.Timeout),
	}
	if waitOptions.Visible != nil {
		command.Visible = waitOptions.Visible
	}
	if err := p.stream.Send(&enginev1.ContextSessionCommand{
		SurfaceSessionId: p.surfaceSessionID,
		ContextSessionId: p.sessionID,
		Command: &enginev1.ContextSessionCommand_WaitForSelector{
			WaitForSelector: command,
		},
	}); err != nil {
		return nil, fmt.Errorf("send Android WaitForSelectorCommand: %w", err)
	}

	for {
		event, err := p.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive Android tab session event during wait: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			p.attached = true
			_ = payload
		case *enginev1.ContextSessionEvent_SelectorWaitSatisfied:
			return &WaitForSelectorResult{
				Selector: payload.SelectorWaitSatisfied.GetCssSelector(),
				Visible:  payload.SelectorWaitSatisfied.GetVisible(),
				Note:     payload.SelectorWaitSatisfied.GetNote(),
			}, nil
		case *enginev1.ContextSessionEvent_Closed:
			p.closed = true
			return nil, fmt.Errorf("android app session %s closed while waiting for selector", p.sessionID)
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("android app session error while waiting for selector: %s", payload.Error.GetMessage())
		}
	}
}

func (p *NativeApp) screenshotResult(ctx context.Context, options ...ScreenshotOptions) (*ScreenshotResult, error) {
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

func (p *NativeApp) readText(ctx context.Context, selector string, textContent bool, options ...CommandOptions) (*TextResult, error) {
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
	command := &enginev1.ContextSessionCommand{
		SurfaceSessionId: p.surfaceSessionID,
		ContextSessionId: p.sessionID,
	}
	if textContent {
		command.Command = &enginev1.ContextSessionCommand_GetTextContent{
			GetTextContent: &enginev1.GetTextContentCommand{
				CssSelector:  selector,
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		}
	} else {
		command.Command = &enginev1.ContextSessionCommand_GetInnerText{
			GetInnerText: &enginev1.GetInnerTextCommand{
				CssSelector:  selector,
				RetryOptions: retryOptionsProto(commandOptions.Timeout),
			},
		}
	}
	if err := p.stream.Send(command); err != nil {
		return nil, fmt.Errorf("send Android text read command: %w", err)
	}

	for {
		event, err := p.stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive Android tab session event during text read: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.ContextSessionEvent_Attached:
			p.attached = true
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
		case *enginev1.ContextSessionEvent_Closed:
			p.closed = true
			return nil, fmt.Errorf("android app session %s closed while reading text", p.sessionID)
		case *enginev1.ContextSessionEvent_Error:
			return nil, fmt.Errorf("android app session error while reading text: %s", payload.Error.GetMessage())
		}
	}
}

func (d *AndroidDevice) SessionID() string {
	if d == nil {
		return ""
	}
	return d.sessionID
}

func (d *AndroidDevice) App() *NativeApp {
	if d == nil {
		return nil
	}
	return d.app
}

func (d *AndroidDevice) InitialApp() *NativeApp {
	return d.App()
}

func (d *AndroidDevice) Launch(ctx context.Context, options MobileAndroidLaunchOptions) (*NativeApp, error) {
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
			d.app = &NativeApp{
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
				app: &NativeApp{
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

func (IOSSurface) Connect(ctx context.Context, options MobileIOSConnectOptions) (*IOSDevice, error) {
	runtime, err := getRuntime(ctx)
	if err != nil {
		return nil, err
	}
	stream, err := runtime.engine.SurfaceSession(ctx)
	if err != nil {
		return nil, fmt.Errorf("open iOS surface session stream: %w", err)
	}
	if err := stream.Send(&enginev1.SurfaceSessionCommand{
		Command: &enginev1.SurfaceSessionCommand_ConnectMobile{
			ConnectMobile: &enginev1.ConnectMobileCommand{
				Platform:         enginev1.MobilePlatform_MOBILE_PLATFORM_IOS,
				Device:           optionalString(options.Device),
				AdbEndpoint:      optionalString(options.AgentEndpoint),
				PreserveAppState: options.PreserveAppState,
				RetryOptions:     retryOptionsProto(durationFromOptionalUint32(options.Timeout)),
			},
		},
	}); err != nil {
		return nil, fmt.Errorf("send iOS ConnectMobileCommand: %w", err)
	}
	for {
		event, err := stream.Recv()
		if err != nil {
			return nil, fmt.Errorf("receive iOS connect event: %w", err)
		}
		switch payload := event.GetEvent().(type) {
		case *enginev1.SurfaceSessionEvent_MobileConnected:
			sessionID := payload.MobileConnected.GetDeviceSessionId()
			if sessionID == "" {
				sessionID = event.GetSessionId()
			}
			return &IOSDevice{android: &AndroidDevice{
				runtime: runtime, stream: stream, sessionID: sessionID,
				surfaceSessionID: event.GetSessionId(),
				app:              &NativeApp{runtime: runtime, surfaceSessionID: event.GetSessionId(), sessionID: payload.MobileConnected.GetInitialAppSessionId()},
			}}, nil
		case *enginev1.SurfaceSessionEvent_Error:
			return nil, fmt.Errorf("iOS device session error: %s", payload.Error.GetMessage())
		}
	}
}

func (d *IOSDevice) SessionID() string {
	if d == nil {
		return ""
	}
	return d.android.SessionID()
}
func (d *IOSDevice) App() *IOSApp {
	if d == nil {
		return nil
	}
	return d.android.App()
}
func (d *IOSDevice) InitialApp() *IOSApp { return d.App() }
func (d *IOSDevice) Launch(ctx context.Context, options MobileIOSLaunchOptions) (*IOSApp, error) {
	if d == nil {
		return nil, fmt.Errorf("ios device is nil")
	}
	return d.android.Launch(ctx, MobileAndroidLaunchOptions{
		APKPath: options.AppPath, AppID: options.AppID, StopBeforeLaunch: options.StopBeforeLaunch, Timeout: options.Timeout,
	})
}

func (l *NativeLocator) App() *NativeApp {
	if l == nil {
		return nil
	}
	return l.page
}

func (l *NativeLocator) Selector() string {
	if l == nil {
		return ""
	}
	return l.selector
}

func (l *NativeLocator) Locator(selector string) *NativeLocator {
	if l == nil {
		return nil
	}
	return &NativeLocator{
		page:     l.page,
		selector: chainMobileSelectorForTransport(l.selector, selector),
	}
}

func (l *NativeLocator) GetByRole(role string, options ...RoleOptions) *NativeLocator {
	o := RoleOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(roleSelector(role, o))
}

func (l *NativeLocator) GetByText(value any, options ...TextOptions) *NativeLocator {
	o := TextOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(semanticSelector(map[string]any{"kind": "text", "text": value, "exact": o.Exact}))
}

func (l *NativeLocator) GetByLabel(value any, options ...TextOptions) *NativeLocator {
	o := TextOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(semanticSelector(map[string]any{"kind": "label", "text": value, "exact": o.Exact}))
}

func (l *NativeLocator) GetByTestId(value any, _ ...TextOptions) *NativeLocator {
	return l.Locator(semanticSelector(map[string]any{"kind": "testId", "text": value}))
}

func (l *NativeLocator) Click(ctx context.Context, options ...CommandOptions) error {
	if l == nil || l.page == nil {
		return fmt.Errorf("android locator page is nil")
	}
	return l.page.Click(ctx, l.selector, options...)
}

func (l *NativeLocator) Count(ctx context.Context, options ...CommandOptions) (int, error) {
	if l == nil || l.page == nil {
		return 0, fmt.Errorf("android locator page is nil")
	}
	return l.page.Count(ctx, l.selector, options...)
}

func (l *NativeLocator) Focus(ctx context.Context, options ...CommandOptions) error {
	if l == nil || l.page == nil {
		return fmt.Errorf("android locator page is nil")
	}
	return l.page.Focus(ctx, l.selector, options...)
}

func (l *NativeLocator) Fill(ctx context.Context, value string, options ...CommandOptions) error {
	if l == nil || l.page == nil {
		return fmt.Errorf("android locator page is nil")
	}
	return l.page.Fill(ctx, l.selector, value, options...)
}

func (l *NativeLocator) Press(ctx context.Context, key string, options ...PressOptions) error {
	if l == nil || l.page == nil {
		return fmt.Errorf("android locator page is nil")
	}
	return l.page.Press(ctx, l.selector, key, options...)
}

func (l *NativeLocator) TextContent(ctx context.Context, options ...CommandOptions) (string, error) {
	if l == nil || l.page == nil {
		return "", fmt.Errorf("android locator page is nil")
	}
	return l.page.TextContent(ctx, l.selector, options...)
}

func (l *NativeLocator) InnerText(ctx context.Context, options ...CommandOptions) (string, error) {
	if l == nil || l.page == nil {
		return "", fmt.Errorf("android locator page is nil")
	}
	return l.page.InnerText(ctx, l.selector, options...)
}

func (l *NativeLocator) WaitFor(ctx context.Context, options ...WaitForSelectorOptions) error {
	if l == nil || l.page == nil {
		return fmt.Errorf("android locator page is nil")
	}
	return l.page.WaitForSelector(ctx, l.selector, options...)
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
	if strings.HasPrefix(lowered, "aw=") || strings.HasPrefix(lowered, "aw:") {
		return mobileSelectorFlavorAW, 3, true
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
	lowered := strings.ToLower(trimmed)
	if strings.HasPrefix(lowered, "ref=") || strings.HasPrefix(lowered, "ref:") {
		return "ref=" + strings.Trim(strings.TrimSpace(trimmed[4:]), "\"")
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
