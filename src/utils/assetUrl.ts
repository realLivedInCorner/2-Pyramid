// 图片 URL 解析：优先使用 Tauri asset 协议（convertFileSrc），
// 若加载失败（asset scope 未覆盖该路径 / CSP 拦截 / dev 模式协议
// 差异），回退到后端 `read_image_bytes` 读取字节并生成 Blob URL。

import { invoke, convertFileSrc } from '@tauri-apps/api/core';

const MIME_BY_EXT: Record<string, string> = {
  png: 'image/png',
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  webp: 'image/webp',
  gif: 'image/gif',
  bmp: 'image/bmp',
};

function probeLoad(url: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve();
    img.onerror = () => reject(new Error('image failed to load'));
    img.src = url;
  });
}

export async function resolveImageUrl(path: string): Promise<string> {
  const assetUrl = convertFileSrc(path);
  try {
    await probeLoad(assetUrl);
    return assetUrl;
  } catch {
    // asset 协议不可用 → 后端读字节 → Blob URL
    const ext = path.split('.').pop()?.toLowerCase() ?? '';
    const bytes = await invoke<number[]>('read_image_bytes', { path });
    const blob = new Blob([new Uint8Array(bytes)], {
      type: MIME_BY_EXT[ext] ?? 'image/png',
    });
    return URL.createObjectURL(blob);
  }
}
