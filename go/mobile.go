package allwright

import (
	"context"
	"encoding/json"
	"fmt"
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

type AndroidPage struct {
	browserSession map[string]any
	pageSession    map[string]any
}

type AndroidDevice struct {
	connectInfo map[string]any
	page        *AndroidPage
}

type AndroidSurface struct{}

type mobileNamespace struct {
	Android AndroidSurface
}

var Mobile = mobileNamespace{
	Android: AndroidSurface{},
}

func (p *AndroidPage) SessionID() string {
	if p == nil {
		return ""
	}
	return stringValueFromMap(p.pageSession, "page_id")
}

func (p *AndroidPage) Click(ctx context.Context, selector string, options ...CommandOptions) (*ClickResult, error) {
	if p == nil {
		return nil, fmt.Errorf("android page is nil")
	}
	request, err := json.Marshal(map[string]any{
		"command":         "click_element",
		"browser_session": p.browserSession,
		"page_session":    p.pageSession,
		"selector":        selector,
		"timeout_ms":      commandOptionsTimeoutMs(firstCommandOptions(options)),
	})
	if err != nil {
		return nil, fmt.Errorf("encode mobile click request: %w", err)
	}
	output, err := invokePlugin(ctx, "mobile-android", string(request))
	if err != nil {
		return nil, err
	}
	result, err := decodeMobileResult(output, "click")
	if err != nil {
		return nil, err
	}
	return &ClickResult{
		Selector:      stringValueFromMap(result, "selector"),
		Note:          stringValueFromMap(result, "note"),
		BidiSessionID: stringValueFromMap(result, "session_id"),
	}, nil
}

func (p *AndroidPage) Fill(ctx context.Context, selector string, value string, options ...CommandOptions) (*FillResult, error) {
	if p == nil {
		return nil, fmt.Errorf("android page is nil")
	}
	request, err := json.Marshal(map[string]any{
		"command":         "fill_element",
		"browser_session": p.browserSession,
		"page_session":    p.pageSession,
		"selector":        selector,
		"value":           value,
		"timeout_ms":      commandOptionsTimeoutMs(firstCommandOptions(options)),
	})
	if err != nil {
		return nil, fmt.Errorf("encode mobile fill request: %w", err)
	}
	output, err := invokePlugin(ctx, "mobile-android", string(request))
	if err != nil {
		return nil, err
	}
	result, err := decodeMobileResult(output, "fill")
	if err != nil {
		return nil, err
	}
	return &FillResult{
		Selector: stringValueFromMap(result, "selector"),
		Value:    stringValueFromMap(result, "value"),
		Note:     stringValueFromMap(result, "note"),
	}, nil
}

func (d *AndroidDevice) SessionID() string {
	if d == nil {
		return ""
	}
	browserSession, _ := d.connectInfo["browser_session"].(map[string]any)
	automation, _ := browserSession["automation"].(map[string]any)
	return stringValueFromMap(automation, "session_id")
}

func (d *AndroidDevice) Page() *AndroidPage {
	if d == nil {
		return nil
	}
	return d.page
}

func (d *AndroidDevice) InitialPage() *AndroidPage {
	return d.Page()
}

func (d *AndroidDevice) Launch(ctx context.Context, options MobileAndroidLaunchOptions) (*AndroidPage, error) {
	if d == nil {
		return nil, fmt.Errorf("android device is nil")
	}
	request, err := json.Marshal(map[string]any{
		"command":         "launch_app",
		"browser_session": d.connectInfo["browser_session"],
		"options": map[string]any{
			"apk_path":           optionalJSONValue(options.APKPath),
			"app_id":             optionalJSONValue(options.AppID),
			"launch_activity":    optionalJSONValue(options.LaunchActivity),
			"stop_before_launch": options.StopBeforeLaunch,
			"timeout_ms":         optionalUint32JSONValue(options.Timeout),
		},
	})
	if err != nil {
		return nil, fmt.Errorf("encode mobile launch request: %w", err)
	}
	output, err := invokePlugin(ctx, "mobile-android", string(request))
	if err != nil {
		return nil, err
	}
	result, err := decodeMobileResult(output, "launch")
	if err != nil {
		return nil, err
	}
	d.page = newAndroidPage(
		mapValueFromMap(d.connectInfo, "browser_session"),
		mapValueFromMap(result, "page_session"),
	)
	return d.page, nil
}

func (AndroidSurface) Connect(ctx context.Context, options MobileAndroidConnectOptions) (*AndroidDevice, error) {
	if err := ensurePluginsInstalled([]string{"mobile-android"}); err != nil {
		return nil, err
	}
	request, err := json.Marshal(map[string]any{
		"command":            "connect",
		"platform":           "android",
		"device":             optionalJSONValue(options.Device),
		"adb_endpoint":       optionalJSONValue(options.AdbEndpoint),
		"preserve_app_state": options.PreserveAppState,
		"timeout_ms":         optionalUint32JSONValue(options.Timeout),
	})
	if err != nil {
		return nil, fmt.Errorf("encode mobile connect request: %w", err)
	}
	output, err := invokePlugin(ctx, "mobile-android", string(request))
	if err != nil {
		return nil, err
	}
	result, err := decodeMobileResult(output, "connect")
	if err != nil {
		return nil, err
	}
	return &AndroidDevice{
		connectInfo: result,
		page: newAndroidPage(
			mapValueFromMap(result, "browser_session"),
			mapValueFromMap(mapValueFromMap(result, "initial_page"), "page_session"),
		),
	}, nil
}

func newAndroidPage(browserSession map[string]any, pageSession map[string]any) *AndroidPage {
	return &AndroidPage{
		browserSession: browserSession,
		pageSession:    pageSession,
	}
}

func decodeMobileResult(payload []byte, commandName string) (map[string]any, error) {
	var envelope struct {
		OK     bool           `json:"ok"`
		Result map[string]any `json:"result"`
		Error  string         `json:"error"`
	}
	if err := json.Unmarshal(payload, &envelope); err != nil {
		return nil, fmt.Errorf("decode mobile plugin %s response: %w", commandName, err)
	}
	if !envelope.OK {
		if envelope.Error == "" {
			return nil, fmt.Errorf("mobile-android plugin %s failed", commandName)
		}
		return nil, fmt.Errorf("%s", envelope.Error)
	}
	if envelope.Result == nil {
		return nil, fmt.Errorf("mobile-android plugin %s returned no result", commandName)
	}
	return envelope.Result, nil
}

func commandOptionsTimeoutMs(options CommandOptions) any {
	if options.Timeout <= 0 {
		return nil
	}
	return uint32(options.Timeout.Milliseconds())
}

func optionalJSONValue(value string) any {
	if value == "" {
		return nil
	}
	return value
}

func optionalUint32JSONValue(value uint32) any {
	if value == 0 {
		return nil
	}
	return value
}

func mapValueFromMap(value map[string]any, key string) map[string]any {
	child, _ := value[key].(map[string]any)
	if child == nil {
		return map[string]any{}
	}
	return child
}

func stringValueFromMap(value map[string]any, key string) string {
	text, _ := value[key].(string)
	return text
}
