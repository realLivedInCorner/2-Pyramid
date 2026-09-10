//! 路径别名与语言键映射（无 IO）。

/// Java stem → Bedrock stem。None 表示不改名。
pub fn java_to_bedrock_stem(stem: &str) -> Option<String> {
    // 药水：Java potion/splash_potion/lingering → Bedrock bottle 命名
    match stem {
        "potion" => return Some("potion_bottle_drinkable".into()),
        "splash_potion" => return Some("potion_bottle_splash".into()),
        "lingering_potion" => return Some("potion_bottle_lingering".into()),
        "glass_bottle" => return Some("potion_bottle_empty".into()),
        "golden_apple" => return Some("apple_golden".into()),
        "golden_carrot" => return Some("carrot_golden".into()),
        "golden_horse_armor" => return Some("horsearmor_gold".into()),
        "iron_horse_armor" => return Some("horsearmor_iron".into()),
        "diamond_horse_armor" => return Some("horsearmor_diamond".into()),
        "leather_horse_armor" => return Some("horsearmor_leather".into()),
        "recovery_compass" => return Some("recovery_compass_item".into()),
        "dragon_breath" => return Some("dragons_breath".into()),
        "slime_ball" => return Some("slimeball".into()),
        "totem_of_undying" => return Some("totem".into()),
        "heart_of_the_sea" => return Some("heartofthesea_closed".into()),
        "nautilus_shell" => return Some("nautilus".into()),
        // 草方块：Java grass_block_* → Bedrock grass_*
        "grass_block_side" => return Some("grass_side".into()),
        "grass_block_top" => return Some("grass_top".into()),
        "grass_block_side_overlay" => return Some("grass_side_overlay".into()),
        // 桶：Java water_bucket → Bedrock bucket_water
        "bucket" => return Some("bucket_empty".into()),
        "water_bucket" => return Some("bucket_water".into()),
        "lava_bucket" => return Some("bucket_lava".into()),
        "milk_bucket" => return Some("bucket_milk".into()),
        "powder_snow_bucket" => return Some("bucket_powder_snow".into()),
        "axolotl_bucket" => return Some("bucket_axolotl".into()),
        "tadpole_bucket" => return Some("bucket_tadpole".into()),
        "cod_bucket" => return Some("bucket_cod".into()),
        "salmon_bucket" => return Some("bucket_salmon".into()),
        "tropical_fish_bucket" => return Some("bucket_tropical".into()),
        "pufferfish_bucket" => return Some("bucket_pufferfish".into()),
        // 弓弩：Java bow.png → bow_standby；拉弓帧名一致
        "bow" => return Some("bow_standby".into()),
        "crossbow" => return Some("crossbow_standby".into()),
        _ => {}
    }

    // 彩色床：{color}_bed → bed_{color}
    const BED_COLORS: &[&str] = &[
        "white",
        "orange",
        "magenta",
        "light_blue",
        "yellow",
        "lime",
        "pink",
        "gray",
        "light_gray",
        "silver",
        "cyan",
        "purple",
        "blue",
        "brown",
        "green",
        "red",
        "black",
    ];
    for c in BED_COLORS {
        if stem == format!("{}_bed", c) {
            return Some(format!("bed_{}", c));
        }
    }

    if let Some(rest) = stem.strip_prefix("music_disc_") {
        return Some(format!("record_{}", rest));
    }
    if let Some(rest) = stem.strip_prefix("golden_") {
        return Some(format!("gold_{}", rest));
    }
    if let Some(rest) = stem.strip_prefix("wooden_") {
        return Some(format!("wood_{}", rest));
    }
    // 注意：netherite_* 在 Bedrock 与 Java 同名，勿改写为 *_netherite
    None
}

