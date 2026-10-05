    use super::*;

    /// `cut_gui` 的任务声明。
    ///
    /// 阶段与注册表一致：`invoke_conversion.rs` 把 `cut_gui` 登记为
    /// `TaskType::Hybrid` / `Tier::Surgeon`。范围覆盖 gui 子树
    /// （读 `container/*.png`、写 `sprites/**`）。
    ///
    /// **§9.135**：注释原先写着"`decl()` 没有调用者"——那是 §9.113 的实况（当时 `cut_gui`
    /// 只能经注册闭包在旧批次内执行）。§9.129 把它并入驱动派发表之后本函数就是生产路径，
    /// `#[allow(dead_code)]` 与那句说明随之删除。
    pub fn decl() -> TaskDecl {
        TaskDecl::new("cut_gui", Tier::Surgeon)
            .reads(ScopeSet::prefix("assets/minecraft/textures/gui"))
            .writes(ScopeSet::prefix("assets/minecraft/textures/gui"))
            .exclusive(true)
    }

    // §9.135：本模块原先还有一个 **workdir 形态入口** `run_in_workdir(workdir)`，
    // 以及它的两个助手 `collect_gui_files` / `write_layer_to_dir`（合计 ~90 行）。
    //
    // 它做的是「读 workdir 的 gui 子树 → 建内存包（`MemSource`）→ 跑 `Tx` → 把层写回
    // workdir」——存在的**唯一理由**是 §9.113 的处境：`cut_gui` 的槽位在**旧批次内部**
    // （`(15,18)`），而批次只能整体喂给旧引擎，`Tx` 形态只在批次之外可用。
    //
    // §9.129 起驱动按 `plan` 逐任务派发 `Tx`，那个理由消失了；§9.135 把「批次后的直连
    // 步骤」也改成直接跑在驱动自己的 pack 上（`native_run` 里 `pack.tx("cut_gui_direct")`），
    // 于是这层往返**没有任何调用者**，已整体删除。
    //
    // 判据是冻结指纹：改动前后都是 4018 条目 / `0x75bb3260e7f578a6`——少了一趟
    // "序列化到磁盘再读回来"，产物一字未变。
