//! A-ROM 统一错误类型。
//!
//! 命令层边界保留既有的 `Result<_, String>` 形状（见文件末尾的 `From<AromError> for String`），
//! 因此引入这个枚举**不会**改动 87 个 Tauri 命令与 CLI 的对外签名——这是总纲 2.10
//! 「对外承诺不得破」的直接落地。

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AromError {
    /// 文件系统 / IO 失败。
    Io(String),
    /// 容器（zip）结构或读取失败。
    Zip(String),
    /// 触碰限额（条目数 / 深度 / 单文件 / 总量 / 压缩比）。
    Limit(String),
    /// 路径非法（绝对路径 / `..` 逃逸 / NUL）。
    Path(String),
    /// 类型化视图解析失败（L2，Step 5 使用）。
    View(String),
    /// 内存预算不足且无法溢出（Step 4 使用）。
    Budget(String),
    /// 层合并冲突（Step 3 使用）。
    Conflict(String),
    /// 内部不变量被破坏。
    Internal(String),
}

impl AromError {
    /// 稳定的分类标签，便于日志与测试断言。
    pub fn kind(&self) -> &'static str {
        match self {
            AromError::Io(_) => "io",
            AromError::Zip(_) => "zip",
            AromError::Limit(_) => "limit",
            AromError::Path(_) => "path",
            AromError::View(_) => "view",
            AromError::Budget(_) => "budget",
            AromError::Conflict(_) => "conflict",
            AromError::Internal(_) => "internal",
        }
    }

    pub fn io(e: impl fmt::Display) -> Self {
        AromError::Io(e.to_string())
    }

    pub fn zip(e: impl fmt::Display) -> Self {
        AromError::Zip(e.to_string())
    }

    pub fn limit(e: impl fmt::Display) -> Self {
        AromError::Limit(e.to_string())
    }

    pub fn path(e: impl fmt::Display) -> Self {
        AromError::Path(e.to_string())
    }

    pub fn internal(e: impl fmt::Display) -> Self {
        AromError::Internal(e.to_string())
    }
}

impl fmt::Display for AromError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, msg) = match self {
            AromError::Io(m) => ("io", m),
            AromError::Zip(m) => ("zip", m),
            AromError::Limit(m) => ("limit", m),
            AromError::Path(m) => ("path", m),
            AromError::View(m) => ("view", m),
            AromError::Budget(m) => ("budget", m),
            AromError::Conflict(m) => ("conflict", m),
            AromError::Internal(m) => ("internal", m),
        };
        write!(f, "arom/{kind}: {msg}")
    }
}

impl std::error::Error for AromError {}

/// 命令层边界：既有 `Result<_, String>` 形状不变。
impl From<AromError> for String {
    fn from(e: AromError) -> String {
        e.to_string()
    }
}
