package allwright

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestWebLocatorTransport(t *testing.T) {
	page := &Tab{}
	pressed := false
	inner := page.GetByRole("button", RoleOptions{Name: TextPattern{Regex: `^Save "now"$`, Flags: "i"}, Pressed: &pressed})
	var body string
	if err := json.Unmarshal([]byte(strings.TrimPrefix(inner.Selector(), "aw=")), &body); err != nil {
		t.Fatal(err)
	}
	var spec map[string]any
	if err := json.Unmarshal([]byte(body), &spec); err != nil {
		t.Fatal(err)
	}
	if spec["pressed"] != false {
		t.Fatalf("lost false filter: %s", body)
	}
	locator := page.Locator("article").Filter(LocatorFilterOptions{Has: inner, HasNotText: `xpath="trap"`, Visible: &pressed}).GetByTestId(`a"b`).Last()
	if normalizeSelectorForTransport(locator.Selector()) != locator.Selector() {
		t.Fatal("normalization corrupted semantic chain")
	}
}
func TestWebLocatorFilterScope(t *testing.T) {
	defer func() {
		if recover() == nil {
			t.Fatal("expected cross-page filter rejection")
		}
	}()
	(&Tab{}).Locator("li").Filter(LocatorFilterOptions{Has: (&Tab{}).GetByText("other")})
}

func TestLocatorExclusion(t *testing.T) {
	page := &Tab{}
	all := page.GetByRole("button")
	result := all.Not(page.GetByText("Cancel")).First()
	if !strings.Contains(result.Selector(), "exclude") || strings.Contains(all.Selector(), "exclude") {
		t.Fatal("exclusion must create an immutable locator")
	}
	if normalizeSelectorForTransport(result.Selector()) != result.Selector() {
		t.Fatal("exclusion transport corrupted")
	}
	defer func() {
		if recover() == nil {
			t.Fatal("expected cross-page exclusion rejection")
		}
	}()
	all.Not((&Tab{}).GetByText("Cancel"))
}
