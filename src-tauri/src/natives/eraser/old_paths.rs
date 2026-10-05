    use super::*;

    /// 与旧实现逐字对应的映射表。
    pub const MAPPINGS: [(&str, &str); 2] = [
        ("assets/minecraft/terrain.png", "assets/minecraft/block.png"),
        ("assets/minecraft/gui/items.png", "assets/minecraft/item.png"),
    ];

    pub fn decl() -> TaskDecl {
        let mut reads = ScopeSet::none();
        let mut writes = ScopeSet::none();
        for (old, new) in MAPPINGS {
            reads = reads.with_exact(old);
            writes = writes.with_exact(new);
        }
        TaskDecl::new("convert_old_texture_paths", Tier::Eraser)
            .reads(reads)
            .writes(writes)
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let mut changed = 0;
        let mut notes = Vec::new();
        for (old, new) in MAPPINGS {
            if tx.exists(old) {
                tx.alias(old, new)?;
                changed += 1;
                notes.push(format!("{old} -> {new}"));
            }
        }
        Ok(Outcome {
            changed,
            notes,
            ..Outcome::default()
        })
    }
