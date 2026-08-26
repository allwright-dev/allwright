#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectorFlavor {
    Css,
    XPath,
}

impl SelectorFlavor {
    fn as_str(self) -> &'static str {
        match self {
            Self::Css => "css",
            Self::XPath => "xpath",
        }
    }
}

fn decode_selector_body(body: &str) -> String {
    let candidate = body.trim();
    if candidate.len() >= 2 && candidate.starts_with('"') && candidate.ends_with('"') {
        if let Ok(decoded) = serde_json::from_str::<String>(candidate) {
            return decoded;
        }
    }
    candidate.to_string()
}

fn parse_selector_for_transport(selector: &str) -> (SelectorFlavor, String) {
    let trimmed = selector.trim();
    let lowered = trimmed.to_ascii_lowercase();
    if lowered.starts_with("xpath=") || lowered.starts_with("xpath:") {
        return (SelectorFlavor::XPath, decode_selector_body(&trimmed[6..]));
    }
    if lowered.starts_with("css=") || lowered.starts_with("css:") {
        return (SelectorFlavor::Css, decode_selector_body(&trimmed[4..]));
    }
    if trimmed.starts_with("//")
        || trimmed.starts_with(".//")
        || trimmed.starts_with("../")
        || trimmed.starts_with('/')
        || trimmed.starts_with('(')
    {
        return (SelectorFlavor::XPath, trimmed.to_string());
    }
    (SelectorFlavor::Css, trimmed.to_string())
}

pub(crate) fn normalize_selector_for_transport(selector: &str) -> String {
    let (flavor, body) = parse_selector_for_transport(selector);
    format!(
        "{}={}",
        flavor.as_str(),
        serde_json::to_string(&body).unwrap_or_else(|_| format!("{body:?}"))
    )
}

pub(crate) fn chain_selector_for_transport(parent: &str, child: &str) -> String {
    let parent_selector = if parent.trim().is_empty() {
        String::new()
    } else {
        normalize_selector_for_transport(parent)
    };
    let child_selector = if child.trim().is_empty() {
        String::new()
    } else {
        normalize_selector_for_transport(child)
    };
    if parent_selector.is_empty() {
        return child_selector;
    }
    if child_selector.is_empty() {
        return parent_selector;
    }
    format!("{parent_selector} {child_selector}")
}