/// Bedrock stem → Java stem。
pub fn bedrock_to_java_stem(stem: &str) -> Option<String> {
    match stem {
        "potion_bottle_drinkable" => return Some("potion".into()),
        "potion_bottle_splash" => return Some("splash_potion".into()),
        "potion_bottle_lingering" => return Some("lingering_potion".into()),
        "potion_bottle_empty" => return Some("glass_bottle".into()),
        "apple_golden" => return Some("golden_apple".into()),
        "carrot_golden" => return Some("golden_carrot".into()),
        "horsearmor_gold" => return Some("golden_horse_armor".into()),
        "horsearmor_iron" => return Some("iron_horse_armor".into()),
        "horsearmor_diamond" => return Some("diamond_horse_armor".into()),
        "horsearmor_leather" => return Some("leather_horse_armor".into()),
        "recovery_compass_item" | "compass_recovery" => return Some("recovery_compass".into()),
        "dragons_breath" => return Some("dragon_breath".into()),
        "slimeball" => return Some("slime_ball".into()),
        "totem" => return Some("totem_of_undying".into()),
        "heartofthesea_closed" => return Some("heart_of_the_sea".into()),
        "nautilus" => return Some("nautilus_shell".into()),
        "grass_side" => return Some("grass_block_side".into()),
        "grass_top" => return Some("grass_block_top".into()),
        "grass_side_overlay" => return Some("grass_block_side_overlay".into()),
        "bucket_empty" => return Some("bucket".into()),
        "bucket_water" => return Some("water_bucket".into()),
        "bucket_lava" => return Some("lava_bucket".into()),
        "bucket_milk" => return Some("milk_bucket".into()),
        "bucket_powder_snow" => return Some("powder_snow_bucket".into()),
        "bucket_axolotl" => return Some("axolotl_bucket".into()),
        "bucket_tadpole" => return Some("tadpole_bucket".into()),
        "bucket_cod" => return Some("cod_bucket".into()),
        "bucket_salmon" => return Some("salmon_bucket".into()),
        "bucket_tropical" => return Some("tropical_fish_bucket".into()),
        "bucket_pufferfish" => return Some("pufferfish_bucket".into()),
        "bow_standby" => return Some("bow".into()),
        "crossbow_standby" => return Some("crossbow".into()),
        _ => {}
    }

    const BED_COLORS: &[&str] = &[
        "white",
        "orange",
        "magenta",
        "light_blue",
        "yellow",
        "lime",
        "pink",
        "gray",
        "silver",
        "cyan",
        "purple",
        "blue",
        "brown",
        "green",
        "red",
        "black",
    ];
    for c in BED_COLORS {
        if stem == format!("bed_{}", c) {
            let java_c = if *c == "silver" { "light_gray" } else { *c };
            return Some(format!("{}_bed", java_c));
        }
    }

    if let Some(rest) = stem.strip_prefix("record_") {
        if rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !rest.is_empty() {
            return Some(format!("music_disc_{}", rest));
        }
    }
    if let Some(rest) = stem.strip_prefix("gold_") {
        return Some(format!("golden_{}", rest));
    }
    if let Some(rest) = stem.strip_prefix("wood_") {
        return Some(format!("wooden_{}", rest));
    }
    None
}

/// `en_us` ↔ `en_US`
pub fn normalize_bedrock_lang_code(java_code: &str) -> String {
    if let Some((lang, region)) = java_code.split_once('_') {
        format!("{}_{}", lang, region.to_ascii_uppercase())
    } else {
        java_code.to_string()
    }
}

pub fn normalize_java_lang_code(bedrock_code: &str) -> String {
    if let Some((lang, region)) = bedrock_code.split_once('_') {
        format!("{}_{}", lang, region.to_ascii_lowercase())
    } else {
        bedrock_code.to_ascii_lowercase()
    }
}

/// Java lang key → Bedrock lang key（尽力前缀映射，其余原样）。
pub fn java_lang_key_to_bedrock(key: &str) -> String {
    if let Some(rest) = key.strip_prefix("block.minecraft.") {
        return format!("tile.{}.name", rest);
    }
    if let Some(rest) = key.strip_prefix("item.minecraft.") {
        return format!("item.{}.name", rest);
    }
    if let Some(rest) = key.strip_prefix("entity.minecraft.") {
        return format!("entity.{}.name", rest);
    }
    if let Some(rest) = key.strip_prefix("enchantment.minecraft.") {
        return format!("enchantment.{}.name", rest);
    }
    key.to_string()
}

/// Bedrock lang key → Java lang key。无法识别时原样透传。
pub fn bedrock_lang_key_to_java(key: &str) -> Option<String> {
    if let Some(rest) = key.strip_prefix("tile.") {
        if let Some(name) = rest.strip_suffix(".name") {
            return Some(format!("block.minecraft.{}", name));
        }
    }
    if let Some(rest) = key.strip_prefix("item.") {
        if let Some(name) = rest.strip_suffix(".name") {
            if !name.contains('.') {
                return Some(format!("item.minecraft.{}", name));
            }
        }
    }
    if let Some(rest) = key.strip_prefix("entity.") {
        if let Some(name) = rest.strip_suffix(".name") {
            if !name.contains('.') {
                return Some(format!("entity.minecraft.{}", name));
            }
        }
    }
    if let Some(rest) = key.strip_prefix("enchantment.") {
        if let Some(name) = rest.strip_suffix(".name") {
            if !name.contains('.') {
                return Some(format!("enchantment.minecraft.{}", name));
            }
        }
    }
    Some(key.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stem_aliases_roundtrip_core() {
        assert_eq!(java_to_bedrock_stem("golden_apple").as_deref(), Some("apple_golden"));
        assert_eq!(bedrock_to_java_stem("apple_golden").as_deref(), Some("golden_apple"));
        assert_eq!(java_to_bedrock_stem("music_disc_13").as_deref(), Some("record_13"));
        assert_eq!(bedrock_to_java_stem("record_13").as_deref(), Some("music_disc_13"));
        // netherite 与 Bedrock 同名，不得改写
        assert_eq!(java_to_bedrock_stem("netherite_sword"), None);
        assert_eq!(java_to_bedrock_stem("wooden_sword").as_deref(), Some("wood_sword"));
        assert_eq!(bedrock_to_java_stem("wood_sword").as_deref(), Some("wooden_sword"));
        assert_eq!(java_to_bedrock_stem("stone"), None);
    }

    #[test]
    fn test_visible_pack_renames() {
        // 药水
        assert_eq!(
            java_to_bedrock_stem("potion").as_deref(),
            Some("potion_bottle_drinkable")
        );
        assert_eq!(
            java_to_bedrock_stem("splash_potion").as_deref(),
            Some("potion_bottle_splash")
        );
        assert_eq!(
            java_to_bedrock_stem("lingering_potion").as_deref(),
            Some("potion_bottle_lingering")
        );
        // 草方块
        assert_eq!(java_to_bedrock_stem("grass_block_side").as_deref(), Some("grass_side"));
        assert_eq!(java_to_bedrock_stem("grass_block_top").as_deref(), Some("grass_top"));
        // 床
        assert_eq!(java_to_bedrock_stem("white_bed").as_deref(), Some("bed_white"));
        assert_eq!(java_to_bedrock_stem("light_gray_bed").as_deref(), Some("bed_light_gray"));
        assert_eq!(bedrock_to_java_stem("bed_red").as_deref(), Some("red_bed"));
        // 合金/材料
        assert_eq!(java_to_bedrock_stem("golden_sword").as_deref(), Some("gold_sword"));
        assert_eq!(java_to_bedrock_stem("totem_of_undying").as_deref(), Some("totem"));
        assert_eq!(java_to_bedrock_stem("slime_ball").as_deref(), Some("slimeball"));
        // 桶 / 弓
        assert_eq!(java_to_bedrock_stem("water_bucket").as_deref(), Some("bucket_water"));
        assert_eq!(java_to_bedrock_stem("bucket").as_deref(), Some("bucket_empty"));
        assert_eq!(java_to_bedrock_stem("bow").as_deref(), Some("bow_standby"));
        assert_eq!(java_to_bedrock_stem("crossbow").as_deref(), Some("crossbow_standby"));
    }

    #[test]
    fn test_lang_code_and_keys() {
        assert_eq!(normalize_bedrock_lang_code("zh_cn"), "zh_CN");
        assert_eq!(normalize_java_lang_code("zh_CN"), "zh_cn");
        assert_eq!(java_lang_key_to_bedrock("block.minecraft.stone"), "tile.stone.name");
        assert_eq!(
            bedrock_lang_key_to_java("tile.stone.name").as_deref(),
            Some("block.minecraft.stone")
        );
        assert_eq!(
            bedrock_lang_key_to_java("menu.play").as_deref(),
            Some("menu.play")
        );
    }
}
