// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    // 右键菜单静默转换: 2-pyramid.exe --convert "path\to\file.zip" --format N
    let convert_idx = args.iter().position(|a| a == "--convert");
    let format_idx = args.iter().position(|a| a == "--format");

    if let (Some(ci), Some(fi)) = (convert_idx, format_idx) {
        if ci + 1 < args.len() && fi + 1 < args.len() {
            let file_path = args[ci + 1].clone();
            if let Ok(format_num) = args[fi + 1].parse::<u32>() {
                two_pyramid_lib::run_silent(file_path, format_num);
                return;
            }
        }
    }

    // 检查是否有 --nogui 参数
    if args.contains(&"--nogui".to_string()) {
        // 启动无头模式
        eprintln!("[2-Pyramid Dev2.0.0]Checking Environment.....");
        eprintln!("[2-Pyramid Dev2.0.0]Checked Successfully, Starting 2-Pyramid Hurray Engine");
        eprintln!("[2-Pyramid Dev2.0.0]Processing NGUI Conifg");
        eprintln!("[2-Pyramid Dev2.0.0]Loaded Setting Module");
        eprintln!("[2-Pyramid Dev2.0.0]Loaded Minecraft ResourcePack Convert Module");
        eprintln!("[2-Pyramid Dev2.0.0]All Prepared");
        eprintln!("[2-Pyramid Dev2.0.0]Launched 2-Pyramid Dev2.0.0 NGUI Successfully");
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
