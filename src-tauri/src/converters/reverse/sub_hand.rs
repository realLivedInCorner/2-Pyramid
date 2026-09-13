use std::path::Path;

/// fix_ui_sub_hand copies pixel regions to widgets.png positions that overwrite original data.
/// The original pixels at (24,23) and (60,23) are lost — cannot be cleanly reversed.
pub fn reverse_fix_ui_sub_hand(_path: &Path) -> Result<(), String> {
    Ok(())
}
