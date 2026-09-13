use std::path::Path;

/// overlay_icons alpha-blends external icons onto gui/icons.png — the original pixel data
/// is permanently overwritten and cannot be recovered.
pub fn reverse_overlay_icons(_path: &Path) -> Result<(), String> {
    Ok(())
}
