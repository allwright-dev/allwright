//! A standard block-YAML encoding of JSON's data model. Quote every string using
//! JSON escapes (also valid YAML) so YAML 1.1 and 1.2 parsers agree on types.
use serde_json::Value;
use std::fmt::Write;

pub(super) fn to_string(value: &Value) -> Result<String, String> {
    let mut output = String::new();
    write_value(value, 0, &mut output)?;
    Ok(output)
}

fn scalar(value: &Value) -> Result<String, String> {
    serde_json::to_string(value)
        // YAML treats these as line breaks even inside quoted scalars.
        .map(|text| {
            text.replace('\u{0085}', "\\u0085")
                .replace('\u{2028}', "\\u2028")
                .replace('\u{2029}', "\\u2029")
        })
        .map_err(|error| error.to_string())
}

fn is_collection(value: &Value) -> bool {
    match value {
        Value::Array(items) => !items.is_empty(),
        Value::Object(items) => !items.is_empty(),
        _ => false,
    }
}

fn write_entry(value: &Value, indent: usize, output: &mut String) -> Result<(), String> {
    if is_collection(value) {
        output.push('\n');
        write_value(value, indent + 2, output)
    } else {
        writeln!(output, " {}", scalar(value)?).unwrap();
        Ok(())
    }
}

fn write_value(value: &Value, indent: usize, output: &mut String) -> Result<(), String> {
    let padding = " ".repeat(indent);
    match value {
        Value::Object(items) if !items.is_empty() => {
            for (key, child) in items {
                write!(output, "{padding}{}:", scalar(&Value::String(key.clone()))?).unwrap();
                write_entry(child, indent, output)?;
            }
        }
        Value::Array(items) if !items.is_empty() => {
            for child in items {
                write!(output, "{padding}-").unwrap();
                write_entry(child, indent, output)?;
            }
        }
        _ => writeln!(output, "{padding}{}", scalar(value)?).unwrap(),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn scalar_types_survive_standard_yaml_parsing() {
        let value = json!({"yes": ["yes", "NO", "on", "OFF", "2026-09-05", "12:34",
            "true", "null", "001", "a\nb\r\tc", "\u{0085}\u{2028}\u{2029}", "😀",
            "\\\"# - [] {} :", true, false, null, 2, 1.5, [], {}]});
        let output = to_string(&value).unwrap();
        assert!(output.contains("\"2026-09-05\""));
        assert!(output.contains("\"yes\""));
        assert_eq!(serde_yaml::from_str::<Value>(&output).unwrap(), value);
    }
}
