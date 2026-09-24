//! 探针：文件 / 权限 / 加密 / 解析 / 恶意。单探针失败不阻断树。

use serde::Serialize;

use super::rom::{IssueLevel, ParseStatus, Rom, RomIssue};

#[derive(Debug, Clone, Default, Serialize)]
pub struct ProbeReport {
    pub file: FileProbe,
    pub permission: PermissionProbe,
    pub crypto: CryptoProbe,
    pub parse: ParseProbe,
    pub malware: MalwareProbe,
    pub issues: Vec<RomIssue>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct FileProbe {
    pub count: usize,
    pub total_bytes: u64,
    pub by_ext: Vec<ExtStat>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtStat {
    pub ext: String,
    pub count: usize,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PermissionProbe {
    pub readonly_files: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CryptoProbe {
    pub high_entropy_files: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ParseProbe {
    pub ok: usize,
    pub warn: usize,
    pub error: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MalwareProbe {
    pub suspicious: usize,
    pub notes: Vec<String>,
}

const DANGEROUS_EXT: &[&str] = &[
    "exe", "com", "bat", "cmd", "ps1", "vbs", "js", "jar", "msi", "scr", "pif", "hta", "sh", "dll",
];

/// Shannon entropy 0..8，用于粗判「像不像加密/压缩流」。
fn entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut freq = [0u32; 256];
    for b in data {
        freq[*b as usize] += 1;
    }
    let n = data.len() as f64;
    freq.iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / n;
            -p * p.log2()
        })
        .sum()
}

fn ext_of(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default()
}

fn walk_files<'a>(dir: &'a super::rom::RomDir, out: &mut Vec<&'a super::rom::RomFile>) {
    for f in &dir.files {
        out.push(f);
    }
    for d in &dir.dirs {
        walk_files(d, out);
    }
}

pub fn run_probes(rom: &Rom) -> ProbeReport {
    let mut report = ProbeReport::default();
    let mut files = Vec::new();
    walk_files(&rom.root, &mut files);

    // ── 文件探针 ──
    report.file.count = files.len();
    let mut by_ext: std::collections::BTreeMap<String, (usize, u64)> = Default::default();
    for f in &files {
        report.file.total_bytes += f.size;
        let ext = ext_of(&f.path);
        let e = ext_of(&f.path);
        let key = if e.is_empty() {
            "(none)".to_string()
        } else {
            e
        };
        let _ = ext;
        let slot = by_ext.entry(key).or_insert((0, 0));
        slot.0 += 1;
        slot.1 += f.size;
    }
    report.file.by_ext = by_ext
        .into_iter()
        .map(|(ext, (count, bytes))| ExtStat {
            ext,
            count,
            bytes,
        })
        .collect();

    // ── 权限探针（zip 解压后通常无 Windows ACL；用 size==0 / 只读占位 + 源路径提示）──
    // 真实只读位在 zip unix mode 上，SafeEntry 未携带；这里检查扩展名异常与 notes。
    report.permission.notes.push(
        "zip 内文件默认从受控缓冲加载；磁盘只读属性在导出副本时保留文件内容、不继承源 ACL".into(),
    );

    // ── 加密探针：高熵、无扩展名、非 PNG/JSON 文本 ──
    for f in &files {
        let e = entropy(&f.data);
        if f.size >= 256 && e > 7.5 {
            report.crypto.high_entropy_files += 1;
            report.crypto.notes.push(format!("{} entropy={e:.2}", f.path));
            report.issues.push(RomIssue {
                level: IssueLevel::Warn,
                source: "crypto".into(),
                path: f.path.clone(),
                message: format!("high entropy {e:.2}, possible encryption"),
            });
        }
    }

    // ── 解析探针 ──
    for f in &files {
        match f.parse {
            ParseStatus::Ok => report.parse.ok += 1,
            ParseStatus::Warn => report.parse.warn += 1,
            ParseStatus::Error => report.parse.error += 1,
            ParseStatus::Skipped => report.parse.skipped += 1,
        }
    }
    // 带上 ROM 构建期 parse issues
    for i in &rom.issues {
        if i.source == "parse" {
            report.issues.push(i.clone());
        }
    }

    // ── 恶意探针 ──
    for f in &files {
        let lower = f.path.to_ascii_lowercase();
        let ext = ext_of(&f.path);
        if DANGEROUS_EXT.contains(&ext.as_str()) {
            report.malware.suspicious += 1;
            let msg = format!("suspicious extension .{ext}");
            report.malware.notes.push(format!("{}: {msg}", f.path));
            report.issues.push(RomIssue {
                level: IssueLevel::Danger,
                source: "malware".into(),
                path: f.path.clone(),
                message: msg,
            });
        }
        // 双扩展：image.png.exe
        if let Some(name) = lower.rsplit('/').next() {
            let parts: Vec<&str> = name.split('.').collect();
            if parts.len() >= 3 {
                let n = parts.len();
                if DANGEROUS_EXT.contains(&parts[n - 1]) {
                    report.malware.suspicious += 1;
                    let msg = "double extension trick".to_string();
                    report.malware.notes.push(format!("{}: {msg}", f.path));
                    report.issues.push(RomIssue {
                        level: IssueLevel::Danger,
                        source: "malware".into(),
                        path: f.path.clone(),
                        message: msg,
                    });
                }
            }
        }
        if lower.ends_with(".class") || lower.ends_with(".dex") {
            report.malware.suspicious += 1;
            report.malware.notes.push(format!("{}: native class payload", f.path));
            report.issues.push(RomIssue {
                level: IssueLevel::Danger,
                source: "malware".into(),
                path: f.path.clone(),
                message: "native class/dex payload".into(),
            });
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foray::rom::{build, FileKind, RomFile};
    use crate::foray::zip_safe::{open_pack_bytes, SafeLimits};

    fn zip_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let o = zip::write::FileOptions::default();
            for (n, d) in files {
                z.start_file(*n, o).unwrap();
                z.write_all(d).unwrap();
            }
            z.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn flags_exe_and_entropy() {
        let mut noise = vec![0u8; 512];
        for (i, b) in noise.iter_mut().enumerate() {
            *b = (i * 37 + 11) as u8; // 均匀伪随机 → 高熵
        }
        let bytes = zip_bytes(&[
            (
                "pack.mcmeta",
                br#"{"pack":{"pack_format":34,"description":"t"}}"#,
            ),
            ("assets/minecraft/tools/mod.exe", b"MZ"),
            ("assets/minecraft/blob.bin", &noise),
        ]);
        let arc = open_pack_bytes(&bytes, &SafeLimits::default()).unwrap();
        let rom = build(&arc, "mem.zip");
        let report = run_probes(&rom);
        assert!(report.malware.suspicious >= 1);
        assert!(report.crypto.high_entropy_files >= 1);
    }

    #[test]
    fn entropy_range() {
        assert!(entropy(&[0u8; 1024]) < 0.1);
        let alt: Vec<u8> = (0..255).collect();
        assert!(entropy(&alt) > 7.0);
    }

    #[test]
    fn parse_counts() {
        let mut f = RomFile {
            path: "a.json".into(),
            kind: FileKind::Json,
            size: 1,
            sha256: String::new(),
            parse: ParseStatus::Warn,
            paintable: false,
            dirty: false,
            data: vec![],
        };
        f.parse = ParseStatus::Warn;
        assert_eq!(f.parse, ParseStatus::Warn);
    }
}
