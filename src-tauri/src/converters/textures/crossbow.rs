use std::fs;
use std::path::Path;

use image::imageops;

use crate::converters::get_uimage_path;

fn overlay_pair(base: &image::RgbaImage, overlay: &image::RgbaImage) -> image::RgbaImage {
    let mut combined = base.clone();
    imageops::overlay(&mut combined, overlay, 0, 0);
    combined
}

pub fn generate_crossbow(resource_pack_path: &Path) -> Result<(), String> {
    let items_path = resource_pack_path.join("assets/minecraft/textures/item");
    let uimage_path = match get_uimage_path() {
        Ok(p) => p,
        Err(_) => {
            crate::log_info!("UImage path not available, skip crossbow generation");
            return Ok(());
        }
    };
    let crossbow_dir = uimage_path.join("crossbow");

    let size_to_path = [
        (16, crossbow_dir.join("crossbow_16.png")),
        (32, crossbow_dir.join("crossbow_32.png")),
        (64, crossbow_dir.join("crossbow_64.png")),
        (128, crossbow_dir.join("crossbow_128.png")),
        (256, crossbow_dir.join("crossbow_256.png")),
    ];

    let bow_path = items_path.join("bow.png");
    if bow_path.exists() {
        let bow_img = image::open(&bow_path)
            .map_err(|e| format!("failed to open {}: {}", bow_path.display(), e))?
            .to_rgba8();
        if let Some((_, base_path)) = size_to_path.iter().find(|(size, _)| *size == bow_img.width()) {
            if base_path.exists() {
                let base_img = image::open(base_path)
                    .map_err(|e| format!("failed to open {}: {}", base_path.display(), e))?
                    .to_rgba8();
                let standby = overlay_pair(&base_img, &bow_img);
                standby
                    .save(items_path.join("crossbow_standby.png"))
                    .map_err(|e| format!("failed to save crossbow_standby.png: {}", e))?;
            }
        }
    }

    let bow_pulling_files = [
        "bow_pulling_0.png",
        "bow_pulling_1.png",
        "bow_pulling_2.png",
        "bow_pulling_2.png",
    ];
    let crossbow_files = [
        "crossbow_pulling_0.png",
        "crossbow_pulling_1.png",
        "crossbow_pulling_2.png",
        "crossbow_arrow.png",
    ];

    let bow_pulling0 = items_path.join("bow_pulling_0.png");
    if bow_pulling0.exists() {
        let sample_img = image::open(&bow_pulling0)
            .map_err(|e| format!("failed to open {}: {}", bow_pulling0.display(), e))?
            .to_rgba8();
        if let Some((_, base_path)) = size_to_path.iter().find(|(size, _)| *size == sample_img.width()) {
            if base_path.exists() {
                let base_img = image::open(base_path)
                    .map_err(|e| format!("failed to open {}: {}", base_path.display(), e))?
                    .to_rgba8();

                for (bow_file, crossbow_file) in bow_pulling_files.iter().zip(crossbow_files.iter()) {
                    let bow_file_path = items_path.join(bow_file);
                    if !bow_file_path.exists() {
                        continue;
                    }
                    let bow_img = image::open(&bow_file_path)
                        .map_err(|e| format!("failed to open {}: {}", bow_file_path.display(), e))?
                        .to_rgba8();
                    let output_img = overlay_pair(&base_img, &bow_img);
                    let output_path = items_path.join(crossbow_file);
                    output_img
                        .save(&output_path)
                        .map_err(|e| format!("failed to save {}: {}", output_path.display(), e))?;

                    if *crossbow_file == "crossbow_arrow.png" {
                        let firework_path = items_path.join("crossbow_firework.png");
                        fs::copy(&output_path, &firework_path)
                            .map_err(|e| format!("failed to copy to crossbow_firework.png: {}", e))?;
                        let overlay_path = crossbow_dir.join(format!("crossbow_firework_{}.png", sample_img.width()));
                        if overlay_path.exists() {
                            let overlay_img = image::open(&overlay_path)
                                .map_err(|e| format!("failed to open {}: {}", overlay_path.display(), e))?
                                .to_rgba8();
                            let mut firework_img = image::open(&firework_path)
                                .map_err(|e| format!("failed to open {}: {}", firework_path.display(), e))?
                                .to_rgba8();
                            let overlay_img = if overlay_img.dimensions() != firework_img.dimensions() {
                                imageops::resize(
                                    &overlay_img,
                                    firework_img.width(),
                                    firework_img.height(),
                                    imageops::FilterType::Triangle,
                                )
                            } else {
                                overlay_img
                            };
                            imageops::overlay(&mut firework_img, &overlay_img, 0, 0);
                            firework_img
                                .save(&firework_path)
                                .map_err(|e| format!("failed to save {}: {}", firework_path.display(), e))?;
                        }
                    }
                }
            }
        }
    }

    crate::log_info!("generated crossbow textures");
    Ok(())
}

pub fn register_task(engine: &mut crate::hurray::engine::HurrayEngine) {
    engine.register_task(
        "generate_crossbow",
        crate::hurray::scheduler::TaskType::Parallel,
        crate::hurray::scheduler::TaskTier::Architect,
        |context| generate_crossbow(context.temp_dir()),
    );
}
