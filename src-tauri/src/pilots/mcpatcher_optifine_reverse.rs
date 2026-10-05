    use super::rename_blocks::merge_or_rename_dir;
    use super::*;

    const OPTIFINE: &str = "assets/minecraft/optifine";
    const MCPATCHER: &str = "assets/minecraft/mcpatcher";

    pub fn decl() -> TaskDecl {
        TaskDecl::new("reverse_rename_mcpatcher_to_optifine", Tier::Eraser)
            .reads(ScopeSet::prefix("assets/minecraft"))
            .writes(ScopeSet::prefix("assets/minecraft"))
            .exclusive(true)
    }

    pub fn run(tx: &mut Tx<'_>) -> Result<Outcome, AromError> {
        if !tx.has_prefix(OPTIFINE)? || tx.has_prefix(MCPATCHER)? {
            return Ok(Outcome::default());
        }
        merge_or_rename_dir(tx, OPTIFINE, MCPATCHER)?;
        Ok(Outcome {
            changed: 1,
            notes: vec![format!("{OPTIFINE} -> {MCPATCHER}")],
            ..Outcome::default()
        })
    }
