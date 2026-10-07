# 内嵌页面（对话框 / 侧栏）设计规格

> 适用范围：`src/components/**` 下所有 `*Dialog.vue` / `*Sidebar.vue`，共 **27 个文件、5311 行**。
> 目标：让这些内嵌页面和主页面（首页 / 设置 / 转换 / 覆盖）说同一种语言。
>
> 写在逐个改造**之前**。不先定规格就分别改，七个对话框会改成七种样子。

---

## 1. 已完成的公共部分（不要在各组件里重写）

### 1.1 弹窗外壳 —— 唯一一份，在 `src/styles/shared.css`

**历史问题**：全项目曾有 5 份 `.dialog-overlay` + 4 份 `.dialog-content`，z-index 从 100 到 2000
差二十倍，遮罩从纯黑到冷灰，圆角 20 与 24 混用。弹窗长什么样取决于它被哪个组件定义。

而且 scoped 组件里的选择器会被加上 `data-v` 属性 →
`.dialog-overlay[data-v-x]`（特异度 **0,2,0**）**压过** shared.css 的 `.dialog-overlay`（**0,1,0**）。
所以**光写一份新的没用，页面级的那几份必须删掉**。现已删除，全项目只剩 shared.css 一对。

```css
.dialog-overlay {
  position: fixed; inset: 0;
  background: rgba(28, 24, 18, 0.34);   /* 暖色。原 rgba(15,23,42,·) 是冷蓝灰，压在暖白界面上会发脏 */
  display: flex; align-items: center; justify-content: center;
  z-index: 1000;                         /* 统一值。低于侧栏与窗口控制，高于页面内容 */
  backdrop-filter: blur(7px);
}
.dialog-content {
  width: 420px; max-width: 90vw;
  background: #fff;
  border: 1px solid rgba(120, 110, 95, 0.18);
  border-radius: 12px;                   /* 弹窗比按钮（8px）略圆：体量更大 */
  padding: 20px;
  box-shadow: 0 16px 40px rgba(28, 24, 18, 0.14);   /* 保留 —— 弹窗是真的浮起来的元素 */
  max-height: min(86vh, 760px);
  overflow: hidden;
  display: flex; flex-direction: column;
}
```

### 1.2 内部骨架 —— 也在 `shared.css`

| 类 | 作用 | 关键取值 |
|---|---|---|
| `.dialog-header` | 标题行 | `align-items: flex-start`；标题在左、关闭钮在右 |
| `.dialog-header h3` | 标题 | **15px / 600**（原 16px/700 —— 层级靠字号和下方留白建立，不靠字重） |
| `.dialog-header .dialog-sub` | 副标题（可选） | 12px / 400 / `#7d776d` |
| `.dialog-close` | 关闭钮 | 26×26、圆角 6、`#9c968c`，悬停暖色底 + 墨色 |
| `.dialog-body` | 正文区 | `overflow-y: auto`、`gap: 8px` |
| `.dialog-body p` | 正文段落 | 13px / `line-height: 1.65` / `#5f5a51` |
| `.dialog-footer` | 按钮行 | 靠右、`border-top: 1px solid rgba(120,110,95,0.14)` |
| `.btn-text` 系列 | 弹窗按钮 | 32px 高、圆角 8、13px / 500 |

`.btn-text` 的变体（按模板里的实际用量）：

| 变体 | 用量 | 语义 |
|---|---|---|
| `.btn-text` | 23 | 安静态（默认） |
| `.btn-text.secondary` | 23 | 与默认同义，显式写出 |
| `.btn-text.danger` | 9 | `#b3261e` |
| `.btn-text.primary` | — | 墨底白字，用于主操作 |

---

## 2. 语言约定（改任何一个内嵌页面都遵守）

### 2.1 颜色

| 用途 | 取值 |
|---|---|
| 主文字 / 强调 / 选中 | `#14171a`（墨） |
| 次要文字 | `#5f5a51` |
| 三级文字 / 标签 | `#7d776d` |
| 最弱（占位、时间戳） | `#9c968c` |
| 面板面 | `#fff`（弹窗）/ `rgba(255,255,255,0.6)`（页内块） |
| 发丝线 | `rgba(120, 110, 95, 0.14)` 起，最深 `0.2` |
| 悬停染色 | `rgba(120, 110, 95, 0.055)` ~ `0.09` |
| 危险 | `#b3261e` |

**禁止**：`#007bff` 及任何蓝色主题色、`rgba(0,0,0,·)` 冷灰发丝线、`rgba(15,23,42,·)` 冷蓝灰。

### 2.2 字号阶梯

```
24px  页面主标题（仅页面级）
18px  阶段标题
15px  弹窗标题
14px  卡片/条目标题
13px  正文、按钮
12px  次要说明
11.5px 标语
10.5px 小字标签（letter-spacing: 0.18em, uppercase）
```

### 2.3 圆角

```
12px  弹窗
 8px  按钮、输入框、页内块
 6px  行、小图标钮
 50%  圆点
```

### 2.4 投影

**默认不用。** 只有真正脱离文档流的元素才保留：弹窗、下拉浮层、主操作按钮（轻微抬升）。
卡片、列表行、页内块**一律无投影**。

### 2.5 层级（font-weight）

层级由**字号 + 留白**建立，不靠加粗。允许用到 600 的场合：弹窗标题、条目标题、主按钮文字。
**禁止 700 及以上**（首页品牌名是唯一例外，那是标识不是 UI 文本）。

