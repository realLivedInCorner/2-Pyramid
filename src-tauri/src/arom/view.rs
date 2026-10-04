//! L2：按需类型化视图。
//!
//! 存储只管条目与字节；类型化视图**按需解析、缓存、写入即失效**（`astray-arom-model.md` §6）：
//!
//! * `pack.mcmeta` —— 收编现状的两套实现（`converters/version_converter.rs:294/:341`
//!   与 `foray/mcmeta.rs:82`）；
//! * 贴图 —— `image(path)` 返回 `Arc<RgbaImage>`，取代「以磁盘路径为键」的缓存；
//! * 未识别的类型退化为原始字节/文本/JSON 视图，**不付解析成本**；
//! * 解析失败一律返回 `AromError::View`，不 panic、不中断转换。
//!
//! 缓存存于 [`Pack`](super::layer::Pack) 内：键是路径，**失效依据是包版本号**——
//! 任何一次 `commit` 都会让整张缓存作废（在途事务的视图不参与缓存）。

use std::collections::HashMap;
use std::sync::Arc;

use image::RgbaImage;

use super::error::AromError;
use super::layer::PackView;

/// 结构化 `pack.mcmeta`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackMeta {
    pub description: String,
    pub pack_format: Option<u32>,
    /// 26.3+ 的 `min_format`（可能是数字、`[major, minor]` 数组或 `{"major": n}`）。
    pub min_format: Option<u32>,
    pub max_format: Option<u32>,
    pub has_overlays: bool,
    /// 原始文本（改写时保留未识别字段的依据）。
    pub raw: String,
    pub warn: Option<String>,
}

impl PackMeta {
    pub const PATH: &'static str = "pack.mcmeta";

    pub fn parse(bytes: &[u8]) -> Result<Self, AromError> {
        let raw = String::from_utf8(bytes.to_vec())
            .map_err(|e| AromError::View(format!("pack.mcmeta is not valid UTF-8: {e}")))?;
        let value: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| AromError::View(format!("pack.mcmeta is not valid JSON: {e}")))?;
        let root = value
            .as_object()
            .ok_or_else(|| AromError::View("pack.mcmeta is not a JSON object".into()))?;
        let pack = root.get("pack").and_then(|v| v.as_object());

        let warn = if pack.is_none() {
            Some("pack.mcmeta has no `pack` object".to_string())
        } else {
            None
        };

        let description = pack
            .and_then(|p| p.get("description"))
            .map(|d| match d {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default();

        let has_overlays = root.contains_key("overlays")
            || pack.map(|p| p.contains_key("overlays")).unwrap_or(false);

        Ok(Self {
            description,
            pack_format: pack.and_then(|p| p.get("pack_format")).and_then(format_major),
            min_format: pack.and_then(|p| p.get("min_format")).and_then(format_major),
            max_format: pack.and_then(|p| p.get("max_format")).and_then(format_major),
            has_overlays,
            raw,
            warn,
        })
    }

    /// 有效主版本号：优先 `pack_format`，其次 `min_format`。
    pub fn effective_format(&self) -> Option<u32> {
        self.pack_format.or(self.min_format)
    }

    /// 现状写路径规则（`converters/version_converter.rs:363–390`）：目标 ≥ 69 时只写
    /// `min_format`/`max_format`，不再写 `pack_format`。
    pub fn wants_min_max_format(target: u32) -> bool {
        target >= 69
    }
}

fn format_major(value: &serde_json::Value) -> Option<u32> {
    match value {
        serde_json::Value::Number(n) => n.as_u64().map(|x| x as u32),
        // 26.3 的写法是 [major, minor]
        serde_json::Value::Array(a) => a.first().and_then(|x| x.as_u64()).map(|x| x as u32),
        serde_json::Value::Object(o) => o
            .get("major")
            .and_then(|x| x.as_u64())
            .map(|x| x as u32),
        _ => None,
    }
}

/// 视图缓存：按包版本号整体失效。
#[derive(Debug, Default)]
pub struct ViewCache {
    version: u64,
    mcmeta: HashMap<String, Arc<PackMeta>>,
    images: HashMap<String, Arc<RgbaImage>>,
}

impl ViewCache {
    /// 版本变化即整体作废。
    pub fn sync(&mut self, version: u64) {
        if self.version != version {
            self.version = version;
            self.mcmeta.clear();
            self.images.clear();
        }
    }

    pub fn cached_mcmeta(&self) -> usize {
        self.mcmeta.len()
    }

    pub fn cached_images(&self) -> usize {
        self.images.len()
    }

    pub fn get_mcmeta(&self, path: &str) -> Option<Arc<PackMeta>> {
        self.mcmeta.get(path).cloned()
    }

