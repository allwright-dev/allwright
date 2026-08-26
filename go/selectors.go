package allwright

import (
	"encoding/json"
	"fmt"
	"strings"
)

type selectorFlavor string

const (
	selectorFlavorCSS   selectorFlavor = "css"
	selectorFlavorXPath selectorFlavor = "xpath"
)

func decodeSelectorBody(body string) string {
	candidate := strings.TrimSpace(body)
	if len(candidate) >= 2 && candidate[0] == '"' && candidate[len(candidate)-1] == '"' {
		var decoded string
		if err := json.Unmarshal([]byte(candidate), &decoded); err == nil {
			return decoded
		}
	}
	return candidate
}

func parseSelectorForTransport(selector string) (selectorFlavor, string) {
	trimmed := strings.TrimSpace(selector)
	lowered := strings.ToLower(trimmed)
	if strings.HasPrefix(lowered, "xpath=") || strings.HasPrefix(lowered, "xpath:") {
		return selectorFlavorXPath, decodeSelectorBody(trimmed[6:])
	}
	if strings.HasPrefix(lowered, "css=") || strings.HasPrefix(lowered, "css:") {
		return selectorFlavorCSS, decodeSelectorBody(trimmed[4:])
	}
	if strings.HasPrefix(trimmed, "//") ||
		strings.HasPrefix(trimmed, ".//") ||
		strings.HasPrefix(trimmed, "../") ||
		strings.HasPrefix(trimmed, "/") ||
		strings.HasPrefix(trimmed, "(") {
		return selectorFlavorXPath, trimmed
	}
	return selectorFlavorCSS, trimmed
}

func normalizeSelectorForTransport(selector string) string {
	flavor, body := parseSelectorForTransport(selector)
	encoded, err := json.Marshal(body)
	if err != nil {
		return fmt.Sprintf("%s=%q", flavor, body)
	}
	return fmt.Sprintf("%s=%s", flavor, encoded)
}

func chainSelectorForTransport(parent string, child string) string {
	parentSelector := ""
	childSelector := ""
	if strings.TrimSpace(parent) != "" {
		parentSelector = normalizeSelectorForTransport(parent)
	}
	if strings.TrimSpace(child) != "" {
		childSelector = normalizeSelectorForTransport(child)
	}
	if parentSelector == "" {
		return childSelector
	}
	if childSelector == "" {
		return parentSelector
	}
	return parentSelector + " " + childSelector
}