---

## 3. 动效约定

### 3.1 曲线

```css
--ease-out:    cubic-bezier(0.22, 1, 0.36, 1)   /* 进场 */
--ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1) /* 位移、选中框 */
```

### 3.2 ⚠️ fill-mode 陷阱（本项目已踩过四次）

**动过 `translate` / `transform` 的动画，fill-mode 必须用 `backwards`，不能用 `both` / `forwards`。**

`both` / `forwards` 会让元素在动画结束后**永久保留末态关键帧里的 `translate: 0 0`**，
而 `translate` 只要不是 `none` 就会成为**包含块** —— 子元素里的 `position: fixed` 浮层
会相对它定位而不是视口。（实际症状：设置页的对话框遮罩只盖住右侧舞台区，盖不住整屏。）

改 fill-mode 时**必须同时检查基线规则**：基线里不能写 `opacity: 0` 之类的隐形状态，
否则 `backwards` 动画一结束就回落到隐形态，文字会整体消失。
（实际症状：首页 SplitText 的字符在动画结束后全部消失。）

### 3.3 时序

| 场合 | 时长 | 说明 |
|---|---|---|
| 遮罩淡入 | 200ms | |
| 弹窗进场 | 240ms | `opacity` + `translate: 0 10px`，**不要用 `scale`**（同样会造成包含块） |
| 列表项错开 | 420ms + `n × 55ms` | 延迟差要小 —— 读起来是「一起出现」，不是「依次点亮」 |

**不要用序号、箭头（`→`）、步骤条**暗示先后顺序，除非那个顺序真实存在。

### 3.4 减少动效

每个带动画的组件都要有：

```css
@media (prefers-reduced-motion: reduce) {
  .xxx { animation: none; transition: none; }
}
```

---

## 4. 逐组件改造清单

### 4.1 轻量（20 个，30–155 行）

几乎无自有样式，**外壳统一后外观已自动跟随**。只需逐个确认：
- 没有自己的 `.dialog-overlay` / `.dialog-content`（应为 0）
- 没有硬编码的圆角 > 12px 或投影
- 按钮用的是 `.btn-text` 系列
- 有 `@media (prefers-reduced-motion)`（如果带动画）

```
conversion\BedrockWarnDialog.vue            42 行
conversion\ConversionResultDialog.vue      155 行
conversion\DeleteSourceDialog.vue           30 行
conversion\ItemsListDialog.vue             108 行
conversion\Java263WarnDialog.vue            52 行   ← 主题色 1
overlay\CreateProjectDialog.vue             70 行
overlay\DeleteProjectDialog.vue             48 行
overlay\ExportShareDialog.vue               60 行
overlay\ImportShareDialog.vue               80 行
settings\ClearConfigDialog.vue              49 行
settings\DevUnlockDialog.vue                93 行
settings\FactoryResetDialog.vue            105 行
settings\HistoryDialog.vue                 138 行
settings\LogWindowDialog.vue               195 行
settings\OutputPathDialog.vue               63 行
settings\PackAnalysisDialog.vue            154 行   ← 圆角 3
settings\ThemeDialog.vue                   141 行   ← 投影 1
AuthorsDialog.vue                           88 行   ← 主题色 1
VersionInfoDialog.vue                      154 行   ← 主题色 2
settings\ClearConfigDialog.vue              49 行
```

### 4.2 重量级（7 个，298–905 行）—— 需要逐个处理

| 文件 | 行数 | 圆角 | 投影 | 主题色 |
|---|---|---|---|---|
| `ItemNameDialog.vue` | 905 | 14 | 11 | 14 |
| `VisualDialog.vue` | 537 | 10 | 4 | 4 |
| `UpdateDialog.vue` | 477 | 13 | 1 | 12 |
| `ItemSizeDialog.vue` | 470 | 4 | 5 | 5 |
| `LegalSidebar.vue` | 353 | 5 | 2 | 4 |
| `VersionSidebar.vue` | 319 | 3 | 0 | 7 |
| `settings\BackgroundDialog.vue` | 298 | 2 | 0 | 2 |

**侧栏（Sidebar）的额外约定**：`LegalSidebar` / `VersionSidebar` 是侧滑面板不是居中弹窗，
不套 `.dialog-overlay` 的居中 flex，但**共用同一套颜色、字号、圆角、发丝线**。

---

## 5. 验证要求

改完每一批必须：

1. `vue-tsc -b` 通过
2. `vite build` 通过
3. `npx` 在本机不可用，用直接二进制：
   - `code\node_modules\.bin\vue-tsc.cmd -b`
   - `code\node_modules\.bin\vite.cmd build`
4. **不要用 `npx`** —— 本机会报 `could not determine executable to run`
5. 抽查实际渲染：`.dsh-tmp/ui-review/` 下有基于裸 CDP 的截图脚本可复用
6. 改完报告里写明**哪些是实测的、哪些只是静态审查** —— 内嵌页面多数需要 Tauri 能力
   （文件系统、对话框、通知），浏览器里进不去

---

## 6. 已确认的全局决策（不要回退）

- 默认主题色 `#14171a`，全项目 `#007bff` 残留 0
- 全局卡片系统已把 `.card`、`.group-card`、`.ghost-btn`、`.seg-btn.active` 摘出名单
  （它们会以同特异度靠加载顺序压过页面自己的样式 —— 本项目遇到过**四次**）
- 页面背景统一用 `shell/StageBackdrop.vue`，不要各页自己画
