// tools/audit-converters.mjs
// 扫描 rust converters + pack.py,出 rs↔py mapping 矩阵 + 行数对比
import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = 'D:/develop/ResourcePackConverter/TauriRust/Hurricane';
const RS_DIR = `${ROOT}/src-tauri/src/converters`;
const PY = `${ROOT}/Python_Script/pack.py`;

// 排除工具型 converter(没有 py 对应纯工具/调度)
const SKIP_RS = new Set([
  'main_converter',         // orchestration
  'legacy_eraser',          // batch cleanup wrapper
  'legacy_processor',       // batch architect wrapper
  'version_converter',      // entry point
  'zip',                    // zip I/O
  'blockstate_adapter',     // internal
  'anims_folder_conversion',// empty stub
  'color_utils',            // internal helper
  'convert_old_texture_paths', // py 已废弃
  'convert_sound_files',    // py 已废弃
  'adjust_hue_brightness',  // 内部 helper
  'rename_and_process_blocks', // 被 rename_blocks_items 内部调用
  'gui_surgeon',            // 大调度,不是单一任务
  'mod',                    // 模块声明
]);

// 1. 扫所有 rs converter 文件
const rsFiles = readdirSync(RS_DIR)
  .filter(f => f.endsWith('.rs'))
  .map(f => f.replace('.rs', ''))
  .filter(n => !SKIP_RS.has(n));

// 2. 对每个 rs 文件,找 pub fn (顶层)
const rsFns = new Map(); // fn_name -> { file, line_count }
for (const name of rsFiles) {
  const content = readFileSync(join(RS_DIR, name + '.rs'), 'utf8');
  const lines = content.split('\n');
  // 找所有 ^pub fn xxx
  const matches = [];
  for (let i = 0; i < lines.length; i++) {
    const m = lines[i].match(/^pub fn (\w+)/);
    if (m) {
      // 数函数体行数(粗略:数下一个 ^pub fn 之前的行数,或文件末尾)
      let end = lines.length;
      for (let j = i + 1; j < lines.length; j++) {
        if (/^pub fn \w+/.test(lines[j]) || /^#\[cfg\(test\)\]/.test(lines[j])) {
          end = j; break;
        }
      }
      matches.push({ name: m[1], startLine: i + 1, bodyLines: end - i });
    }
  }
  for (const m of matches) {
    if (!rsFns.has(m.name)) {
      rsFns.set(m.name, { file: name, startLine: m.startLine, bodyLines: m.bodyLines });
    }
  }
}

// 3. 扫 pack.py
const pyContent = readFileSync(PY, 'utf8');
const pyLines = pyContent.split('\n');
const pyFns = new Map(); // fn_name -> { startLine, bodyLines }
for (let i = 0; i < pyLines.length; i++) {
  const m = pyLines[i].match(/^def (\w+)/);
  if (m) {
    // 简单数函数体行数:到下一个 def 或 ^class 或文件末尾
    let end = pyLines.length;
    for (let j = i + 1; j < pyLines.length; j++) {
      if (/^def \w+/.test(pyLines[j]) || /^class \w+/.test(pyLines[j])) {
        end = j; break;
      }
    }
    // 只保留第一个匹配(避免函数 shadow)
    if (!pyFns.has(m[1])) {
      pyFns.set(m[1], { startLine: i + 1, bodyLines: end - i });
    }
  }
}

// 4. 交叉匹配
const both = [];
const rsOnly = [];
const pyOnly = [];
for (const [name, rs] of rsFns) {
  if (pyFns.has(name)) {
    const py = pyFns.get(name);
    const diff = rs.bodyLines - py.bodyLines;
    const ratio = py.bodyLines > 0 ? (rs.bodyLines / py.bodyLines).toFixed(2) : 'inf';
    both.push({ name, rs, py, diff, ratio });
  } else {
    rsOnly.push({ name, rs });
  }
}
for (const [name, py] of pyFns) {
  if (!rsFns.has(name)) {
    pyOnly.push({ name, py });
  }
}

// 排序: 行数差异大的排前
both.sort((a, b) => Math.abs(b.diff) - Math.abs(a.diff));
rsOnly.sort((a, b) => a.name.localeCompare(b.name));
pyOnly.sort((a, b) => a.name.localeCompare(b.name));

// 5. 输出
const lines = [];
lines.push('# Converter rs↔py Mapping 矩阵\n');
lines.push(`- rs 端 converter 文件: ${rsFiles.length}`);
lines.push(`- rs 端 pub fn 总数: ${rsFns.size}`);
lines.push(`- py 端 def 总数(pack.py): ${pyFns.size}`);
lines.push(`- 两边都有的: ${both.length}`);
lines.push(`- rs 有 py 缺(可能 py 端被废弃): ${rsOnly.length}`);
lines.push(`- py 有 rs 缺(可能需要补): ${pyOnly.length}\n`);

lines.push('## ⚠️  行数差异 Top 30 (按 |rs-py| 差值排序)\n');
lines.push('| converter | rs 文件 | rs 行数 | py 行号 | py 行数 | 差值 | 倍数 |');
lines.push('|---|---|---:|---:|---:|---:|---:|');
for (const b of both.slice(0, 30)) {
  lines.push(`| ${b.name} | ${b.rs.file}.rs | ${b.rs.bodyLines} | ${b.py.startLine} | ${b.py.bodyLines} | ${b.diff > 0 ? '+' : ''}${b.diff} | ${b.ratio}x |`);
}

lines.push('\n## ✅ rs 有但 pack.py 找不到(可能 py 端用不同名,或 pack.py 不存)\n');
if (rsOnly.length) {
  lines.push('| converter | rs 文件 |');
  lines.push('|---|---|');
  for (const r of rsOnly) {
    lines.push(`| ${r.name} | ${r.rs.file}.rs |`);
  }
} else {
  lines.push('(无)');
}

lines.push('\n## ⚠️  pack.py 有但 rs 端没 converter(可能要新建 converter)\n');
if (pyOnly.length) {
  lines.push('| py 函数 | py 行号 | 行数 |');
  lines.push('|---|---:|---:|');
  for (const p of pyOnly) {
    lines.push(`| ${p.name} | ${p.py.startLine} | ${p.py.bodyLines} |`);
  }
} else {
  lines.push('(无)');
}

const out = lines.join('\n') + '\n';
writeFileSync('audit-converters.md', out, 'utf8');
console.log('Wrote audit-converters.md');
console.log(`matched=${both.length} rs_only=${rsOnly.length} py_only=${pyOnly.length}`);
console.log(`top diff: ${both.slice(0, 5).map(b => `${b.name}(${b.ratio}x)`).join(', ')}`);