    pub fn put_mcmeta(&mut self, path: &str, meta: Arc<PackMeta>) {
        self.mcmeta.insert(path.to_string(), meta);
    }

    pub fn get_image(&self, path: &str) -> Option<Arc<RgbaImage>> {
        self.images.get(path).cloned()
    }

    pub fn put_image(&mut self, path: &str, img: Arc<RgbaImage>) {
        self.images.insert(path.to_string(), img);
    }

    pub fn clear(&mut self) {
        self.mcmeta.clear();
        self.images.clear();
    }
}

impl<'a> PackView<'a> {
    /// 根 `pack.mcmeta`；缺失时返回 `View` 错误（调用方可降级）。
    pub fn mcmeta(&self) -> Result<Arc<PackMeta>, AromError> {
        self.mcmeta_at(PackMeta::PATH)
    }

    pub fn mcmeta_at(&self, path: &str) -> Result<Arc<PackMeta>, AromError> {
        // 在途事务的视图不参与缓存（否则未提交内容会漏进其它视图，且版本号不会变化）
        let cacheable = !self.is_pending();
        if cacheable {
            if let Some(hit) = self.pack.cached_mcmeta(path) {
                return Ok(hit);
            }
        }
        let bytes = self.read(path)?.ok_or_else(|| {
            AromError::View(format!("pack metadata not found: {path}"))
        })?;
        let meta = Arc::new(PackMeta::parse(&bytes)?);
        if cacheable {
            self.pack.store_mcmeta(path, meta.clone());
        }
        Ok(meta)
    }

    /// 贴图视图：解码为 `RgbaImage` 并缓存（取代「磁盘路径为键」的旧缓存）。
    pub fn image(&self, path: &str) -> Result<Arc<RgbaImage>, AromError> {
        let cacheable = !self.is_pending();
        if cacheable {
            if let Some(hit) = self.pack.cached_image(path) {
                return Ok(hit);
            }
        }
        let bytes = self
            .read(path)?
            .ok_or_else(|| AromError::View(format!("image not found: {path}")))?;
        let decoded = image::load_from_memory(&bytes)
            .map_err(|e| AromError::View(format!("decode image `{path}`: {e}")))?;
        let img = Arc::new(decoded.to_rgba8());
        if cacheable {
            self.pack.store_image(path, img.clone());
        }
        Ok(img)
    }

    /// 文本视图（UTF-8）。
    pub fn text(&self, path: &str) -> Result<String, AromError> {
        let bytes = self
            .read(path)?
            .ok_or_else(|| AromError::View(format!("text not found: {path}")))?;
        String::from_utf8(bytes)
            .map_err(|e| AromError::View(format!("`{path}` is not valid UTF-8: {e}")))
    }

