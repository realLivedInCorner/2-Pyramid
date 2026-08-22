//! Manifest Generator Module
//! 
//! 专门处理manifest.json文件生成的模块

use serde::{Deserialize, Serialize};
use crate::uuid_generator::UuidGenerator;
use crate::utils::{Utils, LogLevel};

/// 基岩版材质包清单文件结构
#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub format_version: u32,
    pub header: ManifestHeader,
    pub modules: Vec<ManifestModule>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ManifestHeader {
    pub description: String,
    pub name: String,
    pub uuid: String,
    pub version: [u32; 3],
    pub min_engine_version: [u32; 3],
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ManifestModule {
    pub description: String,
    pub r#type: String,
    pub uuid: String,
    pub version: [u32; 3],
}

/// Manifest生成器
pub struct ManifestGenerator {
    uuid_generator: UuidGenerator,
}

impl ManifestGenerator {
    /// 创建新的Manifest生成器实例
    pub fn new() -> Self {
        Self {
            uuid_generator: UuidGenerator::new(),
        }
    }

    /// 创建基岩版manifest.json文件内容
    pub fn create_bedrock_manifest(&self, pack_name: &str, original_description: &str) -> Manifest {
        let manifest = Manifest {
            format_version: 2,
            header: ManifestHeader {
                description: "KPT Converted".to_string(),
                name: pack_name.to_string(),
                uuid: self.uuid_generator.generate_uuid(),
                version: [1, 0, 0],
                min_engine_version: [1, 16, 2],
            },
            modules: vec![ManifestModule {
                description: original_description.to_string(),
                r#type: "resources".to_string(),
                uuid: self.uuid_generator.generate_uuid(),
                version: [1, 0, 0],
            }],
        };

        Utils::log_message(&format!("已生成manifest.json for pack: {}", pack_name), LogLevel::Info);
        manifest
    }

    /// 创建JSON字符串格式的manifest
    pub fn create_bedrock_manifest_json(&self, pack_name: &str, original_description: &str) -> String {
        let manifest = self.create_bedrock_manifest(pack_name, original_description);
        match serde_json::to_string_pretty(&manifest) {
            Ok(json_str) => {
                Utils::log_message("manifest.json生成成功", LogLevel::Info);
                json_str
            }
            Err(e) => {
                Utils::log_message(&format!("manifest.json生成失败: {}", e), LogLevel::Error);
                // 返回错误时的默认manifest
                r#"{
  "format_version": 2,
  "header": {
    "description": "KPT Converted",
    "name": "Default Pack",
    "uuid": "00000000-0000-0000-0000-000000000000",
    "version": [1, 0, 0],
    "min_engine_version": [1, 16, 2]
  },
  "modules": [
    {
      "description": "KPT Converted Pack",
      "type": "resources",
      "uuid": "00000000-0000-0000-0000-000000000000",
      "version": [1, 0, 0]
    }
  ]
}"#.to_string()
            }
        }
    }

    /// 验证manifest格式是否正确
    pub fn validate_manifest(&self, manifest_json: &str) -> Result<Manifest, String> {
        match serde_json::from_str::<Manifest>(manifest_json) {
            Ok(manifest) => {
                // 基本验证
                if manifest.format_version != 2 {
                    return Err(format!("不支持的manifest版本: {}", manifest.format_version));
                }
                if manifest.header.uuid.is_empty() {
                    return Err("Header UUID不能为空".to_string());
                }
                if manifest.modules.is_empty() {
                    return Err("至少需要一个模块".to_string());
                }
                
                Utils::log_message("manifest.json验证通过", LogLevel::Info);
                Ok(manifest)
            }
            Err(e) => {
                let error_msg = format!("manifest.json格式错误: {}", e);
                Utils::log_message(&error_msg, LogLevel::Error);
                Err(error_msg)
            }
        }
    }

    /// 获取默认的基岩版manifest
    pub fn get_default_manifest() -> &'static str {
        r#"{
  "format_version": 2,
  "header": {
    "description": "KPT Converted",
    "name": "Default Pack",
    "uuid": "00000000-0000-0000-0000-000000000000",
    "version": [1, 0, 0],
    "min_engine_version": [1, 16, 2]
  },
  "modules": [
    {
      "description": "KPT Converted Pack",
      "type": "resources",
      "uuid": "00000000-0000-0000-0000-000000000000",
      "version": [1, 0, 0]
    }
  ]
}"#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_generation() {
        let generator = ManifestGenerator::new();
        let manifest = generator.create_bedrock_manifest("Test Pack", "Test Description");
        
        assert_eq!(manifest.format_version, 2);
        assert_eq!(manifest.header.name, "Test Pack");
        assert_eq!(manifest.header.description, "KPT Converted");
        assert!(!manifest.header.uuid.is_empty());
        assert_eq!(manifest.modules.len(), 1);
        assert_eq!(manifest.modules[0].r#type, "resources");
    }

    #[test]
    fn test_manifest_validation() {
        let generator = ManifestGenerator::new();
        let manifest_json = r#"{
  "format_version": 2,
  "header": {
    "description": "Test",
    "name": "Test Pack",
    "uuid": "12345678-1234-1234-1234-123456789012",
    "version": [1, 0, 0],
    "min_engine_version": [1, 16, 2]
  },
  "modules": [
    {
      "description": "Test Module",
      "type": "resources",
      "uuid": "87654321-4321-4321-4321-210987654321",
      "version": [1, 0, 0]
    }
  ]
}"#;

        let result = generator.validate_manifest(manifest_json);
        assert!(result.is_ok());
        
        let manifest = result.unwrap();
        assert_eq!(manifest.format_version, 2);
        assert_eq!(manifest.header.name, "Test Pack");
    }
}