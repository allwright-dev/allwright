package allwright

import "encoding/json"

// TextPattern carries a JavaScript regular expression and flags to the web surface.
type TextPattern struct {
	Regex string `json:"regex"`
	Flags string `json:"flags,omitempty"`
}
type TextOptions struct {
	Exact bool `json:"exact,omitempty"`
}
type RoleOptions struct {
	Name          any   `json:"name,omitempty"`
	Exact         bool  `json:"exact,omitempty"`
	Checked       *bool `json:"checked,omitempty"`
	Disabled      *bool `json:"disabled,omitempty"`
	Expanded      *bool `json:"expanded,omitempty"`
	IncludeHidden bool  `json:"includeHidden,omitempty"`
	Level         *int  `json:"level,omitempty"`
	Pressed       *bool `json:"pressed,omitempty"`
	Selected      *bool `json:"selected,omitempty"`
}
type LocatorFilterOptions struct {
	Has        *Locator
	HasNot     *Locator
	HasText    any
	HasNotText any
	Visible    *bool
}

func semanticSelector(spec map[string]any) string {
	body, err := json.Marshal(spec)
	if err != nil {
		panic(err)
	}
	quoted, _ := json.Marshal(string(body))
	return "aw=" + string(quoted)
}
func roleSelector(role string, options RoleOptions) string {
	if options.Level != nil && *options.Level < 1 {
		panic("Role level must be positive")
	}
	body, err := json.Marshal(options)
	if err != nil {
		panic(err)
	}
	spec := map[string]any{}
	if err := json.Unmarshal(body, &spec); err != nil {
		panic(err)
	}
	spec["kind"], spec["role"] = "role", role
	return semanticSelector(spec)
}

func (l *Tab) GetByRole(role string, options ...RoleOptions) *Locator {
	o := RoleOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(roleSelector(role, o))
}

func (l *Tab) GetByText(text any, options ...TextOptions) *Locator {
	o := TextOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(semanticSelector(map[string]any{"kind": "text", "text": text, "exact": o.Exact}))
}

func (l *Tab) GetByLabel(text any, options ...TextOptions) *Locator {
	o := TextOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(semanticSelector(map[string]any{"kind": "label", "text": text, "exact": o.Exact}))
}

func (l *Tab) GetByPlaceholder(text any, options ...TextOptions) *Locator {
	o := TextOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(semanticSelector(map[string]any{"kind": "placeholder", "text": text, "exact": o.Exact}))
}

func (l *Tab) GetByAltText(text any, options ...TextOptions) *Locator {
	o := TextOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(semanticSelector(map[string]any{"kind": "altText", "text": text, "exact": o.Exact}))
}

func (l *Tab) GetByTitle(text any, options ...TextOptions) *Locator {
	o := TextOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(semanticSelector(map[string]any{"kind": "title", "text": text, "exact": o.Exact}))
}

func (l *Tab) GetByTestId(text any, options ...TextOptions) *Locator {
	o := TextOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(semanticSelector(map[string]any{"kind": "testId", "text": text, "exact": o.Exact}))
}

func (l *Locator) GetByRole(role string, options ...RoleOptions) *Locator {
	o := RoleOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(roleSelector(role, o))
}

func (l *Locator) GetByText(text any, options ...TextOptions) *Locator {
	o := TextOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(semanticSelector(map[string]any{"kind": "text", "text": text, "exact": o.Exact}))
}

func (l *Locator) GetByLabel(text any, options ...TextOptions) *Locator {
	o := TextOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(semanticSelector(map[string]any{"kind": "label", "text": text, "exact": o.Exact}))
}

func (l *Locator) GetByPlaceholder(text any, options ...TextOptions) *Locator {
	o := TextOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(semanticSelector(map[string]any{"kind": "placeholder", "text": text, "exact": o.Exact}))
}

func (l *Locator) GetByAltText(text any, options ...TextOptions) *Locator {
	o := TextOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(semanticSelector(map[string]any{"kind": "altText", "text": text, "exact": o.Exact}))
}

func (l *Locator) GetByTitle(text any, options ...TextOptions) *Locator {
	o := TextOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(semanticSelector(map[string]any{"kind": "title", "text": text, "exact": o.Exact}))
}

func (l *Locator) GetByTestId(text any, options ...TextOptions) *Locator {
	o := TextOptions{}
	if len(options) > 0 {
		o = options[0]
	}
	return l.Locator(semanticSelector(map[string]any{"kind": "testId", "text": text, "exact": o.Exact}))
}

func (l *Locator) Filter(options LocatorFilterOptions) *Locator {
	spec := map[string]any{"kind": "filter"}
	for key, inner := range map[string]*Locator{"has": options.Has, "hasNot": options.HasNot} {
		if inner != nil {
			if inner.page != l.page {
				panic("Filter locators must belong to the same page")
			}
			spec[key] = inner.selector
		}
	}
	if options.HasText != nil {
		spec["hasText"] = options.HasText
	}
	if options.HasNotText != nil {
		spec["hasNotText"] = options.HasNotText
	}
	if options.Visible != nil {
		spec["visible"] = *options.Visible
	}
	return l.Locator(semanticSelector(spec))
}
func (l *Locator) Nth(index int) *Locator {
	return l.Locator(semanticSelector(map[string]any{"kind": "nth", "index": index}))
}
func (l *Locator) First() *Locator { return l.Nth(0) }
func (l *Locator) Last() *Locator  { return l.Nth(-1) }

// Not excludes the nodes matched by other, preserving this locator's order and scope.
func (l *Locator) Not(other *Locator) *Locator {
	if other == nil || other.page != l.page {
		panic("Excluded locators must belong to the same page")
	}
	return l.Locator(semanticSelector(map[string]any{"kind": "exclude", "selector": other.selector}))
}