    /// JSON 视图（不缓存：调用方通常只读一次并立即改写）。
    pub fn json(&self, path: &str) -> Result<serde_json::Value, AromError> {
        let text = self.text(path)?;
        serde_json::from_str(&text)
            .map_err(|e| AromError::View(format!("`{path}` is not valid JSON: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arom::layer::Pack;
    use crate::arom::limits::SafeLimits;
    use crate::arom::source::MemSource;

    fn pack_with_mcmeta(json: &str, extra: Vec<(&str, Vec<u8>)>) -> Pack {
        let mut files: Vec<(String, Vec<u8>)> =
            vec![(PackMeta::PATH.to_string(), json.as_bytes().to_vec())];
        files.extend(extra.into_iter().map(|(p, b)| (p.to_string(), b)));
        let src = MemSource::new(files).expect("mem source");
        Pack::from_source(Box::new(src), None).expect("pack")
    }

    fn png(color: [u8; 4], w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(w, h, image::Rgba(color));
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .expect("encode");
        buf
    }

    #[test]
    fn mcmeta_is_typed_and_cached() {
        let pack = pack_with_mcmeta(
            r#"{"pack":{"pack_format":34,"description":"hello"},"overlays":{"entries":[]}}"#,
            vec![],
        );
        let view = pack.view();
        let meta = view.mcmeta().expect("mcmeta");
        assert_eq!(meta.pack_format, Some(34));
        assert_eq!(meta.description, "hello");
        assert!(meta.has_overlays);
        assert_eq!(meta.effective_format(), Some(34));

        let again = view.mcmeta().expect("mcmeta");
        assert!(Arc::ptr_eq(&meta, &again), "第二次必须命中缓存");
        assert_eq!(pack.cached_mcmeta_count(), 1);
    }

    #[test]
    fn mcmeta_cache_invalidates_on_commit() {
        let mut pack = pack_with_mcmeta(r#"{"pack":{"pack_format":34,"description":"old"}}"#, vec![]);
        assert_eq!(
            pack.view().mcmeta().expect("mcmeta").description,
            "old"
        );

        let mut tx = pack.tx("test");
        tx.put(
            PackMeta::PATH,
            br#"{"pack":{"pack_format":97,"description":"new"}}"#.to_vec(),
        )
        .expect("put");
        tx.commit().expect("commit");

        let meta = pack.view().mcmeta().expect("mcmeta");
        assert_eq!(meta.description, "new", "写入必须让缓存失效");
        assert_eq!(meta.pack_format, Some(97));
    }

    #[test]
    fn mcmeta_understands_26x_arrays_and_nested_objects() {
        let meta = PackMeta::parse(
            br#"{"pack":{"min_format":[97,0],"max_format":{"major":97},"description":{"text":"d"}}}"#,
        )
        .expect("parse");
        assert_eq!(meta.min_format, Some(97), "数组形式取主版本");
        assert_eq!(meta.max_format, Some(97), "对象形式取 major");
        assert_eq!(meta.pack_format, None);
        assert_eq!(meta.effective_format(), Some(97), "回落到 min_format");
        assert_eq!(meta.description, "{\"text\":\"d\"}", "富文本描述原样保留");
        assert!(!meta.has_overlays);
    }

    #[test]
    fn invalid_mcmeta_is_a_view_error() {
        let err = PackMeta::parse(b"not json").expect_err("must fail");
        assert_eq!(err.kind(), "view");

        let pack = pack_with_mcmeta(r#"["array"]"#, vec![]);
        let err = pack.view().mcmeta().expect_err("must fail");
        assert_eq!(err.kind(), "view");

        let empty = MemSource::new(vec![("a.txt".to_string(), b"x".to_vec())]).expect("mem");
        let pack = Pack::from_source(Box::new(empty), None).expect("pack");
        let err = pack.view().mcmeta().expect_err("missing mcmeta");
        assert_eq!(err.kind(), "view");
    }

    #[test]
    fn missing_pack_object_is_tolerated_with_warning() {
        let meta = PackMeta::parse(br#"{"description":"no pack here"}"#).expect("parse");
        assert!(meta.warn.is_some());
        assert_eq!(meta.pack_format, None);
    }

    #[test]
    fn image_view_decodes_and_caches_then_invalidates() {
        let mut pack = pack_with_mcmeta(
            r#"{"pack":{"pack_format":34}}"#,
            vec![("t.png", png([1, 2, 3, 255], 4, 6))],
        );

        let view = pack.view();
        let img = view.image("t.png").expect("image");
        assert_eq!((img.width(), img.height()), (4, 6));
        assert!(Arc::ptr_eq(&img, &view.image("t.png").expect("cached")), "第二次命中缓存");
        assert_eq!(pack.cached_image_count(), 1);

        // 写入同路径的新图 → 缓存作废、内容更新
        let mut tx = pack.tx("test");
        let new_img = image::RgbaImage::from_pixel(2, 2, image::Rgba([9, 9, 9, 255]));
        tx.put_image("t.png", &new_img).expect("put_image");
        tx.commit().expect("commit");

        let img = pack.view().image("t.png").expect("image");
        assert_eq!((img.width(), img.height()), (2, 2), "缓存必须失效");
    }

    #[test]
    fn image_view_reports_decode_failures() {
        let pack = pack_with_mcmeta(
            r#"{"pack":{"pack_format":34}}"#,
            vec![("t.png", b"not a png".to_vec())],
        );
        let err = pack.view().image("t.png").expect_err("must fail");
        assert_eq!(err.kind(), "view");

        let err = pack.view().image("missing.png").expect_err("must fail");
        assert_eq!(err.kind(), "view");
    }

    #[test]
    fn text_and_json_views_behave() {
        let pack = pack_with_mcmeta(
            r#"{"pack":{"pack_format":34}}"#,
            vec![
                ("data.json", br#"{"a":1}"#.to_vec()),
                ("bad.bin", vec![0xff, 0xfe, 0xfd]),
            ],
        );
        let view = pack.view();
        assert_eq!(view.text("data.json").expect("text"), r#"{"a":1}"#);
        assert_eq!(view.json("data.json").expect("json")["a"], 1);

        let err = view.json("bad.bin").expect_err("must fail");
        assert_eq!(err.kind(), "view");
        let err = view.text("bad.bin").expect_err("must fail");
        assert_eq!(err.kind(), "view");
    }

    #[test]
    fn min_max_format_rule_matches_the_current_pipeline() {
        assert!(!PackMeta::wants_min_max_format(68));
        assert!(PackMeta::wants_min_max_format(69));
        assert!(PackMeta::wants_min_max_format(97));
    }
}
