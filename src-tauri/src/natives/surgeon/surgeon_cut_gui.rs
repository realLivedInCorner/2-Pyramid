    use super::*;
    use std::path::Path;

    /// `decl()` 目前**没有调用者**——`cut_gui` 由调度器在旧批次内执行（闭包体已是本模块的
    /// `run_in_workdir`），不经过 A-ROM 派发表，故声明暂时用不上；保留它是为了将来真正
    /// 本地化到 `Tx` 形态时可直接接入（那时它就有调用者了）。
    #[allow(dead_code)]
    pub fn decl() -> TaskDecl {
        // 阶段与活注册表一致：`invoke_conversion.rs` 把 `cut_gui` 登记为
        // `TaskType::Hybrid` / `TaskTier::Surgeon`。
        // 范围覆盖 gui 子树（读 `container/*.png`、写 `sprites/**`）。
        TaskDecl::new("cut_gui", Tier::Surgeon)
            .reads(ScopeSet::prefix("assets/minecraft/textures/gui"))
            .writes(ScopeSet::prefix("assets/minecraft/textures/gui"))
            .exclusive(true)
    }

    /// **workdir 形态入口**（§9.113）：内部走 **`Tx`**，即 `gui_surgeon_tx::run`。
    ///
    /// **它做什么**：把 workdir 的 `gui/` 子树读成一份内存包（`MemSource`），
    /// 在其上跑纯 `Tx` 形态的 `gui_surgeon_tx::run`，再把产出的层**写回 workdir**，
    /// 最后把 `Outcome.deferred_removals` 登记到**调用方的 `ctx`** 上——
    /// 这样收尾的 `execute_cleanup()` 仍在**同一时机**删除它们（与旧实现一致）。
    ///
    /// **为什么只读 `gui/` 子树而不是整包**：`GuiSurgeon` 的读写**全部**在
    /// `assets/minecraft/textures/gui/` 之下（源图与 `sprites/**` 产物），
    /// 清理清单的 20 个文件也都在其内。只读这一棵子树既够用，又避免把整个包（几千个文件）
    /// 读进内存。这是**有界且已知**的输入面，不是"碰巧够用"。
    ///
    /// **为什么需要这层桥**：`cut_gui` 的计划槽位在**旧批次内部**（`(15,18)`），
    /// 而 `Tx` 形态只在批次之外可用（§9.100 实测：挪出批次就少 3 个 sprite）。
    /// 因此这里的做法是**位置不变、实现换成 `Tx`**——那层"建内存包 → 应用回 workdir"
    /// 的往返，正是调用方本来就有的 workdir 形态所要求的，不是新增的架构。
    pub fn run_in_workdir(workdir: &Path) -> Result<Outcome, String> {
        crate::log_info!("2-Pyramid: starting cut_gui (native Tx pipeline)...");

        // ① 读 workdir 的 gui 子树 → 内存包
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        collect_gui_files(workdir, GUI_PREFIX, &mut files)?;
        let source = crate::arom::MemSource::new(files).map_err(|e| e.to_string())?;
        let mut pack = crate::arom::Pack::from_source(Box::new(source), None)
            .map_err(|e| e.to_string())?;

        // ② 跑纯 `Tx` 形态，取出层
        let (outcome, layer) = {
            let mut tx = pack.tx("cut_gui");
            let o = super::gui_surgeon_tx::run(&mut tx).map_err(|e| e.to_string())?;
            (o, tx.into_layer())
        };

        // ③ 层写回 workdir（`sprites/**` 产物 + `gui/title/minecraft.png` 的就地回写）
        write_layer_to_dir(workdir, &pack, &layer)?;
        pack.commit(layer);

        // ④ 延迟删除**经 `Outcome` 返回给驱动**（§9.128）。
        //
        // 原先这里把路径 `ctx.defer_remove_file(workdir.join(rel))` 登记到 `HurrayContext`
        // 的清理清单，由驱动末尾的 `ctx.execute_cleanup()` 执行。改走 `Outcome` 的理由：
        //   * 其余 **43 个**原生任务早就是这个机制（`report.deferred_removals`）；
        //   * 驱动里两个清理点**本来就相邻**（`ctx.execute_cleanup()` 之后紧跟
        //     `report.deferred_removals` 的 tombstone），时机完全一致；
        //   * 于是本函数**不再需要 `HurrayContext`** —— 这是"让 A-ROM 取代 hurray"
        //     的第一刀：把最后一个因旧引擎而存在的 context 依赖切断。
        //
        // 注意返回的是**包内相对路径**（与其它原生任务一致），由驱动去 join workdir。
        crate::log_info!(
            "cut_gui: wrote {} entries, deferred {} removals",
            outcome.changed,
            outcome.deferred_removals.len()
        );
        Ok(outcome)
    }

    /// `GuiSurgeon` 的读写**全部**落在这个前缀之下（源图 + `sprites/**` 产物 + 清理清单）。
    const GUI_PREFIX: &str = "assets/minecraft/textures/gui";

    /// 递归收集 `workdir/<rel_prefix>/` 下的所有文件，路径为**包内相对**。
    fn collect_gui_files(
        workdir: &Path,
        rel_prefix: &str,
        out: &mut Vec<(String, Vec<u8>)>,
    ) -> Result<(), String> {
        let dir = workdir.join(rel_prefix);
        let Ok(rd) = std::fs::read_dir(&dir) else {
            return Ok(()); // 目录不存在 = 没有可读输入（GuiSurgeon 会各自跳过）
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            let rel = format!("{rel_prefix}/{name}");
            if p.is_dir() {
                collect_gui_files(workdir, &rel, out)?;
            } else if let Ok(bytes) = std::fs::read(&p) {
                out.push((rel, bytes));
            }
        }
        Ok(())
    }

    /// 把层里的写入落到 `workdir` 上（`GuiSurgeon` 不产生改名规则，故只处理写入）。
    fn write_layer_to_dir(workdir: &Path, pack: &crate::arom::Pack, layer: &crate::arom::Layer) -> Result<(), String> {
        for (path, slot) in layer.writes() {
            let full = workdir.join(path);
            match slot {
                crate::arom::Slot::Tombstone => {
                    if full.is_dir() {
                        std::fs::remove_dir_all(&full).map_err(|e| e.to_string())?;
                    } else if full.exists() {
                        std::fs::remove_file(&full).map_err(|e| e.to_string())?;
                    }
                }
                crate::arom::Slot::Present(crate::arom::Body::Dir) => {
                    std::fs::create_dir_all(&full).map_err(|e| e.to_string())?;
                }
                crate::arom::Slot::Present(body) => {
                    if let Some(parent) = full.parent() {
                        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    }
                    let bytes = pack.read_body(body).map_err(|e| e.to_string())?;
                    std::fs::write(&full, &bytes).map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }
