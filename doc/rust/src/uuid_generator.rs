//! UUID Generator Module
//! 
//! 专门处理UUID生成的模块

use uuid::Uuid;

/// UUID生成器
pub struct UuidGenerator {
    // 可以添加配置选项，比如是否使用v4 UUID
    use_v4: bool,
}

impl UuidGenerator {
    /// 创建新的UUID生成器实例
    pub fn new() -> Self {
        Self {
            use_v4: true,
        }
    }

    /// 创建指定UUID版本的生成器
    pub fn new_with_version(use_v4: bool) -> Self {
        Self { use_v4 }
    }

    /// 生成随机UUID字符串
    pub fn generate_uuid(&self) -> String {
        if self.use_v4 {
            Uuid::new_v4().to_string()
        } else {
            // 如果需要其他版本的UUID，可以在这里扩展
            Uuid::new_v4().to_string()
        }
    }

    /// 生成多个UUID
    pub fn generate_multiple(&self, count: usize) -> Vec<String> {
        (0..count)
            .map(|_| self.generate_uuid())
            .collect()
    }

    /// 验证UUID格式
    pub fn validate_uuid(uuid_str: &str) -> bool {
        Uuid::parse_str(uuid_str).is_ok()
    }

    /// 从字符串解析UUID
    pub fn parse_uuid(uuid_str: &str) -> Result<Uuid, String> {
        match Uuid::parse_str(uuid_str) {
            Ok(uuid) => Ok(uuid),
            Err(e) => Err(format!("UUID解析失败: {}", e)),
        }
    }

    /// 生成短UUID（去掉连字符）
    pub fn generate_short_uuid(&self) -> String {
        self.generate_uuid().replace('-', "")
    }

    /// 生成带前缀的UUID
    pub fn generate_prefixed_uuid(&self, prefix: &str) -> String {
        format!("{}_{}", prefix, self.generate_uuid())
    }

    /// 检查UUID是否为nil UUID（全零）
    pub fn is_nil_uuid(uuid_str: &str) -> bool {
        match Uuid::parse_str(uuid_str) {
            Ok(uuid) => uuid.is_nil(),
            Err(_) => false,
        }
    }

    /// 获取UUID版本信息
    pub fn get_uuid_version(uuid_str: &str) -> Option<u8> {
        match Uuid::parse_str(uuid_str) {
            Ok(uuid) => Some(uuid.get_version().map(|v| v as u8).unwrap_or(0)),
            Err(_) => None,
        }
    }
}

impl Default for UuidGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_generation() {
        let generator = UuidGenerator::new();
        let uuid = generator.generate_uuid();
        
        assert_eq!(uuid.len(), 36); // 标准UUID格式长度
        assert!(uuid.contains('-')); // 包含连字符
        assert!(UuidGenerator::validate_uuid(&uuid));
    }

    #[test]
    fn test_multiple_uuid_generation() {
        let generator = UuidGenerator::new();
        let uuids = generator.generate_multiple(3);
        
        assert_eq!(uuids.len(), 3);
        for uuid in uuids {
            assert!(UuidGenerator::validate_uuid(&uuid));
        }
        
        // 确保生成的UUID不重复
        assert_ne!(uuids[0], uuids[1]);
        assert_ne!(uuids[1], uuids[2]);
        assert_ne!(uuids[0], uuids[2]);
    }

    #[test]
    fn test_short_uuid() {
        let generator = UuidGenerator::new();
        let short_uuid = generator.generate_short_uuid();
        
        assert_eq!(short_uuid.len(), 32); // 去掉连字符后的长度
        assert!(!short_uuid.contains('-'));
    }

    #[test]
    fn test_prefixed_uuid() {
        let generator = UuidGenerator::new();
        let prefixed_uuid = generator.generate_prefixed_uuid("test");
        
        assert!(prefixed_uuid.starts_with("test_"));
        assert!(prefixed_uuid.len() > 5); // 至少包含前缀和部分UUID
    }

    #[test]
    fn test_uuid_validation() {
        let generator = UuidGenerator::new();
        let valid_uuid = generator.generate_uuid();
        let invalid_uuids = vec![
            "invalid",
            "12345",
            "",
            "not-a-uuid",
        ];
        
        assert!(UuidGenerator::validate_uuid(&valid_uuid));
        
        for invalid in invalid_uuids {
            assert!(!UuidGenerator::validate_uuid(invalid));
        }
    }

    #[test]
    fn test_nil_uuid_check() {
        assert!(UuidGenerator::is_nil_uuid("00000000-0000-0000-0000-000000000000"));
        assert!(!UuidGenerator::is_nil_uuid("12345678-1234-1234-1234-123456789012"));
        assert!(!UuidGenerator::is_nil_uuid("invalid"));
    }

    #[test]
    fn test_uuid_parsing() {
        let generator = UuidGenerator::new();
        let original_uuid = generator.generate_uuid();
        
        match UuidGenerator::parse_uuid(&original_uuid) {
            Ok(uuid) => assert_eq!(uuid.to_string(), original_uuid),
            Err(_) => panic!("UUID解析应该成功"),
        }
        
        match UuidGenerator::parse_uuid("invalid") {
            Ok(_) => panic!("无效UUID应该解析失败"),
            Err(_) => {}, // 期望的错误
        }
    }

    #[test]
    fn test_uuid_version_detection() {
        let generator = UuidGenerator::new();
        let uuid = generator.generate_uuid();
        
        if let Some(version) = UuidGenerator::get_uuid_version(&uuid) {
            assert_eq!(version, 4); // v4 UUID
        }
    }
}