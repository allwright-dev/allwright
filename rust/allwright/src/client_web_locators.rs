use super::types::{Locator, Tab};
use serde::Serialize;
use serde_json::json;

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum TextMatcher {
    Text(String),
    Regex { regex: String, flags: String },
}
impl From<&str> for TextMatcher {
    fn from(value: &str) -> Self {
        Self::Text(value.into())
    }
}
impl From<String> for TextMatcher {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct TextOptions {
    pub exact: bool,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<TextMatcher>,
    pub exact: bool,
    pub include_hidden: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expanded: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pressed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
}
#[derive(Clone, Default)]
pub struct LocatorFilterOptions {
    pub has: Option<Locator>,
    pub has_not: Option<Locator>,
    pub has_text: Option<TextMatcher>,
    pub has_not_text: Option<TextMatcher>,
    pub visible: Option<bool>,
}
fn semantic_selector(spec: serde_json::Value) -> String {
    format!(
        "aw={}",
        serde_json::to_string(&spec.to_string()).expect("JSON string")
    )
}
impl Tab {
    pub fn get_by_role(&self, role: impl Into<String>) -> Locator {
        self.get_by_role_with_options(role, RoleOptions::default())
    }
    pub fn get_by_role_with_options(
        &self,
        role: impl Into<String>,
        options: RoleOptions,
    ) -> Locator {
        assert!(options.level != Some(0), "Role level must be positive");
        let mut spec = serde_json::to_value(options).expect("role options");
        spec["kind"] = json!("role");
        spec["role"] = json!(role.into());
        self.locator(semantic_selector(spec))
    }
    pub fn get_by_text(&self, text: impl Into<TextMatcher>) -> Locator {
        self.get_by_text_with_options(text, TextOptions::default())
    }
    pub fn get_by_text_with_options(
        &self,
        text: impl Into<TextMatcher>,
        options: TextOptions,
    ) -> Locator {
        self.locator(semantic_selector(
            json!({"kind":"text","text":text.into(),"exact":options.exact}),
        ))
    }
    pub fn get_by_label(&self, text: impl Into<TextMatcher>) -> Locator {
        self.get_by_label_with_options(text, TextOptions::default())
    }
    pub fn get_by_label_with_options(
        &self,
        text: impl Into<TextMatcher>,
        options: TextOptions,
    ) -> Locator {
        self.locator(semantic_selector(
            json!({"kind":"label","text":text.into(),"exact":options.exact}),
        ))
    }
    pub fn get_by_placeholder(&self, text: impl Into<TextMatcher>) -> Locator {
        self.get_by_placeholder_with_options(text, TextOptions::default())
    }
    pub fn get_by_placeholder_with_options(
        &self,
        text: impl Into<TextMatcher>,
        options: TextOptions,
    ) -> Locator {
        self.locator(semantic_selector(
            json!({"kind":"placeholder","text":text.into(),"exact":options.exact}),
        ))
    }
    pub fn get_by_alt_text(&self, text: impl Into<TextMatcher>) -> Locator {
        self.get_by_alt_text_with_options(text, TextOptions::default())
    }
    pub fn get_by_alt_text_with_options(
        &self,
        text: impl Into<TextMatcher>,
        options: TextOptions,
    ) -> Locator {
        self.locator(semantic_selector(
            json!({"kind":"altText","text":text.into(),"exact":options.exact}),
        ))
    }
    pub fn get_by_title(&self, text: impl Into<TextMatcher>) -> Locator {
        self.get_by_title_with_options(text, TextOptions::default())
    }
    pub fn get_by_title_with_options(
        &self,
        text: impl Into<TextMatcher>,
        options: TextOptions,
    ) -> Locator {
        self.locator(semantic_selector(
            json!({"kind":"title","text":text.into(),"exact":options.exact}),
        ))
    }
    pub fn get_by_test_id(&self, text: impl Into<TextMatcher>) -> Locator {
        self.get_by_test_id_with_options(text, TextOptions::default())
    }
    pub fn get_by_test_id_with_options(
        &self,
        text: impl Into<TextMatcher>,
        options: TextOptions,
    ) -> Locator {
        self.locator(semantic_selector(
            json!({"kind":"testId","text":text.into(),"exact":options.exact}),
        ))
    }
}
impl Locator {
    pub fn get_by_role(&self, role: impl Into<String>) -> Locator {
        self.get_by_role_with_options(role, RoleOptions::default())
    }
    pub fn get_by_role_with_options(
        &self,
        role: impl Into<String>,
        options: RoleOptions,
    ) -> Locator {
        assert!(options.level != Some(0), "Role level must be positive");
        let mut spec = serde_json::to_value(options).expect("role options");
        spec["kind"] = json!("role");
        spec["role"] = json!(role.into());
        self.locator(semantic_selector(spec))
    }
    pub fn get_by_text(&self, text: impl Into<TextMatcher>) -> Locator {
        self.get_by_text_with_options(text, TextOptions::default())
    }
    pub fn get_by_text_with_options(
        &self,
        text: impl Into<TextMatcher>,
        options: TextOptions,
    ) -> Locator {
        self.locator(semantic_selector(
            json!({"kind":"text","text":text.into(),"exact":options.exact}),
        ))
    }
    pub fn get_by_label(&self, text: impl Into<TextMatcher>) -> Locator {
        self.get_by_label_with_options(text, TextOptions::default())
    }
    pub fn get_by_label_with_options(
        &self,
        text: impl Into<TextMatcher>,
        options: TextOptions,
    ) -> Locator {
        self.locator(semantic_selector(
            json!({"kind":"label","text":text.into(),"exact":options.exact}),
        ))
    }
    pub fn get_by_placeholder(&self, text: impl Into<TextMatcher>) -> Locator {
        self.get_by_placeholder_with_options(text, TextOptions::default())
    }
    pub fn get_by_placeholder_with_options(
        &self,
        text: impl Into<TextMatcher>,
        options: TextOptions,
    ) -> Locator {
        self.locator(semantic_selector(
            json!({"kind":"placeholder","text":text.into(),"exact":options.exact}),
        ))
    }
    pub fn get_by_alt_text(&self, text: impl Into<TextMatcher>) -> Locator {
        self.get_by_alt_text_with_options(text, TextOptions::default())
    }
    pub fn get_by_alt_text_with_options(
        &self,
        text: impl Into<TextMatcher>,
        options: TextOptions,
    ) -> Locator {
        self.locator(semantic_selector(
            json!({"kind":"altText","text":text.into(),"exact":options.exact}),
        ))
    }
    pub fn get_by_title(&self, text: impl Into<TextMatcher>) -> Locator {
        self.get_by_title_with_options(text, TextOptions::default())
    }
    pub fn get_by_title_with_options(
        &self,
        text: impl Into<TextMatcher>,
        options: TextOptions,
    ) -> Locator {
        self.locator(semantic_selector(
            json!({"kind":"title","text":text.into(),"exact":options.exact}),
        ))
    }
    pub fn get_by_test_id(&self, text: impl Into<TextMatcher>) -> Locator {
        self.get_by_test_id_with_options(text, TextOptions::default())
    }
    pub fn get_by_test_id_with_options(
        &self,
        text: impl Into<TextMatcher>,
        options: TextOptions,
    ) -> Locator {
        self.locator(semantic_selector(
            json!({"kind":"testId","text":text.into(),"exact":options.exact}),
        ))
    }
}
impl Locator {
    pub fn not(&self, other: &Locator) -> Locator {
        assert!(
            std::sync::Arc::ptr_eq(&self.page.inner, &other.page.inner),
            "Excluded locators must belong to the same page"
        );
        self.locator(semantic_selector(
            json!({"kind":"exclude", "selector":other.selector}),
        ))
    }

