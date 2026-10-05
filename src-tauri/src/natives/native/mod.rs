//! `natives/native`：原生实现共用的基础设施：延迟删除/条件删除两个 helper，以及 `rename_blocks` 的映射表。
//!
//! 模块**声明**统一留在 `natives/mod.rs`（用 `#[path]` 指向本目录的文件），
//! 因此 `crate::natives::<名字>` 路径保持不变。
//!
//! 下面这行不是装饰：本目录下的模块体用 `use super::*;` 引用 `natives` 的共享项
//! （`Outcome` / `TaskDecl` / `Tx` / `read_dimensions` / 各 `macro_rules!`），
//! 而 `super` 在这里指的是**本模块**——必须显式透传，否则它们全部解析不到。

#![allow(unused_imports)]
pub use crate::natives::*;

// 本组还承载 `all()`（全量试点清单）——**注意**：它定义在本文件而非 `natives/mod.rs`，
// 因为 `natives/mod.rs` 在测试模块声明之后，而 `all()` 原先就在那个位置。

