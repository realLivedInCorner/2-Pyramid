// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;

/// 结构分析 CLI：`2-pyramid.exe --analyze <zip | 目录>`（只读，不启动 GUI）。
///
/// 用于「多版本资源包」调研阶段批量跑真实样本：
///   * 传 zip  → 输出该包的分析 JSON；
///   * 传目录  → 递归找出目录下所有 .zip / .mcpack，逐个分析并输出 JSON 数组。
///
/// 退出码：0 = 全部成功；1 = 至少一个失败。
fn run_analyze_cli(path: &str) -> i32 {
    use two_pyramid_lib::{analyze_dir, analyze_zip};

    let target = std::path::Path::new(path);
    if !target.exists() {
        eprintln!("路径不存在: {}", path);
        return 1;
    }

    if target.is_file() {
        return match analyze_zip(target) {
            Ok(report) => {
                println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
                eprintln!("[analysis] {}", report.summary());
                0
            }
            Err(e) => {
                eprintln!("分析失败: {}", e);
                1
            }
        };
    }

    // 目录：收集所有压缩包（含子目录）
    let mut packs: Vec<std::path::PathBuf> = Vec::new();
    for entry in walkdir::WalkDir::new(target).into_iter().filter_map(Result::ok) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let ext = p
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if ext == "zip" || ext == "mcpack" {
            packs.push(p.to_path_buf());
        }
    }
    packs.sort();

    if packs.is_empty() {
        // 目录本身可能就是一个解压后的包
        return match analyze_dir(target) {
            Ok(report) => {
                println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
                0
            }
            Err(e) => {
                eprintln!("分析失败: {}", e);
                1
            }
        };
    }

    let mut failures = 0usize;
    let mut reports = Vec::new();
    for p in &packs {
        match analyze_zip(p) {
            Ok(report) => {
                eprintln!("[analysis] {} -> {}", p.display(), report.summary());
                reports.push(report);
            }
            Err(e) => {
                failures += 1;
                eprintln!("[analysis] {} -> 失败: {}", p.display(), e);
            }
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&reports).unwrap_or_default()
    );
    eprintln!(
        "[analysis] 共 {} 个包，成功 {}，失败 {}",
        packs.len(),
        packs.len() - failures,
        failures
    );
    if failures > 0 { 1 } else { 0 }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // （右键菜单静默转换入口已移除 —— 安装器不再注册 .zip 右键菜单）

    // 结构分析 CLI：只读分析，不启动 GUI
    if let Some(idx) = args.iter().position(|a| a == "--analyze") {
        let target = args.get(idx + 1).cloned().unwrap_or_default();
        if target.is_empty() {
            eprintln!("用法: 2-pyramid.exe --analyze <zip 或目录>");
            std::process::exit(2);
        }
        std::process::exit(run_analyze_cli(&target));
    }

    // 检查是否有 --nogui 参数
    if args.contains(&"--nogui".to_string()) {
        // 启动无头模式
        let ver = env!("CARGO_PKG_VERSION");
        let build = env!("BUILD_NUMBER");
        let stamp = format!("{}+{}", ver, build);
        eprintln!("[2-Pyramid {}]Checking Environment.....", stamp);
        eprintln!("[2-Pyramid {}]Checked Successfully, Starting 2-Pyramid Hurray Engine", stamp);
        eprintln!("[2-Pyramid {}]Processing NGUI Conifg", stamp);
        eprintln!("[2-Pyramid {}]Loaded Setting Module", stamp);
        eprintln!("[2-Pyramid {}]Loaded Minecraft ResourcePack Convert Module", stamp);
        eprintln!("[2-Pyramid {}]All Prepared", stamp);
        eprintln!("[2-Pyramid {}]Launched 2-Pyramid {} NGUI Successfully", stamp, stamp);
        eprintln!("Time:%time_total_start%");
        eprintln!("Help Doc");
        eprintln!("--convert [location] [Version]");
        eprintln!("--setting");
        eprintln!("--exit");
        eprintln!("");
        eprintln!("Welcome To 2-Pyramid！（NGUI Demo）");
        eprintln!("Please type your command");
        eprintln!(">>");

        // 这里可以添加命令行交互逻辑
    } else {
        // 正常启动 GUI 模式
        two_pyramid_lib::run()
    }
}
