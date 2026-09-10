//! 路径别名与语言键映射（无 IO）。

/// Java stem → Bedrock stem。None 表示不改名。
pub fn java_to_bedrock_stem(stem: &str) -> Option<String> {
    match stem {
        "golden_apple" => return Some("apple_golden".into()),
        "golden_carrot" => return Some("carrot_golden".into()),
        "golden_horse_armor" => return Some("horsearmor_gold".into()),
        "iron_horse_armor" => return Some("horsearmor_iron".into()),
        "diamond_horse_armor" => return Some("horsearmor_diamond".into()),
        "leather_horse_armor" => return Some("horsearmor_leather".into()),
        "recovery_compass" => return Some("compass_recovery".into()),
        _ => {}
    }
    if let Some(rest) = stem.strip_prefix("netherite_") {
        return Some(format!("{}_netherite", rest));
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
    None
}

/// Bedrock stem → Java stem。
pub fn bedrock_to_java_stem(stem: &str) -> Option<String> {
    match stem {
        "apple_golden" => return Some("golden_apple".into()),
        "carrot_golden" => return Some("golden_carrot".into()),
        "horsearmor_gold" => return Some("golden_horse_armor".into()),
        "horsearmor_iron" => return Some("iron_horse_armor".into()),
        "horsearmor_diamond" => return Some("diamond_horse_armor".into()),
        "horsearmor_leather" => return Some("leather_horse_armor".into()),
        "compass_recovery" => return Some("recovery_compass".into()),
        _ => {}
    }
    if let Some(rest) = stem.strip_prefix("record_") {
        if rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !rest.is_empty() {
            return Some(format!("music_disc_{}", rest));
        }
    }
    if let Some(rest) = stem.strip_suffix("_netherite") {
        if !rest.is_empty() {
            return Some(format!("netherite_{}", rest));
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
        assert_eq!(java_to_bedrock_stem("netherite_sword").as_deref(), Some("sword_netherite"));
        assert_eq!(bedrock_to_java_stem("sword_netherite").as_deref(), Some("netherite_sword"));
        assert_eq!(java_to_bedrock_stem("wooden_sword").as_deref(), Some("wood_sword"));
        assert_eq!(bedrock_to_java_stem("wood_sword").as_deref(), Some("wooden_sword"));
        assert_eq!(java_to_bedrock_stem("stone"), None);
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
