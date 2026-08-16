// 图片 URL 解析：直接通过后端 `read_image_b64` 读取文件并返回
// data URL（data:image/<mime>;base64,...）。
//
// 不用 asset 协议的原因：asset scope / CSP / dev 与生产协议差异
// （http vs https）在多种环境组合下表现不一致，排查成本高；data
// URL 内联显示与这些完全无关，行为在任何环境下一致。背景图与
// 对话框预览均为低频操作（设置一次 + 启动加载一次），50MB 上限下
// 的 base64 传输开销可接受。

import { invoke } from '@tauri-apps/api/core';

export async function resolveImageUrl(path: string): Promise<string> {
  return await invoke<string>('read_image_b64', { path });
}
