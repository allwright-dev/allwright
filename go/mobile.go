package allwright

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"
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

type AndroidPage struct {
	browserSession map[string]any
	pageSession    map[string]any
}

type AndroidLocator struct {
	page     *AndroidPage
	selector string
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

func (p *AndroidPage) Locator(selector string) *AndroidLocator {
	if p == nil {
		return nil
	}
	return &AndroidLocator{
		page:     p,
		selector: normalizeMobileSelectorForTransport(selector),
	}
}

func (p *AndroidPage) Click(ctx context.Context, selector string, options ...CommandOptions) (*ClickResult, error) {
	if p == nil {
		return nil, fmt.Errorf("android page is nil")
	}
	request, err := json.Marshal(map[string]any{
		"command":         "click_element",
		"browser_session": p.browserSession,
		"page_session":    p.pageSession,
		"selector":        normalizeMobileSelectorForTransport(selector),
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
		"selector":        normalizeMobileSelectorForTransport(selector),
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

func (l *AndroidLocator) Page() *AndroidPage {
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
