// 一次性脚本:用 sharp 把 2pyr-logo.svg 渲成多尺寸 PNG
// 用法: node tools/generate-logos.mjs
//
// ⚠️ 严禁把 logo PNG 写到 src-tauri/UImage/ 下的任何路径!
// UImage/ 是打包到最终安装目录的资源目录,会被 overlay_icons /
// generate_crossbow / overlay_icons 等转换器当作资源包覆盖层读
// 取。任何把 logo 误写到这里的行为都会让转换后的资源包被 logo
// 污染(实测: 2-Pyramid logo 出现在 icons.png 上)。
// 本脚本只允许写以下白名单位置,新增目标请先核对:
import sharp from 'sharp';
import { mkdir, copyFile, unlink } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '..');
const SRC = resolve(ROOT, '2pyr-logo.svg');

const ALLOWED_DIRS = new Set([
  ROOT,                                            // 项目根(logo-source.png)
  resolve(ROOT, 'public'),                         // favicon
  resolve(ROOT, 'src/assets'),                     // HomePage / SettingsPage / toast logo
]);

const TASKS = [
  // 喂给 tauri icon 的源(必须 1024x1024)
  { out: 'logo-source.png', size: 1024, dir: ROOT },
  // favicon(浏览器 tab 用)
  { out: 'favicon.png', size: 32, dir: resolve(ROOT, 'public') },
  { out: 'favicon-192.png', size: 192, dir: resolve(ROOT, 'public') },
  // HomePage 大图(mask 用,需要 alpha)
  { out: 'logo-256.png', size: 256, dir: resolve(ROOT, 'src/assets') },
  // 通用高清 logo(可给 SettingsPage / toast 等)
  { out: 'logo-512.png', size: 512, dir: resolve(ROOT, 'src/assets') },
];

function assertSafeOutputDir(targetDir) {
  // 1) 必须落在 ALLOWED_DIRS 之一
  let allowed = false;
  for (const root of ALLOWED_DIRS) {
    if (targetDir === root || targetDir.startsWith(root + sep)) {
      allowed = true;
      break;
    }
  }
  if (!allowed) {
    throw new Error(
      `Refusing to write logo PNG outside the allow-list.\n` +
      `  target = ${targetDir}\n` +
      `  allowed = ${[...ALLOWED_DIRS].join(', ')}\n` +
      `  If you really need a new location, add it to ALLOWED_DIRS in ` +
      `tools/generate-logos.mjs.`
    );
  }
  // 2) 严禁写到 src-tauri/UImage/ — 这是打包资源目录,会被覆盖到最终资源包
  const sepN = sep === '/' ? /[/\\]/ : new RegExp(`\\${sep}`);
  const pathParts = targetDir.split(sepN);
  if (pathParts.includes('UImage')) {
    throw new Error(
      `Refusing to write logo PNG into UImage/ — that directory is bundled ` +
      `into installed packs and will pollute converted resource packs. ` +
      `Target was: ${targetDir}`
    );
  }
}

async function main() {
  for (const t of TASKS) {
    assertSafeOutputDir(t.dir);
    if (!existsSync(t.dir)) await mkdir(t.dir, { recursive: true });
    const out = resolve(t.dir, t.out);
    // density: 对 SVG,只需稍大于目标尺寸即可;过高会爆 sharp 的像素上限
    const density = Math.max(96, Math.ceil(t.size * 1.5));
    await sharp(SRC, { density })
      .resize(t.size, t.size, { fit: 'contain', background: { r: 0, g: 0, b: 0, alpha: 0 } })
      .png()
      .toFile(out);
    console.log(`  ✓ ${out.replace(ROOT + '\\', '')}  (${t.size}×${t.size})`);
  }

  // 同步把 app-icon.png 替换为新 logo(HomePage 用的那个)
  const newIcon = resolve(ROOT, 'src/assets/logo-256.png');
  const oldIcon = resolve(ROOT, 'src/assets/app-icon.png');
  if (existsSync(oldIcon)) {
    await unlink(oldIcon);
    await copyFile(newIcon, oldIcon);
    console.log(`  ✓ src/assets/app-icon.png  ← logo-256.png  (replaced)`);
  }
}
main().catch(e => { console.error(e); process.exit(1); });
