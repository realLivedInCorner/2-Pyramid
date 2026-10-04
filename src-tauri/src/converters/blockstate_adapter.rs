use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

pub fn generate_blockstates(assets_path: &Path) -> io::Result<()> {
    let blockstates_dir = assets_path.join("minecraft/blockstates");
    if !blockstates_dir.exists() {
        fs::create_dir_all(&blockstates_dir)?;
    }

    crate::log_info!("generating 1.13+ compatible blockstates");

    let simple_blocks = vec![
        ("grass_block", "grass_block"),
        ("stone", "stone"),
        ("dirt", "dirt"),
        ("oak_planks", "oak_planks"),
        ("cobblestone", "cobblestone"),
    ];

    for (block_name, model_name) in simple_blocks {
        let file_path = blockstates_dir.join(format!("{}.json", block_name));
        if !file_path.exists() {
            let content = json!({
                "variants": {
                    "": { "model": format!("minecraft:block/{}", model_name) }
                }
            });
            write_json_file(&file_path, &content)?;
        }
    }

    let colors = vec![
        "white",
        "orange",
        "magenta",
        "light_blue",
        "yellow",
        "lime",
        "pink",
        "gray",
        "light_gray",
        "cyan",
        "purple",
        "blue",
        "brown",
        "green",
        "red",
        "black",
    ];

    for color in colors {
        let wool_name = format!("{}_wool", color);
        let wool_path = blockstates_dir.join(format!("{}.json", wool_name));

        if !wool_path.exists() {
            let wool_json = json!({
                "variants": {
                    "": { "model": format!("minecraft:block/{}", wool_name) }
                }
            });
            write_json_file(&wool_path, &wool_json)?;
        }
    }

    Ok(())
}

fn write_json_file(path: &PathBuf, content: &Value) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    let buf = serde_json::to_string_pretty(content)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    file.write_all(buf.as_bytes())?;
    Ok(())
}

pub fn fix_complex_states(_assets_path: &Path) -> io::Result<()> {
    Ok(())
}
