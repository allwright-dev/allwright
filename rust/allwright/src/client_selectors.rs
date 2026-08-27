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

fn parse_explicit_selector_prefix(selector: &str) -> Option<(SelectorFlavor, usize)> {
    let lowered = selector.to_ascii_lowercase();
    if lowered.starts_with("xpath=") || lowered.starts_with("xpath:") {
        return Some((SelectorFlavor::XPath, 6));
    }
    if lowered.starts_with("css=") || lowered.starts_with("css:") {
        return Some((SelectorFlavor::Css, 4));
    }
    None
}

fn find_json_string_end(value: &str) -> Option<usize> {
    let bytes = value.as_bytes();
    if bytes.first().copied()? != b'"' {
        return None;
    }

    let mut index = 1usize;
    let mut escaped = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        match byte {
            b'\\' => escaped = true,
            b'"' => return Some(index + 1),
            _ => {}
        }
        index += 1;
    }
    None
}

fn is_normalized_transport_selector(selector: &str) -> bool {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return false;
    }

    let mut index = 0usize;
    while index < trimmed.len() {
        let Some((_, prefix_len)) = parse_explicit_selector_prefix(&trimmed[index..]) else {
            return false;
        };

        index += prefix_len;
        let remainder = &trimmed[index..];
        if !remainder.starts_with('"') {
            return false;
        }

        let Some(json_end) = find_json_string_end(remainder) else {
            return false;
        };
        index += json_end;

        if index == trimmed.len() {
            return true;
        }

        let whitespace_len = trimmed[index..]
            .chars()
            .take_while(|char| char.is_ascii_whitespace())
            .count();
        if whitespace_len == 0 {
            return false;
        }
        index += whitespace_len;

        if parse_explicit_selector_prefix(&trimmed[index..]).is_none() {
            return false;
        }
    }

    true
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
    let trimmed = selector.trim();
    if is_normalized_transport_selector(trimmed) {
        return trimmed.to_string();
    }
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
