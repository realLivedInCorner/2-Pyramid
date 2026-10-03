//! A-ROM 的唯一限额来源。
//!
//! 现状有**两套**互不一致的限额：
//!
//! * `converters/zip.rs:9–11` —— 总解压 500 MB / 条目 100 000 / 深度 64；
//! * `foray/zip_safe.rs:12–31` —— 总解压 1 GiB / 条目 50 000 / 深度 32 / 单文件 64 MiB / 压缩比 200。
//!
//! 本模块是它们的**单一来源**。Step 1 只建立结构与判定逻辑，**尚未替换任何调用点**——
//! 合并取值会改变「被接受的输入集合」，属对外行为变更，需显式裁决（总纲 D24）。
//! 因此这里提供两个预设：默认的 [`SafeLimits::preserving_current`]（保持现状行为）
//! 与显式的 [`SafeLimits::hardened`]（foray 已在用的收紧取值）。

use super::error::AromError;

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;

#[derive(Debug, Clone, PartialEq)]
pub struct SafeLimits {
    /// 单个条目解压后大小上限；`None` = 不限制（现状行为）。
    pub max_file_bytes: Option<u64>,
    /// 所有条目解压后总大小上限。
    pub max_total_bytes: u64,
    /// 条目数上限。
    pub max_entries: usize,
    /// 路径深度上限（路径段数）。
    pub max_depth: usize,
    /// 声明解压大小 / 压缩后大小 的上限；`None` = 不判定（现状行为）。
    pub max_compression_ratio: Option<f64>,
}

impl SafeLimits {
    /// 保持现状「被接受的输入集合」：两套限额按字段取**宽松者**，
    /// 新引入的两项限制（单文件大小、压缩比）关闭。
    pub fn preserving_current() -> Self {
        Self {
            max_file_bytes: None,
            max_total_bytes: GIB,
            max_entries: 100_000,
            max_depth: 64,
            max_compression_ratio: None,
        }
    }

    /// 收紧档：foray 已在使用的取值，仅在显式选择时生效。
    pub fn hardened() -> Self {
        Self {
            max_file_bytes: Some(64 * MIB),
            max_total_bytes: GIB,
            max_entries: 50_000,
            max_depth: 32,
            max_compression_ratio: Some(200.0),
        }
    }

    /// 仅测试使用：不设限。
    pub fn unlimited() -> Self {
        Self {
            max_file_bytes: None,
            max_total_bytes: u64::MAX,
            max_entries: usize::MAX,
            max_depth: usize::MAX,
            max_compression_ratio: None,
        }
    }

    pub fn check_entries(&self, count: usize) -> Result<(), AromError> {
        if count > self.max_entries {
            return Err(AromError::limit(format!(
                "too many entries: {count} > {}",
                self.max_entries
            )));
        }
        Ok(())
    }

    pub fn check_depth(&self, depth: usize, name: &str) -> Result<(), AromError> {
        if depth > self.max_depth {
            return Err(AromError::limit(format!(
                "path too deep: {name} ({depth} > {})",
                self.max_depth
            )));
        }
        Ok(())
    }

    pub fn check_file_bytes(&self, size: u64, name: &str) -> Result<(), AromError> {
        if let Some(max) = self.max_file_bytes {
            if size > max {
                return Err(AromError::limit(format!(
                    "entry too large: {name} ({size} > {max} bytes)"
                )));
            }
        }
        Ok(())
    }

    pub fn check_total_bytes(&self, total: u64) -> Result<(), AromError> {
        if total > self.max_total_bytes {
            return Err(AromError::limit(format!(
                "uncompressed size exceeds limit: {total} > {} bytes",
                self.max_total_bytes
            )));
        }
        Ok(())
    }

    /// 压缩比判定：只在两侧都非零时进行（目录条目与空文件不参与）。
    pub fn check_ratio(
        &self,
        uncompressed: u64,
        compressed: u64,
        name: &str,
    ) -> Result<(), AromError> {
        if let Some(max) = self.max_compression_ratio {
            if compressed > 0 && uncompressed > 0 {
                let ratio = uncompressed as f64 / compressed as f64;
                if ratio > max {
                    return Err(AromError::limit(format!(
                        "suspicious compression ratio: {name} ({ratio:.1} > {max})"
                    )));
                }
            }
        }
        Ok(())
    }
}

impl Default for SafeLimits {
    fn default() -> Self {
        Self::preserving_current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_the_looser_of_the_two_existing_sets() {
        let p = SafeLimits::preserving_current();
        // zip.rs:9–11 与 zip_safe.rs:12–31 的宽松者
        assert_eq!(p.max_total_bytes, GIB, "500 MB 与 1 GiB 取 1 GiB");
        assert_eq!(p.max_entries, 100_000, "50 000 与 100 000 取 100 000");
        assert_eq!(p.max_depth, 64, "32 与 64 取 64");
        // 新引入的两项默认关闭，避免改变被接受的输入集合
        assert_eq!(p.max_file_bytes, None);
        assert_eq!(p.max_compression_ratio, None);

        let h = SafeLimits::hardened();
        assert_eq!(h.max_file_bytes, Some(64 * MIB));
        assert_eq!(h.max_entries, 50_000);
        assert_eq!(h.max_depth, 32);
        assert_eq!(h.max_compression_ratio, Some(200.0));
    }

    #[test]
    fn checks_fire_with_stable_kinds() {
        let tight = SafeLimits {
            max_file_bytes: Some(10),
            max_total_bytes: 100,
            max_entries: 2,
            max_depth: 1,
            max_compression_ratio: Some(2.0),
        };
        assert_eq!(tight.check_entries(3).unwrap_err().kind(), "limit");
        assert_eq!(tight.check_depth(2, "a/b.txt").unwrap_err().kind(), "limit");
        assert_eq!(tight.check_file_bytes(11, "big.bin").unwrap_err().kind(), "limit");
        assert_eq!(tight.check_total_bytes(101).unwrap_err().kind(), "limit");
        assert_eq!(tight.check_ratio(100, 10, "bomb.bin").unwrap_err().kind(), "limit");

        // 边界不误伤
        assert!(tight.check_entries(2).is_ok());
        assert!(tight.check_depth(1, "a.txt").is_ok());
        assert!(tight.check_file_bytes(10, "ok.bin").is_ok());
        assert!(tight.check_total_bytes(100).is_ok());
        assert!(tight.check_ratio(20, 10, "ok.bin").is_ok());
        // 空文件 / 目录条目不参与压缩比判定
        assert!(tight.check_ratio(0, 0, "dir/").is_ok());
    }
}
