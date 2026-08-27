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

func parseExplicitSelectorPrefix(selector string) (selectorFlavor, int, bool) {
	lowered := strings.ToLower(selector)
	if strings.HasPrefix(lowered, "xpath=") || strings.HasPrefix(lowered, "xpath:") {
		return selectorFlavorXPath, 6, true
	}
	if strings.HasPrefix(lowered, "css=") || strings.HasPrefix(lowered, "css:") {
		return selectorFlavorCSS, 4, true
	}
	return "", 0, false
}

func findJSONStringEnd(value string) int {
	if !strings.HasPrefix(value, "\"") {
		return -1
	}
	escaped := false
	for index := 1; index < len(value); index++ {
		switch {
		case escaped:
			escaped = false
		case value[index] == '\\':
			escaped = true
		case value[index] == '"':
			return index + 1
		}
	}
	return -1
}

func isNormalizedTransportSelector(selector string) bool {
	trimmed := strings.TrimSpace(selector)
	if trimmed == "" {
		return false
	}

	for index := 0; index < len(trimmed); {
		_, prefixLen, ok := parseExplicitSelectorPrefix(trimmed[index:])
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
		if _, _, ok := parseExplicitSelectorPrefix(trimmed[index:]); !ok {
			return false
		}
	}

	return true
}

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
	trimmed := strings.TrimSpace(selector)
	if isNormalizedTransportSelector(trimmed) {
		return trimmed
	}
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
