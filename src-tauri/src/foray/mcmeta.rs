//! pack.mcmeta 解析（限制体积与嵌套）。

use serde::Deserialize;

pub const MAX_MCMETA_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_JSON_DEPTH: usize = 64;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PackSection {
    pub description: Option<serde_json::Value>,
    pub pack_format: Option<u32>,
    pub min_format: Option<serde_json::Value>,
    pub max_format: Option<serde_json::Value>,
    pub supported_formats: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PackMetaInner {
    pub pack: Option<PackSection>,
    pub overlays: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default)]
pub struct PackMeta {
    pub raw: String,
    pub description: String,
    pub pack_format: Option<u32>,
    pub min_format: Option<u32>,
    pub max_format: Option<u32>,
    pub has_overlays: bool,
    pub warn: Option<String>,
}

fn value_as_u32(v: &Option<serde_json::Value>) -> Option<u32> {
    match v {
        Some(serde_json::Value::Number(n)) => n.as_u64().map(|x| x as u32),
        Some(serde_json::Value::Array(a)) => a
            .first()
            .and_then(|x| x.as_u64())
            .map(|x| x as u32),
        _ => None,
    }
}

fn value_to_string(v: &Option<serde_json::Value>) -> String {
    match v {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Object(o)) => {
            if let Some(t) = o.get("text").and_then(|t| t.as_str()) {
                return t.to_string();
            }
            // text component: 尽量取出第一个 string
            o.values()
                .find_map(|x| x.as_str().map(|s| s.to_string()))
                .unwrap_or_default()
        }
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

fn check_depth(v: &serde_json::Value, depth: usize) -> Result<(), String> {
    if depth > MAX_JSON_DEPTH {
        return Err("json too deeply nested".into());
    }
    match v {
        serde_json::Value::Array(a) => {
            for x in a {
                check_depth(x, depth + 1)?;
            }
        }
        serde_json::Value::Object(o) => {
            for (_, x) in o {
                check_depth(x, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn parse_mcmeta(bytes: &[u8]) -> Result<PackMeta, String> {
    if bytes.len() > MAX_MCMETA_BYTES {
        return Err("pack.mcmeta too large".into());
    }
    // strip UTF-8 BOM
    let raw_bytes = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        bytes
    };
    let raw = String::from_utf8_lossy(raw_bytes).to_string();
    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("pack.mcmeta invalid JSON: {e}"))?;
    check_depth(&value, 0)?;

    let parsed: PackMetaInner =
        serde_json::from_value(value).map_err(|e| format!("pack.mcmeta shape: {e}"))?;
    let pack = parsed.pack.unwrap_or_default();

    let mut warn = None;
    if pack.pack_format.is_none() && pack.min_format.is_none() {
        warn = Some("pack.mcmeta missing pack_format / min_format".into());
    }

    Ok(PackMeta {
        description: value_to_string(&pack.description),
        pack_format: pack.pack_format,
        min_format: value_as_u32(&pack.min_format),
        max_format: value_as_u32(&pack.max_format),
        has_overlays: parsed.overlays.is_some(),
        warn,
        raw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_legacy_format() {
        let m = parse_mcmeta(br#"{"pack":{"description":"hi","pack_format":34}}"#).unwrap();
        assert_eq!(m.pack_format, Some(34));
        assert_eq!(m.description, "hi");
    }

    #[test]
    fn parses_min_max_format() {
        let m = parse_mcmeta(br#"{"pack":{"description":"x","min_format":97,"max_format":97}}"#)
            .unwrap();
        assert_eq!(m.min_format, Some(97));
        assert_eq!(m.max_format, Some(97));
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_mcmeta(b"{").is_err());
    }

    #[test]
    fn strips_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(br#"{"pack":{"pack_format":1}}"#);
        let m = parse_mcmeta(&bytes).unwrap();
        assert_eq!(m.pack_format, Some(1));
    }
}
