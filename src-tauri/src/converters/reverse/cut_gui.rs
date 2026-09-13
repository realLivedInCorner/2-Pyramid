use std::path::Path;

/// cut_gui extracts sprites from atlas textures and deletes the originals.
/// Reconstructing the atlases from individual sprites is not feasible without
/// the exact layout metadata. This is a no-op.
pub fn reverse_cut_gui(_path: &Path) -> Result<(), String> {
    Ok(())
}
