# 性能基准页（tools/benchmark）

单文件、零依赖的静态页：**`index.html`** —— 横向对比 2-Pyramid 各优化节点的转换耗时。

## 用法

直接用浏览器打开 `index.html`（无需服务器、无外部请求）：

```
start "" "tools\benchmark\index.html"
```

页面包含：

- **堆叠柱状图**：每个节点按「解压 / 引擎 / 打包 / 临时目录清理」分段；
- **双计时口径切换**：「用户实际等待」（含隐性清理）与「进程内测量」；
  早期节点的清理发生在计时快照之后，以**斜纹段**标注，说明两种口径的差异来源；
- **逐项数据表**：分阶段耗时、加速比与对比条；
- **引擎内部拆解**、**质量校验结果**（`--pack-diff` 字节级一致）与方法学说明。

## 更新数据

编辑 `index.html` 中 `<script>` 顶部的 `MILESTONES` 数组即可，图表、表格、加速比与结论卡会自动重算：

```js
const MILESTONES = [
  { name:"节点名称", tag:"短标签", commit:"提交或说明",
    extract:0.98, engine:1.99, pack:0.55, cleanup:0, hiddenCleanup:false,
    note:"一句话说明" },
  // …
];
```

单位：秒。`cleanup` 填 `0` 表示「清理已移出等待路径」；`hiddenCleanup: true` 表示该节点的清理**未被计时但真实占用等待**（会画成斜纹段）。

数值来源：`2-pyramid.exe --convert <包> --to <版本> --report 报告.json` 生成的报告
（`timing.pureS / extractS / packS / cleanupS`），或日志里的 `conversion timing:` 一行。

## 生成预览图（可选）

预览图是生成物，**不入库**（`.gitignore` 已隔离）。需要时用无头 Edge/Chrome 自行渲染：

```powershell
& "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe" `
  --headless=new --disable-gpu --hide-scrollbars `
  --window-size=1280,4200 `
  --screenshot="tools\benchmark\preview.png" `
  "file:///<仓库绝对路径>/tools/benchmark/index.html"
```

## 目录约定

- 页面与说明**入库**（`index.html` / `README.md`）；
- 截图、导出的 JSON 等**生成物不入库**，见仓库根 `.gitignore` 的「基准页的生成物」段。