    pub fn filter(&self, options: LocatorFilterOptions) -> Locator {
        let mut spec = json!({"kind":"filter"});
        for (key, inner) in [("has", options.has), ("hasNot", options.has_not)] {
            if let Some(inner) = inner {
                assert!(
                    std::sync::Arc::ptr_eq(&self.page.inner, &inner.page.inner),
                    "Filter locators must belong to the same page"
                );
                spec[key] = json!(inner.selector);
            }
        }
        if let Some(text) = options.has_text {
            spec["hasText"] = json!(text);
        }
        if let Some(text) = options.has_not_text {
            spec["hasNotText"] = json!(text);
        }
        if let Some(visible) = options.visible {
            spec["visible"] = json!(visible);
        }
        self.locator(semantic_selector(spec))
    }
    pub fn nth(&self, index: i32) -> Locator {
        self.locator(semantic_selector(json!({"kind":"nth", "index":index})))
    }
    pub fn first(&self) -> Locator {
        self.nth(0)
    }
    pub fn last(&self) -> Locator {
        self.nth(-1)
    }
}

#[cfg(test)]
mod tests {
    use super::super::selectors::{chain_selector_for_transport, normalize_selector_for_transport};
    use super::*;

    #[test]
    fn quoted_regex_and_false_options_survive_chaining() {
        let options = RoleOptions {
            name: Some(TextMatcher::Regex {
                regex: "^Save \"now\"$".into(),
                flags: "i".into(),
            }),
            pressed: Some(false),
            ..Default::default()
        };
        let mut spec = serde_json::to_value(options).unwrap();
        assert_eq!(spec["pressed"], false);
        assert!(spec.get("checked").is_none());
        spec["kind"] = json!("role");
        spec["role"] = json!("button");
        let selector = semantic_selector(spec);
        let chained = chain_selector_for_transport("article", &selector);
        let chained = chain_selector_for_transport(&chained, "xpath=//span");
        assert_eq!(normalize_selector_for_transport(&chained), chained);
        assert!(chained.starts_with("css=\"article\" aw="));
        assert!(chained.ends_with("xpath=\"//span\""));
    }
}
