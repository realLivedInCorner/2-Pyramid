    use super::rename_blocks::{BLOCK, BLOCKS, ITEM, ITEMS, TEXTURES};
    use super::rename_blocks_tables::REVERSE_PAIRS;
    use super::*;

    pub fn decl() -> TaskDecl {
        TaskDecl::new("reverse_rename_blocks_items", Tier::Eraser)
            .reads(ScopeSet::prefix(TEXTURES))
            .writes(ScopeSet::prefix(TEXTURES))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        let mut outcome = Outcome::default();
        if tx.has_prefix(ITEM)? && !tx.has_prefix(ITEMS)? {
            super::rename_blocks::merge_or_rename_dir(tx, ITEM, ITEMS)?;
            outcome.changed += 1;
            outcome.notes.push("item -> items".into());
        }
        if tx.has_prefix(BLOCK)? && !tx.has_prefix(BLOCKS)? {
            super::rename_blocks::merge_or_rename_dir(tx, BLOCK, BLOCKS)?;
            outcome.changed += 1;
            outcome.notes.push("block -> blocks".into());
        }
        for (old, new) in REVERSE_PAIRS {
            if super::rename_blocks::rename_with_mcmeta(tx, ITEMS, old, new)? {
                outcome.changed += 1;
            }
        }
        Ok(outcome)
    }
