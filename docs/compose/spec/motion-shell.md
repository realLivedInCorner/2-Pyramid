---
feature: motion-shell
status: draft
updated: 2026-10-06
branch: feat/motion
commits: dc3a149..HEAD
---

# Motion & Shell — 动效系统重构与界面结构决策

## Report

**本文档的性质** — 决策文档，不是实施记录。当前 `feat/motion` 分支**尚无代码改动**，`HEAD == dc3a149`（与 `master` 同点）。目的是在动 UI 之前，把「动效」与「界面结构」的依赖关系摆清楚，供裁决。

**决策状态** — **D1 = C（常驻外壳 + 内容井），已定（2026-10-06）**，落地结构见 §2.9。
**M0–M6 均已实施完成**（§M0、§2.9.8、§2.9.9、§M2、§M3、§M5、§M6），`vue-tsc -b` 与 `vite build` 通过，逐页、跨断点与运行时断言全部通过。**D2 = 是（已定）**。剩余 D3–D6 见 §审查时请确认。

**分界很重要**：M0–M5 改的是**动效系统**（取消架构、作用域、死代码）—— 按定义不可见；**M6 才是动效观感**（节奏、时间线、反馈密度）。参考项目 RhineLabUI 一个时长/缓动 token 都没有，所以「借它的系统」不会带来视觉变化；要「看得见」必须动节奏与密度。

**四项被证据修正的原计划**：
- M2.6（页面转场重写）实测只在 <100ms 极端连点下可见问题，建议缓做；
- M4.3 原估的「约 900 行死 CSS」经严格检测只有 40 条选择器属实；
- 真正的大块是**「作用域写错」**（M5）：父组件 scoped 规则到不了子组件内部，119 条从未生效；
- 入场错峰 `stagger-rise` **只动 opacity、根本没有位移**，且与容器淡入叠加成两层 opacity —— 这是「不连贯 + 动效少」的共同根因（M6）。

**两个结论**

1. 在 2-Pyramid 里，动效不是盖在界面之上的一层，**它的实现方式就是界面结构的契约**。因此「先换动效、UI 之后再说」这条路径不成立。
2. 参考项目（[RhineLabUI](https://github.com/LBEILC/RhineLabUI)）值得拿的**不是它的时长，是它的取消架构**。它解决"转场中途反向"的方式，恰好是 2-Pyramid 侧栏抽屉跳回 bug 的解法。

**已实测** — §1.2 全部数字来自对 `src/` 49 个文件的静态扫描（脚本 `.dsh-tmp/ui-review/motion-inventory.mjs`，可复现），非估算。§2.0 的参考系统拆解基于对上游 `src/` 的逐文件阅读，附 `file:line`。

---

## [S1] Problem

### 1.1 动效模型 = 界面结构契约

问题集中在 `App.vue:705-730` 这 26 行：

```html
<main class="main-content">
  <div class="page-content">
    <Transition :name="animationStyle">                      <!-- 707 -->
      <div class="page-shell" :key="currentPage"             <!-- 708 -->
           :data-anim-speed="animationSpeed">
        <component :is="pageComponent" v-bind="pageProps" ... />
      </div>
    </Transition>
  </div>
</main>
```

派生出五个结构性后果，每一个都不是"动效问题"：

**(a) `:key="currentPage"` → 每次切页整棵树销毁重建。**
页内状态、滚动位置、展开的折叠项、筛选词、选中项全部丢失。这是导航模型问题，但由这行动效代码决定。

**(b) `<Transition>` 未指定 `mode` → 转场期间新旧两页同时在 DOM 中。**
因此 enter / leave 必须 `position: absolute; inset: 0`（`App.vue:1475-1479`、`1488-1493`）。页面根因此是**固定尺寸舞台，不能按内容撑高**。

**(c) 后果：每个页面被迫自建滚动容器。**
`overflow: auto` / `overflow-y: auto` 在页面文件中出现 **25 处**：

| 文件 | 滚动容器 |
|---|---|
| `ForayPage.vue` | 6（693 / 730 / 828 / 949 / 988 / 1002）|
| `SettingsPage.vue` | 4（1529 / 1916 / 2116 …）|
| `OverlayPage.vue` | 3（602 / 803 / 950）|
| `ConversionPage.vue` | 2（1120 / 1300）|
| `StartupPage.vue` | 2（664 / 1278）|
| 其余各页 | 各 1–2 |

`.settings-scroll-area`、`.flip-viewport` 这类结构全是在给「舞台不能滚」打补丁。

**(d) `.page-shell > * > *` 错峰规则把 DOM 形状写死在选择器里**（`App.vue:1528-1544`）。
它假设"页面根的直接子元素 = 视觉分块"。`useSidebarSlide.ts:4-7` 的注释自己承认要绕开它：

> 用 inline style + CSS transition 驱动，**绕开 App.vue 全局 `.page-shell > * > *` 的 page-entry stagger（那条规则会锁 opacity）**

一个 composable 需要绕开自家 shell 的规则，即是耦合到 DOM 形状的直接证据。任何外壳重构（加持久导航、加命令栏、调整包裹层级）都会静默改变或破坏错峰行为。

**(e) 每个页面各写一个 header。**
`ConversionPage` / `OverlayPage` / `ForayPage` / `SettingsPage` 各自渲染 `.header-section` + 自己的返回按钮 + 自己的 26px 标题。导航栏没有连续性可言，因为它在被销毁重建。

### 1.2 现状实测

#### 转场与 token 采纳

| 指标 | 实测 |
|---|---|
| `transition:` 声明 | **111 处**，**72 种**不同写法 |
| 使用 `var(--motion-*/--ease-*)` | 13 处 |
| 写死时长/缓动 | 98 处 |
| **token 采纳率** | **11.7 %** |
| 动效 token 定义数 | 17（`App.vue:847-866`）|
| 动效 token 是否有死变量 | **无，17 个全部被引用** |

> token 层本身是健康的（对比颜色系统有 8 个死变量），问题在于**没人用**。

#### 时长：58 种字面量，且秒/毫秒混用

| 时长 | 写法 A | 写法 B |
|---|---|---|
| 200 ms | `0.2s` ×33 | `200ms` ×11 |
| 300 ms | `0.3s` ×18 | `300ms` ×1 |
| 280 ms | `0.28s` ×7 | `280ms` ×3 |
| 160 ms | `0.16s` ×2 | `160ms` ×3 |

同一个时长有两种拼写，取决于哪个文件写的。总字面量 **260 个**，去重后 **58 种**。

#### 缓动：12 种，`ease` 独占 53 %

| 缓动 | 次数 | 占比 |
|---|---|---|
| `ease` | 115 | **53 %** |
| `linear` | 28 | 13 % |
| `cubic-bezier(0.22, 1, 0.36, 1)` | 19 | 9 % |
| `ease-out` | 17 | 8 % |
| `ease-in` | 14 | 6 % |
| `ease-in-out` | 10 | 5 % |
| `cubic-bezier(0.4, 0, 0.2, 1)` | 7 | 3 % |

`--ease-out` 定义的就是 `cubic-bezier(0.22, 1, 0.36, 1)`，但它以字面量写了 19 次、以 `ease-out` 写了 17 次、以 `--ease-out` 只写了 13 次。53 % 的地方用的是浏览器默认 `ease` —— 等于没做决定。

#### `@keyframes`：23 个名字，34 处定义，11 处是纯复制

| 名字 | 定义次数 | 位置 |
|---|---|---|
| `ri-spin` | **6** | ItemNameDialog:898 / ItemSizeDialog:460 / OverlayPage:1020 / SettingsPage:2246 … |
| `overlay-fade` | **4** | ItemNameDialog:899 / ItemSizeDialog:465 / UpdateDialog:304 / VisualDialog:534 |
| `spin` | 2 | App.vue:1701 / ConversionPage:1037 |
| `bg-breathe` | 2 | ConversionPage:1462 / HomePage:238 |
| `aurora-drift` | 2 | ConversionPage:1467 / HomePage:244 |

`spin` 与 `ri-spin` 是同一个动画的两个名字。

#### 页面转场四套预设质量不齐

| 预设 | 实现 | 问题 |
|---|---|---|
| `fade-scale`（默认）| 全部走 token，含 blur，有注释解释为何不加 scale | 良好 |
| `fade` | `opacity 0.5s ease-in-out` 写死 | 未走 token，0.5s 明显偏慢 |
| `slide` | `all 0.6s cubic-bezier(0.4,0,0.2,1)` | **`all` 会动画化全部属性** |
| `page-turn` | `all 0.8s cubic-bezier(0.2,0.8,0.2,1)` | 同上，0.8s 偏慢 |

且 `@update:animation-style` **全项目没有 emitter** —— 四套预设中只有 `fade-scale` 实际可达，另外三套只能手改 `localStorage`。确定性缺陷。

#### 侧栏抽屉无中断能力

`useSidebarSlide.ts`：

```ts
// 16-22  onBeforeEnter
h.style.transition = "none";
h.style.transform  = "translateX(100%)";   // ← 硬跳回全关位置

// 36-42  onEnter
requestAnimationFrame(() => {
  h.style.transition = "transform 320ms cubic-bezier(0.22, 1, 0.36, 1), ...";
  setTimeout(done, 330);                   // ← 手写 330 配 320ms
});
```

关闭进行到 240 ms 时重新打开，元素会**瞬间弹回屏幕外重来**。同时 `330` / `250` 两个魔数需与人肉同步的 `320ms` / `240ms` 保持一致。

#### 速度档位只覆盖一半

`App.vue:1546-1562` 的 `.page-shell[data-anim-speed="slow"|"fast"]` 只覆盖 6 个 token。**弹窗与 toast 的动效不随速度档变化** —— 选「慢」只慢了页面切换。

#### 错峰有 10 个的上限

`--stagger-i` 只在 `:nth-child(1)` 至 `:nth-child(10)` 上定义。第 11 个及以后的直接子元素取不到值，回退为 `var(--stagger-i, 0)` = 0，**与第 1 个同时升起**。当前各页大多未超过 10 个直接子元素，属隐患。

#### 减少动效是全局压零

```css
/* App.vue:870-879 */
body.motion-reduced * {
  animation-duration: 0.01ms !important;
  transition-duration: 0.01ms !important;
}
/* 881-889 唯一例外：spinner */
```

覆盖面完整，且 **2-Pyramid 在这里比参考项目做得更好**：`App.vue:126-131` 经 `matchMedia` 接入系统偏好，并且（`App.vue:629-633`）注册了 `change` 监听——参考项目只在模块初始化时采样一次，运行中改系统设置不会生效。

但压零**无法区分装饰与状态反馈**：转换进度、加载态、列表插入一律被压成 0.01ms。

### 1.3 为什么必须一起改

参考项目解决"转场中途反向不回跳"的核心手段是：**在取消上一个动画之前，先读回当前正在绘制的值**。

```ts
// RhineLabUI src/ui-transitions.ts:34-42
const revision = ++this.revision;
const hidden = this.root.hidden;
const opacity = hidden ? "0" : getComputedStyle(this.root).opacity;      // ← 读当前值
const transform = this.panel
  ? hidden ? "translateY(12px)" : getComputedStyle(this.panel).transform  // ← 读当前值
  : undefined;
this.animations.forEach((animation) => animation.cancel());               // ← 再取消
```

这要求**动画主体在中断时仍然存在于 DOM 中、且有可读的计算样式**。而 Vue 的 `<Transition>` 本质是挂载/卸载 —— 转场中途节点是新建的，"当前值"无从谈起。

因此要拿到可中断能力，模型必须变成：

> **放弃 `v-if` + CSS transition，改为常驻节点 + 显式数值状态。**

推到 shell 层即是：**页面不再换页，而是常驻分层** —— 直接推翻 1.1(a) 的 `:key` 换页模型，也一并决定 1.1(b)(c)(d)(e)。

「换动效」与「换 UI」在 2-Pyramid 里不是两件事。

---

## [S2] Design

### 2.0 参考系统拆解：RhineLabUI 的动效架构

> 上游仓库无任何动效框架、无 token 文件、无动画库包装（依赖只有 Three.js / Vite / 字体包 / `@kitlangton/rolling-number`）。全部机制靠四个文件承载：`ui-transitions.ts`(109 行)、`motion.ts`(118 行)、`theme-motion.ts`(33 行)、`motion-preferences.ts`(211 行)。

#### 2.0.1 四种驱动方式的分工

| 驱动 | 用途 | 理由 |
|---|---|---|
| **Web Animations API** `element.animate()` | **全部 DOM 表面生命周期**：详情层、弹窗、页签内容、收藏反馈、查看器进出 | 有真实 promise（`.finished`）且可取消 |
| CSS `transition` | hover/focus、持久样式变化、常驻装饰性淡入、区域显隐 | 声明式，无需 JS 记账 |
| CSS `@keyframes` | 仅 4 个存活：`orbit` / `blink` / `fadeIn` / `loading` | 纯装饰，且都在开场遮罩内 |
| rAF + 闭式求值 | 3D 场景、主题波、正文解密、2D 开场 | 需要帧率无关的积分与逐帧投影 |

**关键纪律：全仓库零 `transitionend` / `animationend` 监听。** 完成检测只有四种：`finished.then().catch()`、派生谓词（`get presentationHidden()`）、epsilon 吸附（`ALIGNMENT_EPSILON = 0.001`）、硬数值截断（主题波 `1.18s`）。`setTimeout` 从不用于动画完成。

#### 2.0.2 核心机制：revision ticket + 实时读值

```ts
// RhineLabUI src/ui-transitions.ts:34-58（节选）
private run(show: boolean, reduced: boolean, finished?: () => void) {
  const revision = ++this.revision;                    // ① 单调票据
  const opacity = hidden ? "0" : getComputedStyle(this.root).opacity;   // ② 实时读值
  this.animations.forEach(a => a.cancel());            // ③ 取消旧的
  const complete = () => {
    if (revision !== this.revision) return;            // ④ 过期回调直接丢弃
    this.root.hidden = !show;
    finished?.();
  };
  if (reduced || (!show && hidden)) { complete(); return; }   // ⑤ reduced 同步落位
  // … 用 element.animate(..., { fill: "both" }) 起新动画，.finished.then(complete).catch(()=>{})
}
```

四个要点：
1. **实时读值**：在取消旧动画**之前**采样 `getComputedStyle`。中途反向因此从"当前绘制的像素"开始，而非从 1 或 0。
2. **票据作废旧回调**：不是队列、不是"pending exit"标志 —— 最新调用直接赢。
3. **`fill: "both"`**：保证取消瞬间的视觉状态有定义，计算样式可读。
4. **`.catch(() => {})`**：`cancel()` 会以 `AbortError` 拒绝 `finished`，必须吞掉。

`dispose()` 先 bump 票据再取消动画，使排队中的 `complete()` 无法对已销毁节点生效。

#### 2.0.3 两个独立票据

查看器同时需要守护**视觉生命周期**与**异步资源加载**，因此带两个单调计数器：

| 字段 | 守护对象 |
|---|---|
| `transitionId` | 进场/退场动画生命周期 |
| `request` | 在途模型加载 |
| `isOpen` / `closing` | 重入保护（重复 Esc 无副作用）|

```ts
// src/model-viewer.ts:353-357
// Capture the current fade when Escape interrupts opening.
const opacity = getComputedStyle(this.root).opacity;
const canvasStyle = getComputedStyle(this.canvasHost);
```

以及一条重要纪律：**重型拆除延后到可见退场之后**。

```ts
// src/model-viewer.ts:393-413
private finishClose() {
  // Keep rendering and retain modal focus until the visible exit completes.
  this.isOpen = false;
  if (this.source) { this.scene.remove(this.source.model); this.source.dispose(); … }
  this.opener?.focus({ preventScroll: true });
}
```

#### 2.0.4 距离错峰 + 冻结原点

主题切换是一个从选中卡片向外扩散的波，且用户可能中途反向：

```ts
// src/theme-motion.ts:13-32（节选）
set(dark, time, origin, immediate = false) {
  const target = dark ? 1 : 0;
  if (target === this.target && !immediate) return;              // 幂等
  this.backgroundFrom = immediate ? target : this.background(time);
  this.from = immediate ? new Map() : new Map(this.latest);      // ← 每格从自身当前值接续
  this.target = target; this.start = time;
  this.origin = { ...origin };                                   // ← 原点冻结
}
sample(cell, time) {
  if (time - this.start >= 1.18) return this.target;             // ← 硬截断，无需完成回调
  const delay = Math.min(.6, Math.abs(cell.row - this.origin.row) * .034
                            + Math.abs(cell.lane - this.origin.lane) * .11);
  const from = this.from.get(key(cell)) ?? this.backgroundFrom;
  return from + (this.target - from) * ease((time - this.start - delay) / .58);
}
```

- `beginFrame()` 清空 `latest`，每帧所有格子重新 `sample` 回填 → `latest` 是**上一帧全部格子的实际色值快照**。
- 反向时 `from = new Map(this.latest)`：尚未开始的格子从旧色起步，进行中的格子从中间值续接。
- **错峰按物理距离，不按 DOM 顺序。** 波永远从用户触碰的那张卡辐射出去。
- `1.18s = maxDelay(0.6) + duration(0.58)`，让已稳定的格子可以直接取 `backgroundFrom`，省掉逐格完成追踪。

#### 2.0.5 可重定向弹簧

```ts
// src/motion.ts:108-118
export interface Spring { value: number; velocity: number; }
export function damp(s: Spring, target: number, rate: number, dt: number) {
  const delta = s.value - target;
  const impulse = s.velocity + rate * delta;
  const decay = Math.exp(-rate * dt);
  s.value = target + (delta + impulse * dt) * decay;
  s.velocity = (s.velocity - rate * impulse * dt) * decay;
}
```

解析解，**重定目标时同时保留位置与速度**，且帧率无关（上游有 `check-motion.mjs:122-135` 两条断言）。上游用它以 8 种不同速率驱动相机轨道、悬停抬升、归位。旋转用指数衰减 + epsilon 吸附保证真正终止：

```ts
// src/motion.ts:100-107
export const ALIGNMENT_EPSILON = 0.001;
export function returnStep(angle: number, dt: number, reduced = false) {
  const next = angle * Math.exp(-dt * (reduced ? 35 : 7));
  return Math.abs(next) <= ALIGNMENT_EPSILON ? 0 : next;
}
```

#### 2.0.6 减少动效：14 个标志 / 5 组 / 派生预设

```ts
// src/motion-preferences.ts:1-22
export type MotionKey =
  | "boot" | "selectionWave" | "idleWave" | "pointerParallax" | "dragMomentum"
  | "selectionTransition" | "detailTransition" | "modelDecryption" | "documentReveal"
  | "rollingText" | "rollingNumbers" | "surfaceTransitions"
  | "viewerNavigation" | "viewerModelTransition";
```

**优先级链**（`src/main.ts:158-163`）：存盘的逐项标志 → 存盘的预设 → 旧版布尔 → 系统 `prefers-reduced-motion`（仅首次访问）→ 全开。

**预设标签是派生的，不作为真相存储**：全 true → `"full"`，全 false → `"reduced"`，否则 `"custom"`。这是防止"预设显示 reduced 但还有三处在动"这类 bug 的关键。

短路由六套独立机制承担：全局 CSS 压零、定向 CSS 例外、WAAPI 同步落位、控制器 `animated` 标志、3D 速率替换（`4.2 → 35`）、同步状态吸附。

⚠️ 但它的**全局压零与 2-Pyramid 的 `.reduce-motion *` 是同一个反模式**，见 §2.2。

### 2.1 借什么、不借什么

**借（架构层）**

| # | 机制 | 为什么值得 |
|---|---|---|
| 1 | **revision ticket + `getComputedStyle` 实时读值** | 这是"可中断"的全部秘密，也是 2-Pyramid 侧栏 bug 的直接解法 |
| 2 | **WAAPI 承担表面生命周期，CSS transition 只管 hover/持久态** | 有真实 promise、可取消，不必再手写 `setTimeout` 同步魔数 |
| 3 | **视觉生命周期与异步工作分离的两个票据** | Tauri 下 IPC 延迟不可预测，用户可能在数据到达前关掉面板 —— 这个比 Web 端更重要 |
| 4 | **完成检测的四种手段**（promise / 派生谓词 / epsilon / 硬截断），零 `transitionend` | 消除一整类"动画结束回调没触发"的 bug |
| 5 | **重型拆除延后到可见退场之后** | 2-Pyramid 可用于：停止轮询、关闭文件监听、`URL.revokeObjectURL` 大 blob |
| 6 | **派生预设 + 严格优先级链** | 防止设置自相矛盾 |
| 7 | **`damp()` 15 行可重定向弹簧** | 帧率无关、重定向保速。副作用：分栏拖拽、滚动跟随头部、数值读数 |
| 8 | **错峰按物理距离 + 冻结原点** | 波从用户触碰处辐射，而非 DOM 顺序 |

**不借**

| 不借 | 原因 |
|---|---|
| 三维场景 / 相机 / 材质 / Blender 管线 | 与资源包转换工具无关 |
| `motion.ts` 的波形半边（`archiveWave` / `settlingWave` / `idleWave` / `cinematicField`…）| 是给 32×5 空间卡片阵列调参的场景库，系数（0.65 横向权重、19 行/秒波峰、3.4 高斯宽度）不可移植 |
| `boot-motion.ts` 的 25fps 帧网格 | 是"对齐参考影片"的装置，工具类应用不该背一个帧网格 |
| 电影化编排节奏 | 见下 |
| 视觉识别（配色/字体/标志）| 许可证明确排除原作视觉设计 |

> **借架构，不借节奏。** 参考项目是**六屏展示型**应用，转场可以慢；2-Pyramid 是高频往返的工具（转换 → 设置 → 返回），照搬 320ms 页面转场 + 300ms 延迟的入场级联会明显拖慢操作。

#### ⚠️ 一个反直觉的发现：参考项目没有 token 层

上游六个样式表里，**动效自定义属性数量为零**。没有任何 `--duration-*` / `--ease-*`。每个时长和缓动都是使用处的字面量，跨文件出现 `0.2/0.25/0.3/0.4/0.45/0.6/0.65/0.7/0.8/1/1.3/1.5/1.6 s` 与 `150/180/200/220/260/300/320/380/400/460/580/650/850/950 ms`。

结论要反过来写：

| 维度 | RhineLabUI | 2-Pyramid |
|---|---|---|
| 动效 token 层 | **不存在** | **有，17 个，且全部被引用** |
| token 采纳 | — | 11.7 %（有而不用） |
| 取消/中断架构 | **完整**（票据 + 实时读值） | 无（`transition:none` 硬跳） |
| 完成检测 | 四套手段，零 `transitionend` | `setTimeout` 手写同步 |
| 系统偏好变更监听 | ❌ 仅初始化采样一次 | ✅ 有 `change` 监听 |

**所以两边的问题是镜像的**：2-Pyramid 有 token 层但没人用；参考项目有一套优秀的取消架构但完全没有 token 层。**要拿的是它的第 1–5 项机制，不是它的时长表。** 它那套 14 个各不相同的时长恰恰是需要避免的。

### 2.2 两个项目共有的反模式（需要在本项目内主动规避）

| # | 反模式 | RhineLabUI | 2-Pyramid |
|---|---|---|---|
| 1 | **`.reduce-motion *` 全局压零** | `style.css:1771-1776` | `App.vue:870-879` **同款** |
| 2 | **上帝模块** | `main.ts` 1337 行（prefs + DOM 接线 + 转场 + 键盘 + 音频 + rAF）| `App.vue` **2002 行**（同职责）|
| 3 | **常驻 shell 里的无限装饰动画** | `orbit` 8s/4s/16s、`blink` 0.8s、`loading` 1.3s | HomePage `bg-breathe` 14s / `aurora-drift` 26s / `blob-float` 24s |
| 4 | **全屏遮罩上 `backdrop-filter: blur()`** | `blur(18px)` 淡入 300ms | `.dialog-overlay` `blur(6px)` |
| 5 | **时长方差无共享标度** | 同一交互族内 150/180/200/220/260/300/320/380 | 同一问题，58 种值 |
| 6 | **过长的入场级联** | `.archive-ui` 800ms + 300ms 延迟 | Startup 的 `oobe-rise` 阶梯 |

第 3 条对 2-Pyramid 尤其重要：桌面工具的外壳会常驻数小时，无限 CSS 动画会阻止合成器进入空闲。若采纳 §2.4 的 B 方案（常驻分层），这条会从"三个动画"恶化成"全部页面的动画同时跑"。

第 4 条：`.dialog-overlay { backdrop-filter: blur(6px) }` 在遮罩淡入的每一帧都要模糊整屏，而遮罩底下通常压着密集文字表。建议改近乎不透明的纯色遮罩。

### 2.3 动效能力 → shell 结构要求

这张表是本文的核心。左侧是想要的动效能力，右侧是它**强制**要求的结构。

| 想要的动效能力 | 结构要求 | 现状 |
|---|---|---|
| 转场中途反向不回跳 | 节点常驻；中断时可读回当前计算样式 | ❌ |
| 从当前值接续 | 显式数值状态，而非 CSS 隐式插值 | ❌ |
| 跨页共享元素 | 两页需在同一时刻存在于同一坐标系 | ❌（`:key` 重建）|
| 导航/chrome 连续动画 | chrome 不随页面重建 | ❌（每页各写 header）|
| 切页保留页内状态与滚动 | 页面不销毁 | ❌ |
| 内容区 FLIP / 重排动画 | 内容井为定位容器，且能测到前后几何 | 🟡 |
| 逐项 reduced-motion | 每个动效有独立声明位置 | 🟡（有 token，但被 `!important` 压过）|
| 统一的滚动布局 | 滚动由 shell 或内容井提供 | ❌（25 处各写各的）|
| 高频操作不被延迟 | 时长按工具场景定 | ❌（0.5s / 0.6s / 0.8s）|

### 2.4 候选 shell 模型

#### A. 维持现状：换页舞台

```
.page-content (relative, overflow: hidden)
└─ <Transition>                       ← 转场期间两个 page-shell 并存
   └─ .page-shell :key="currentPage"  (absolute; inset: 0)
      └─ <component :is="pageComponent">
         ├─ .header-section           ← 每页一份
         └─ 内容（各页自建滚动容器）
```

- **改动范围**：仅 token 层
- **可得到**：交叉淡入 + blur、入场错峰
- **得不到**：可中断导航、共享元素、导航连续性、状态保留

#### B. 常驻分层舞台

```
.page-content (relative)
├─ .layer[data-page="home"]        ┐
├─ .layer[data-page="conversion"]  │ 全部常驻
├─ .layer[data-page="overlay"]     │ 可见性由显式数值驱动
└─ .layer[data-page="settings"]    ┘ 非活动层挂起
```

- **改动范围**：shell + 全部页面
- **可得到**：全部动效能力（可中断、可反向、跨页共享元素、状态天然保留）
- **必须配套**：挂起策略 —— `animation-play-state`、`content-visibility`、watcher/IPC 门控。否则 §2.2 第 3 条会全面恶化

#### C. 常驻外壳 + 内容井 ← **推荐**

```
.app-container
├─ .app-chrome         ← 常驻：标题、返回、搜索、窗口按钮
│                        （现在这些被四个页面各写一份）
└─ .content-well (relative)
   └─ 内容层（交换，但 chrome 不动）
```

- **改动范围**：shell + 页面 header 外提 + 滚动收敛到内容井
- **可得到**：chrome 连续动画、内容区 FLIP、可中断、状态保留、滚动统一
- **代价**：中等
- **关键性质**：**C 是 B 的子集** —— 内容井日后可升级为常驻分层，不必推翻

### 2.5 能力矩阵

| 能力 | A | B | C |
|---|---|---|---|
| 页面交叉淡入 + blur | ✅ | ✅ | ✅ |
| 入场错峰 | ✅ | ✅ | ✅（chrome 不参与）|
| 转场中途反向不回跳 | ❌ | ✅ | ✅ |
| 从当前值接续 | ❌ | ✅ | ✅ |
| 跨页共享元素 | ❌ | ✅ | 🟡 仅内容井内 |
| 导航/chrome 连续动画 | ❌ | ✅ | ✅ |
| 切页保留页内状态 | ❌ | ✅ | ✅（需显式管理）|
| 切页保留滚动位置 | ❌ | ✅ | ✅ |
| 滚动布局统一 | ❌ | ✅ | ✅ |
| 逐项 reduced-motion | ❌ | ✅ | ✅ |
| 运行期开销 | 低 | **高** | 中 |
| 迁移成本 | 低 | **高** | 中 |
| 风险 | 低 | 高 | 中 |

### 2.6 Token 层设计（与 shell 决策无关，可先行）

#### 时长归并

58 种 → 7 档：

| 现有值 | 次数 | 归并到 |
|---|---|---|
| `0.15s` | 62 | `--dur-micro 160ms` |
| `0.18s` | 25 | `--dur-micro 160ms` |
| `0.2s` / `200ms` | 44 | `--dur-quick 200ms` |
| `0.25s` | 9 | `--dur-base 240ms` |
| `0.28s` / `280ms` | 10 | `--dur-base 280ms` |
| `0.3s` / `300ms` | 19 | `--dur-slow 320ms` |
| `0.35s` | 5 | `--dur-slow 360ms` |
| `1s` | 10 | `--dur-spin 1000ms`（保留，spinner 专用）|
| 其余 28 种 | 各 ≤3 | 就近归并或删除 |

> 注意：参考项目的详细时长表（180/300/200/150/320/380/220/460/560…）**不作为归并依据** —— 它是展示型应用的节奏，且自身就没有标度。

#### 缓动归一

12 种 → 4 条 + 1 条弹簧：

| 现有 | 次数 | 处理 |
|---|---|---|
| `ease` | 115 | **全部替换**（主要工作量）|
| `linear` | 28 | 保留（spinner / progress）|
| `cubic-bezier(0.22,1,0.36,1)` | 19 | → `var(--ease-out)` |
| `ease-out` | 17 | → `var(--ease-out)` |
| `ease-in` | 14 | → `var(--ease-in)` |
| `ease-in-out` | 10 | → `var(--ease-in-out)` |
| `cubic-bezier(0.4,0,0.2,1)` | 7 | → `var(--ease-in-out)` |
| `cubic-bezier(0.4,0,1,1)` | 3 | → `var(--ease-in)` |
| `cubic-bezier(0.2,0.8,0.2,1)` | 3 | 归并到 `--ease-out` |
| `cubic-bezier(0.34,1.4/1.56,0.64,1)` | 2 | 保留为 `--ease-spring` |

`--ease-out` = `cubic-bezier(0.22, 1, 0.36, 1)` 与参考项目的 `enterEase` 一致；`--ease-in` = `cubic-bezier(0.4, 0, 1, 1)` 与它的 `exitEase` 一致。**这两条曲线已经是对的，直接沿用。**

#### 速度档位下沉

`.page-shell[data-anim-speed]` 只覆盖 6 个 token 且挂在 `.page-shell` 上。改为挂在 `:root[data-anim-speed]`，覆盖**全部**时长 token，使弹窗/toast 一并响应。

### 2.7 减少动效的逐项语义（提案）

以三段分类替代当前的全局压零：

| 类别 | 内容 | 减动效时 |
|---|---|---|
| **装饰** | 入场错峰、极光/光斑、hover 位移、blur 转场、呼吸动效 | 完全移除 |
| **状态反馈** | spinner、进度条、转换进度、按钮按下反馈 | **保留**（当前已有 spinner 例外，需扩到全部状态反馈）|
| **空间/位置变化** | 抽屉进出、弹窗进出、列表插入重排 | 瞬时到位，但保留 opacity 交叉以避免硬闪 |

第三类尤其重要：全局压零会让抽屉/弹窗"啪"地出现，用户失去方位感；保留 opacity 交叉既不触发前庭不适，也不丢方位。

参考项目的教训：它的 `.reduce-motion *` 压零会**静默覆盖组件级豁免**——本该保留的 80ms focus 动画也一并被砍。实现时要留出"信息承载型短转场"的豁免通道。

### 2.8 可中断原语设计（shell 定案后使用）

移植参考项目的 `SurfaceTransition`，落到 Vue composable：

```ts
// 目标形态（示意）
const { show, hide, finish, state } =
  useSurfaceTransition(rootRef, panelRef, { enter: 200, exit: 160, reduced });
```

配合 `<Transition :css="false" @enter="(el, done) => show(done)" @leave="(el, done) => hide(done)">` —— Vue 的 `done` 回调对应参考项目的 `finished?.()`，而 composable 补上 `<Transition>` 本身不提供的两件事：

1. **票据 + 实时读值**（`getComputedStyle` 采样先于 `cancel()`）
2. **视觉票据与异步票据分离**

```ts
// 异步工作的守护（Tauri 下尤其重要）
let visual = 0, io = 0;
async function load() {
  const v = visual, t = ++io;
  const data = await invoke('...');
  if (t !== io || v !== visual) return;   // 已关闭或被新请求取代 → 丢弃
  apply(data);
}
```

**要点：不要用 `transition: none` 再设终值**（那是回跳的根因），而是读当前值 → 改目标 → 按剩余距离重算时长。

### 2.9 D1 已定：C 方案落地结构

> **决策记录** — D1 = **C（常驻外壳 + 内容井）**，2026-10-06。

#### 2.9.1 现状：四个页面各写一份 header

| 页面 | 返回 | 标题 | 页面专属 |
|---|---|---|---|
| Conversion | `.back-btn` | `.title-group` › `h1.page-title` + `p.page-subtitle` | — |
| Overlay | `.back-btn` | `.title-group` › **`h1.title`** + `p.page-subtitle` | `.header-status` 状态条 |
| Foray | ghost back | 标题 + 源包路径副标题 | 3 个动作按钮 |
| Settings | `.back-btn` | `.title-group` › `h1.page-title` + `p.page-subtitle` | 居中搜索框 + 空的 `.header-right` |
| Home | — | 绝对定位的品牌胶囊 | 问候胶囊 |

已存在的不一致：同一个标题，Conversion / Settings 用 `h1.page-title`，Overlay 用 `h1.title`；padding 也不统一（`.header-section` `28px 40px 10px` vs `.settings-header` `24px 48px 16px`）。每页还要各自 `margin-right: 140px` 让开窗口按钮。

#### 2.9.2 目标结构

```
App.vue (shell)
├─ .drag-region                    28px，常驻
├─ .floating-window-controls       常驻
├─ <AppChrome>                     ← 新组件，常驻，不参与内容转场
│   ├─ #chrome-back                返回按钮；home 时隐藏
│   ├─ #chrome-title               h1 + subtitle
│   ├─ #chrome-center              可选：搜索等
│   └─ #chrome-actions             可选：页面动作按钮
└─ .content-well                   flex:1; min-height:0; position:relative
    └─ 内容层（转场只发生在这里）
```

#### 2.9.3 两条数据通路

**文本类信息用响应式声明**（不走 slot）—— 与既有 `useNotification.ts` 的模块级响应式单例同构：

```ts
// 页面内
useAppChrome({
  title: () => t('conversion.title'),
  subtitle: () => t('conversion.subtitle'),
  back: () => emit('switch-page', 'home'),
});
```

**动作按钮用 `<Teleport>`** —— 因为它们是页面自己的交互与响应式：

```html
<!-- ForayPage 内 -->
<Teleport to="#chrome-actions">
  <button :disabled="!opened || !issues.length" @click="showIssues = true">…</button>
  <button class="primary" @click="pickZip">…</button>
</Teleport>
```

理由：文本是「shell 渲染、页面提供数据」，用 composable 最省；按钮是「页面渲染、shell 提供位置」，只有 Teleport 能保住事件处理器与响应式归属。

#### 2.9.4 Home 的例外

Home 没有 `.header-section`，是绝对定位的品牌胶囊 + 问候胶囊 + 全屏 hero。C 方案下：

- `currentPage === 'home'` 时 `<AppChrome>` 隐藏
- `.content-well` 在无 chrome 时占满全高
- Home 保留自己的绝对定位组合（**不改它的视觉**）

不要试图把 Home 塞进 chrome —— 会破坏它现在的构图。

#### 2.9.5 滚动契约

「统一滚动」**不等于**「所有页面都滚」。三类需求不同：

| 页面 | 需求 |
|---|---|
| Conversion / Overlay | **填满**：固定视口网格，`overflow: hidden`，不滚 |
| Settings | **滚动**：单一滚动容器 |
| Foray | **分栏滚动**：三个 pane 各自独立滚 |

因此契约是：**`.content-well` 只保证「有界高度 + `overflow: hidden`」，页面按需自建滚动容器。**

要删的是**冗余**容器 —— 那些仅因「舞台不能滚」才存在的补丁。Settings 的 `.settings-scroll-area` 与 Foray 的三个 pane 是合理的，保留。

#### 2.9.6 转场语义

- chrome **永不卸载** → 导航动画天然连续，这是 C 相对 A 的核心收益
- 内容转场被限制在内容井内，不再需要 `position: absolute; inset: 0` 的舞台假设
- 若 D2 = 保留状态，内容层改为常驻 + `v-show` + 数值驱动（即把 B 收进内容井）—— 可延后，不阻塞 C 先落地

#### 2.9.7 迁移顺序（逐步可回滚）

1. 新建 `AppChrome.vue` + `useAppChrome.ts`，**先不接页面**（空跑验证）
2. Settings 接入（header 结构最规整，风险最低）
3. Conversion 接入
4. Overlay 接入（多一条状态提示，需 `<Teleport>`）
5. Foray 接入（多三个动作按钮，需 `<Teleport>`）
6. Home 保持独立，仅验证 chrome 隐藏时布局正确
7. 删除各页残留的 `.header-section` / `.settings-header` CSS

#### 2.9.8 实施记录（M1.1–M1.5，已完成）

**产物**

| 文件 | 状态 |
|---|---|
| `src/composables/useAppChrome.ts` | 新增 — 模块级响应式单例 + `Symbol` 所有权守卫 |
| `src/components/AppChrome.vue` | 新增 — 常驻外壳，`v-show` 保持 `#chrome-center` / `#chrome-actions` 常驻 |
| `src/App.vue` | 渲染外壳；新增 `--shell-gutter` 及其四档断点 |
| `ConversionPage` / `OverlayPage` / `SettingsPage` / `ForayPage` | header 外提，各接 `useAppChrome()` |
| `HomePage` | **未改动** —— 外壳在 `currentPage === 'home'` 时隐藏 |

**为什么所有权用 `Symbol` 守卫**
`<Transition>` 未指定 `mode`，转场期间新旧两页同时存在、两个 `watchEffect` 都会跑。没有守卫时，旧页在语言切换等场景下重新求值会覆盖新页刚写好的标题。`onUnmounted` **故意不清理**：所有权守卫已保证旧页不会覆盖新页，而旧页卸载晚于新页 setup，在卸载里清理反而会清掉新页的标题。

**验证结果**（1200×750，逐页实测）

| 页面 | chrome | 标题 | 返回 | 动作区 | 中列 | 残留旧 header |
|---|---|---|---|---|---|---|
| home | 隐藏 | — | — | 0 | 0 | 无 |
| conversion | 显示 | 资源包转换 | ✓ | 0 | 0 | 无 |
| overlay | 显示 | 覆盖包制作 | ✓ | 0 | 0 | 无 |
| settings | 显示 | 偏好设置 | ✓ | 0 | **1**（搜索框）| 无 |
| foray | 显示 | Foray 工作台 | ✓ | **3**（按钮组）| 0 | 无 |

跨断点对齐（1600 / 1400 / 1200 / 1100 / 950 / 860 / 700）：内容左边缘恒等于 `--shell-gutter`（64 / 40 / 28 / 20），`chromeH == wellTop`（104/104，700px 下 85/85），无横向溢出。

`npx vue-tsc -b` 与 `npx vite build` 均通过。

**实施中发现的两个问题（已一并修掉）**

1. **窄屏双重缩进**：`App.vue` 的 `@media (max-width: 768px)` 里有遗留的 `.main-content` / `.page-content { padding: 16px }`。它与页面自身的 `--shell-gutter` 叠加，使内容比外壳标题多缩进 32px。已移除，留白统一由 `--shell-gutter` 承担。
2. **窗口按钮让位随留白档位漂移**：动作区右边缘原本需要让开窗口按钮（fixed `right:10px`，宽 132px，左边缘恒在 `窗口宽 − 142px`）。用固定 padding 时，间隙会随 `--shell-gutter` 的档位变化（实测 1200px 宽下只剩 6px）。改成 `calc(162px - var(--shell-gutter))` 后，间隙在任何档位下恒为 20px。

**已知未做**

- ~~D2（切页保留页内状态与滚动位置）未定，内容层仍走 `:key="currentPage"` 换页。~~ **已在 §2.9.9 完成**：D2 = 是，改用 `<KeepAlive>`，`:key` 已移除。
- 逐页实测只覆盖 1200×750 与上表断点；**Foray 在浏览器里没有入口**（仅能经原生拖放进入），验证时通过开发模式下 Vue 在 `#app` 上暴露的 `__vue_app__` 调 `switchPage('foray')` 进入。Tauri 内的原生拖放路径未复测。
- 外壳高度目前由内容决定（104px @ 桌面）。若后续要固定壳高以获得更稳的连续性，需另行决定。

---

#### 2.9.9 D2 = 是：KeepAlive 保留页内状态与滚动位置 ✅ 已完成

> **决策记录** — D2 = **是**，2026-10-06。

**为什么不用 §2.9.6 原计划的「常驻 + `v-show`」**

计划里写的是"内容层改常驻 + `v-show` + 数值驱动"。实际落地选了 `<KeepAlive>`，理由：

| | 常驻 + `v-show` | KeepAlive |
|---|---|---|
| 转场机制 | 要重写成逐层驱动 | **`<Transition>` 结构不变**，四套预设与入场错峰逐帧一致 |
| `.page-shell` DOM | 需重构 | 类名挪到页面根元素即可 |
| 改动面 | shell + 5 个页面 | shell + 4 个页面 |
| 拿到 B 的能力 | 部分 | 部分（同一批）|

§2.9.6 里"C 是 B 的子集"仍然成立；KeepAlive 是达成 D2 目标更省的一条路。真正要搬到 B（跨页共享元素、连续量驱动）时仍需重写转场，但那是 M2 的事。

**结构**

```html
<Transition :name="animationStyle" :duration="transitionMs">
  <KeepAlive>
    <component :is="pageComponent" class="page-shell" v-bind="pageProps" ... />
  </KeepAlive>
</Transition>
```

`.page-shell` 这个类直接落在页面组件的根元素上（class 自动合并），外层不再有包裹 div。**顺序必须是 `Transition > KeepAlive > component`**。

**踩到的五个坑（都已修）**

1. **KeepAlive 放错层级 = 完全没缓存。** 最初的写法是 `<div class="page-shell" :key="currentPage"><KeepAlive>…`，而 `.page-shell` 每次切页都重建 → KeepAlive 实例跟着重建 → 缓存活不过一次切换。
   ⚠️ 而且这个错误**很容易被假阳性掩盖**：测试里"语言设置留下来了"看起来像缓存生效，其实是页面重挂载后从 `localStorage` 重新读的。验证时要用**非持久化**的状态（滚动位置、展开项）才能判真伪。

2. **`useAppChrome` 的所有权模型失效。** 原实现用 `Symbol` 做"最后注册者拥有"，前提是每次切页页面都重新 setup。KeepAlive 之后 setup 一辈子只跑一次 —— 外壳会一直显示**最后挂载那一页**的标题与返回行为（从 Foray 回到设置页，返回按钮仍走 Foray 的逻辑）。已改为**按页面 key 注册的注册表**，外壳按 `currentPage` 取；注册表是普通 Map，用一个 `registryVersion` ref 通知外壳重新求值（否则首次进入某页时 computed 会在页面 setup 之前缓存空标题）。

3. **Teleport 内容不会随页面失活撤走。** 设置页的搜索框、Foray 的动作按钮被缓存后，离开该页仍挂在外壳上（实测：在 Foray 上 `#chrome-center` 仍有 1 个元素）。修法是 `<Teleport :disabled="!chromeActive">`，`chromeActive` 由 `useAppChrome` 返回（`onActivated` / `onDeactivated` 驱动，初值 `true` 以免首帧内联渲染）。

4. **`onDeactivated` 里采不到滚动位置。** Vue 的 KeepAlive 是**先移走 DOM、再调用钩子**（`move()` 同步，钩子走 post-render 队列），等钩子跑起来 scrollTop 已经是 0。
   改为**持续记录**：捕获阶段监听根上的 `scroll`（scroll 不冒泡但捕获阶段经过祖先），激活后 `nextTick` + 一帧兜底写回。

5. **`.fanhua-home` 自带的 14s 无限动画骗过了 Vue 的转场完成检测。** Vue 读元素上的 transition/animation 时长来判断转场何时结束；无限动画永远不发 `animationend`，于是它只能退回"最长动画时长"的超时，转场类长时间不摘除。
   **这是既有缺陷**：改造前离场元素最终还会被删掉（只是晚十几秒），有了 KeepAlive 之后它会**永久留在 DOM 里**（opacity 0 不可见，但无限动画继续跑）。修法是给 `<Transition>` 显式 `:duration`（`transitionMs`，数值取自 `src/motion.ts`，与 CSS token 同源）。
   修完后实测：静止时 DOM 中恰好 1 个 `.page-shell`，无残留、无卡住的转场类。

**`onMounted` 语义变化 → 逐页迁移到 `onActivated`**

页面实例被缓存后 `onMounted` 只跑一次、`onUnmounted` 不再触发。逐页核对：

| 页面 | 原来在 onMounted 做的事 | 迁移 |
|---|---|---|
| ForayPage | 消费 `foray.pendingPath` | **必须迁**：否则第二次从主页拖入 zip 打不开（功能中断）|
| OverlayPage | `loadProjects()` | 迁到 onActivated，每次进入刷新列表 |
| ConversionPage | `loadOutputSettings()` | 迁到 onActivated（输出策略在设置页也能改）|
| SettingsPage | 动作监视轮询 `setInterval` | 拆成 `syncActionStatusTimer` / `stopActionStatusTimer`，`onActivated` / `onDeactivated` 管理；`onUnmounted` 仅兜底 |
| HomePage | 注册拖放监听 | 不动（一次性注册本来就是对的）|

**新增产物**

| 文件 | 内容 |
|---|---|
| `src/composables/usePreserveScroll.ts` | 捕获阶段记录滚动位置，激活后写回 |
| `src/composables/useAppChrome.ts` | 重写为按 key 注册的注册表，并导出 `active` |

**验证（全部 PASS）**

Teleport 不泄漏（离开设置页 `center` 0/1 正确翻转；离开 Foray `actions` 3→0）；四页返回均回到 home；DOM 中恒只有 1 个 `.page-shell`；入场错峰仍生效（`stagger-rise` + `--stagger-i`）；设置页滚动位置两次往返后仍为 1200；`vue-tsc -b` 与 `vite build` 均 exit 0；M0 的 16 项断言复跑全过。

**已知未做**

- 页面仍走 `<Transition>` 的离散进出，**转场中途反向仍会跳**（M2 的连续量原语才能解决）。
- KeepAlive 默认不设 `max`，5 个页面全部常驻。Foray 打开过大图时内存占用会随访问过的页面累积；后续若需要可加 `:max`。


## [S3] Out of Scope

- 三维 / WebGL / 相机 / 材质相关内容
- 视觉识别迁移（配色、字体、标志）—— 且参考项目许可证明确排除原作视觉设计
- Rust 侧（`src-tauri/`）改动
- `toast.html` 独立窗口的动效（独立生命周期，另案）
- 转换引擎性能（见 `perf-verification.md`）

---

## Tasks

### M0 — 与 shell 决策无关 ✅ 已完成

| # | 任务 | 判据 | 结果 |
|---|---|---|---|
| 0.1 | 新建 `src/motion.ts`：时长与缓动常量层 | 58 种时长 → ≤8；12 种缓动 → ≤5 | ✅ 6 档通用刻度 + 角色 token；缓动 4 条 + `linear` |
| 0.2 | 全局替换 `ease` 与秒/毫秒混用 | token 采纳率 11.7 % → >90 % | ✅ **100 %**（严格口径：时长与缓动都必须是 token）|
| 0.3 | 去重 `@keyframes` 收进 `shared.css` | 34 处定义 → 23 处 | ✅ 34 → **21**，另删 1 个死动画 |
| 0.4 | 速度档位下沉到 `:root[data-anim-speed]` | 「慢」档下弹窗/toast 同速变化 | ✅ 24 个 token 全量覆盖 |
| 0.5 | 补 `@update:animation-style` 的 emitter | 设置页可切换四套转场预设 | ✅ 新增「转场样式」行，四预设实测生效 |
| 0.6 | 遮罩去掉 `backdrop-filter` | 弹窗淡入无每帧全屏模糊 | ✅ 5 处全屏遮罩，运行时实测 0 个全屏浮层带 blur |
| 0.7 | 常驻装饰动画失焦时暂停 | 失焦后合成器可空闲 | ✅ `body.window-blurred` + 组件侧声明 |

#### M0.1 为什么不把全部时长都换成通用刻度

现有 14 个 `--motion-*` 角色 token（page / dialog / toast / stagger）是**按语义**命名的，比通用刻度更有价值 —— 「让弹窗整体快一点」只需改一个变量，而不必去追所有 `--dur-quick` 的调用点。因此**保留角色 token，只把长尾字面量收进通用刻度**，两层并存：

- `--dur-{instant,micro,quick,base,slow,emphasis,spin}` —— 通用刻度，覆盖绝大多数交互反馈
- `--motion-{page,dialog,toast,stagger}-*` —— 角色时长，有独立语义、需要单独调节

#### M0.2 脚本化替换踩到的坑（第一版脚本全中）

1. **`transition` / `animation` 简写里第二个时间值是 delay 不是 duration。** 第一版把 `animation: oobe-rise 0.55s … 0.18s both` 的 `0.18s` 当成时长改成 `--dur-micro`（180→160ms），错改了入场节奏。现在按**括号感知的 comma 分段**，每段只替换第一个时间值，`*-delay` 属性整体跳过。
2. **`cubic-bezier(a, b, c, d)` 自带逗号** —— 分段必须感知括号深度。
3. **`var()` 内部必须先占位保护**，否则 `var(--ease-out)` 会被二次替换成 `var(--var(--ease-out))`，fallback 里的时长也会被误改。
4. **`1s` 只在同一条声明含 "spin" 时才映射到 `--dur-spin`**，否则 `animation: ripple 1s` 这类一次性动画会被误标成加载指示器。

替换脚本：`.dsh-tmp/ui-review/m0-tokens.mjs`（带 dry-run，先审后写）。

#### M0 顺带发现

- `--motion-micro` 删除后仍有 3 处引用（`App.vue:1264/1694/1695`），会静默退化成无效声明 —— 已改为 `--dur-micro`。
- `slide` / `page-turn` 预设用 `transition: all …`，`fade` 预设用死值 `0.5s` —— 一并用 token 替换并去掉 `all`（`all` 会连带动画化布局属性）。
- `bg-breathe` / `aurora-drift` 在 `ConversionPage` 里的副本是死的（只被 HomePage 引用），且 **`aurora-drift` 两份内容不同** —— 不能盲目合并，只能删死的那份。
- `floatUp` 零引用。
- **子组件的根元素会继承父组件的 scope id**：`CreateProjectDialog` 等以 `class="dialog-overlay"` 作根，因此页面里 scoped 的 `.dialog-overlay` 实际是生效的。这正是 M0.6 必须连页面内那两份一起改的原因。

#### M0 验证

`vue-tsc -b` 与 `vite build` 均 exit 0。其余实测：5 个页面 shell 状态全部正常；转场样式四预设可切换并写入 `localStorage`，`App.vue` 的 `animationStyle` 同步更新；速率档位实测 `--dur-spin` 恒为 1000ms 不随档变化，而 toast / dialog token 随档变化（此前完全不变）；装饰动画失焦时 `animation-play-state: paused`；7 个场景（含 4 类浮层打开态）实测无任何全屏浮层做 `backdrop-filter`。

### M1 — C 方案落地（D1 已定）✅ 已完成

| # | 任务 | 依据 | 状态 |
|---|---|---|---|
| 1.1 | 新建 `AppChrome.vue` + `useAppChrome.ts`（composable 声明文本，`<Teleport>` 承载动作）| §2.9.3 | ✅ |
| 1.2 | 逐页接入，顺序 Settings → Conversion → Overlay → Foray | §2.9.7 | ✅ |
| 1.3 | Home 保持独立，验证 chrome 隐藏时 `.content-well` 占满全高 | §2.9.4 | ✅ |
| 1.4 | 删除各页 `.header-section` / `.settings-header` 残留 CSS 与 `margin-right: 140px` 补丁 | §2.9.1 | ✅ |
| 1.5 | 错峰改为显式索引（JS 计算或 `nth-child(n+11)` 兜底），解除 10 个上限 | §1.2 | ✅ |
| 1.6 | 内容层保留状态与滚动位置（**D2 = 是**）| §2.9.6 | ✅ 见 §2.9.9 |

实施记录与实测数据见 §2.9.8。

### M2 — 可中断动效

| # | 任务 | 状态 |
|---|---|---|
| 2.1 | 实现 §2.8 的 `useSurfaceTransition`（票据 + 实时读值）| ✅ |
| 2.2 | `useSidebarSlide` 迁移到该原语，去掉 `transition: none` 硬跳 | ✅ |
| 2.3 | `done` 由 `setTimeout` 改为 `finished.then()` + 票据校验 | ✅ |
| 2.4 | 引入 `damp()` 可重定向弹簧 | ⬜ 暂缓：目前没有非 3D 使用场景，先不引入未使用的工具 |
| 2.5 | 双票据（视觉 + 异步）覆盖 `invoke()` 驱动的面板 | ⬜ 暂缓：现有侧栏都由本地状态驱动，没有在途请求可守护 |
| 2.6 | 页面转场接入原语，实现中途反向 | ⬜ 见下方实测判断 |

#### M2 实施记录（2.1–2.3 已完成）

**产物**

| 文件 | 内容 |
|---|---|
| `src/composables/useSurfaceTransition.ts` | **新增** —— 通用可中断转场原语 |
| `src/composables/useSidebarSlide.ts` | 重写为薄封装（**导出签名不变**，5 个使用方零改动）|
| 5 个面板组件 | 面板节点 `v-if` → `v-show`（见下方"关键前提"）|

**四个机制**

1. **实时读值**：在 `cancel()` **之前**用 `getComputedStyle` 读回当前 transform 作为新动画的第一帧。
2. **revision 票据**（`WeakMap<Element, number>`）：每次启动递增，回调里比对，过期直接丢弃 —— 不需要队列或 pending 标志。
3. **WAAPI**：`finished` promise 取代 `setTimeout(330)`；`fill: 'both'` 保证取消瞬间状态有定义；`.catch(() => {})` 吞掉 `cancel()` 的 AbortError。
4. **时长取自 token**：`--dur-slow`（进）/ `--dur-base`（退），因此**跟随设置里的速率档位**（旧实现写死 320/240，完全不受档位影响）。

**关键前提：面板节点必须跨开合存活（`v-if` → `v-show`）**

这是实施中最关键的一条，靠诊断脚本才发现：

```
after open       : tag=FIRST
mid-close        : tag=FIRST   anims=1
right after open : tag=none    ← 元素被重建了
```

`v-if` 在 leave 期间保留旧节点，但重新 show 时 Vue **挂载的是新节点**。新节点的 transform 来自 CSS（`translateX(100%)`），旧节点上正在跑的动画随旧节点一起消失 —— 于是"读当前值"读到的是全关位，中断照跳不误（实测 77px → 420px，一帧跳 343px）。

改成 `v-show` 后元素跨开合存活，实测 `tag=FIRST` 全程不变，中断点 100.4px → 重开两帧后 80.3px，位移 4.8%。

**验证（全部 PASS）**

| 断言 | 实测 |
|---|---|
| 正常打开停在 `translateX(0)` | x=0.0 |
| 结束后无残留动画 | anims=0 |
| 关闭后 `display: none`（v-show 预期行为）| display=none |
| **重开未跳回全关位** | x=80.3 / w=420 |
| **重开与中断点连续** | Δ=20.1px（4.8%）|
| 中断序列逐帧峰值 ≤ 基线 1.6 倍 | 基线 83.7px vs 中断 33.8px |
| 法律侧栏（共用的另一个面板）开合正常 | display flex → none，无残留动画 |

> 逐帧峰值必须**与正常动画的基线对比**才有意义：ease-out 在 420px 行程的起步本来就有约 84px/帧，用绝对阈值会把正常的快起步误判成跳变。

#### M2.6 判断：暂缓，附实测依据

写了探针采样转场期间所有 `.page-shell` 的 opacity 之和（总可见量）：

| 场景 | 逐帧最大跳变 | 最小总可见量 |
|---|---|---|
| 单次转场中途反向 | 0.232 | **1.000**（无变暗）|
| 80ms 间隔快速连点三页 | 0.821 | **0.597**（明显变暗）|

结论：**单次中断没有问题**；只有低于约 100ms 的连续快速切换才会因为多个页面同时淡出而出现亮度塌陷。M2.6 要重写 `<Transition>` 与 KeepAlive 的配合（四套预设 + 入场错峰 + 实例缓存都要保住），风险远高于收益，且实测只在极端操作下可见。

建议：等真正要做 B 方案（跨页共享元素、连续量驱动）时一并重写，而不是现在单独改。

### M3 — 减少动效逐项语义 ✅ 已完成

| # | 任务 | 状态 |
|---|---|---|
| 3.1 | 按 §2.7 三段分类给全部动效打标 | ✅ 以「默认压平 + 两层豁免」实现，无需逐条打标 |
| 3.2 | 移除 `body.motion-reduced *` 全局 `!important` 一刀切 | ✅ 保留压平作默认，状态反馈与空间变化各有豁免层 |
| 3.3 | 保留「信息承载型短转场」豁免通道 | ✅ 转圈/进度保留；遮罩/面板保留 opacity 交叉 |
| 3.4 | 设置页文案说明每一类的影响 | ✅ 中英双语已更新 |

#### M3 实施记录

**为什么用「默认压平 + 豁免」而不是「给每条规则打标」**

打标意味着改上百处调用点，而且以后每加一个动效都得记得打标 —— 漏掉的那个就会在减少动效下继续动。反过来把**压平设为默认**，豁免只写在两个地方，新动效默认就是安全的。

| 层 | 判据 | 实现 |
|---|---|---|
| ① 装饰（默认）| 无需标记 | `body.motion-reduced *` 压到 0.01ms |
| ② 状态反馈 | `.spin` / `.ri-spin` / `.loading` / `[class*="spin"]` / `.ud-spinner` | 恢复 1s + infinite（`.ud-spinner` 保留自带 0.7s）|
| ③ 空间变化 | `.dialog-overlay` / `.sidebar-overlay` / `.dialog-content` | 遮罩保留 `--dur-micro` 的 opacity 过渡；面板**换 keyframes** |

③ 的关键细节：面板不是"把时长缩短"，而是 **`animation-name: overlay-fade !important` 强制换成纯透明度的 keyframes**。只缩时长的话 `panel-rise` 的位移会在 160ms 内快速走完 —— 位移依然存在，只是变快了，那恰恰是减少动效要消除的东西。换 keyframes 才是真正去掉位移。

侧栏面板走 WAAPI（`useSurfaceTransition`），其减少动效分支同样改成"位置瞬时 + 一次 opacity 交叉"，与上面两层对齐。

**实测（开启/关闭 `motion-reduced` 对照）**

| 元素 | 正常 | 减少动效 |
|---|---|---|
| `.blob`（首页光斑）| 24s | 1e-05s |
| `.fanhua-home`（背景呼吸）| 14s | 1e-05s |
| `.vortex-background`（极光）| 0.28s | 1e-05s |
| `.spin`（转圈）| 1s / infinite | **1s / infinite（保留）** |
| `.sidebar-overlay` | — | **0.16s / opacity（保留淡入）** |
| `.sidebar-content` | 滑入 | **瞬时到位，无残留动画** |

设置页文案实测渲染：「关闭或跟随系统时：入场错峰、光斑、模糊转场等装饰全部去掉；加载指示与进度保留（否则会被误读成卡死）；弹窗与抽屉改为瞬时落位并保留淡入。」

---

### M4 — 清理

| # | 任务 | 状态 |
|---|---|---|
| 4.1 | `slide` / `page-turn` 去掉 `all`，改显式属性列表 | ✅ 已在 M0 顺带完成 |
| 4.2 | `fade` 预设接入 token | ✅ 已在 M0 顺带完成 |
| 4.3 | 删除组件抽离后残留的死 CSS | ✅ 但**实际规模远小于预估**，见下 |
| 4.4 | 评估 `App.vue` 拆分 | ✅ 结论与预期不同，见下 |

#### M4.3 实施记录：原估「约 900 行」不成立

第一轮审查估的是"约 900 行死 CSS"（ConversionPage 1184-1454、OverlayPage 909-996、SettingsPage 1541-2235）。那个估算基于**模板字符串 grep**，而 grep 无法区分两种情况：

- 类名真的没人用 → 死代码
- 类名被**子组件**用了，但规则写在父组件的 `<style scoped>` 里 → **规则永远不生效**，可类名是有人用的

第二种占了绝大多数。按 grep 删会删掉本该**搬家**而不是删除的东西。

**两段式检测（宁可漏报，不可误删）**

1. **DOM 探针**：提取 1263 条选择器，在 18 个页面/浮层状态 × 4 个修饰态（`motion-reduced` / `ui-frosted` / 叠加 / 无）下逐一 `querySelectorAll`，全程 0 命中才算候选；剥离 `:hover`/`:disabled` 等状态伪类后再判定。
2. **零引用文本检查**：剥掉各文件的 `<style>` 块，再在**所有**文件的模板+脚本里找类名字面量。取选择器**最右侧复合体**的类名（选择器真正作用的对象）作判据。

**原始输出 639 条「死」，最终确认只有 40 条 —— 94% 是假阳性。** 来源：

| 来源 | 说明 |
|---|---|
| 转场类名 | `*-enter-active` / `*-leave-to` 只在中途存在，静止时探测不到 |
| 条件态 | `body.motion-reduced *`、`.has-background`、`:root[data-anim-speed]`、DEV 组 |
| 未访问的浮层 | 更新弹窗（需真实更新）、Foray 的 4 个模态（需真实资源包）、转换结果类弹窗 |
| 动态类名 | `:class="'toast-' + toastPriority"` 会产生 `.toast-safe`/`.toast-success`，字面量查不到 |

**两个差点误删的例子**（人工复核拦下）：
- `<textarea>`：探针零命中，实际有 3 处使用（ForayPage / SettingsPage / ImportShareDialog），只是那几个状态没走到。
- `.toast-success`：`App.vue:825` 用 `'toast-' + toastPriority` 动态拼出，字面量不存在但运行时确实会有。

**实际删除**：34 条整规则 + 6 条从逗号分组中摘除（−3899 字节），外加 3 个因此失去唯一引用者的 `@keyframes`（`fadeIn` / `slideIn` / `pulse`）。主要是 App.vue 里被移除的旧侧边导航残留（`.logo-text` / `.nav-text` / `.nav-item` / `.nav-menu` / `.nav-icon` / `.theme-toggle` / `.toggle-text`）、从未使用的 `.float-ripple-btn` / `.tooltip`，以及 `.status-*` / `.history-*` / `.dialog-button` / `.icon-button` 等。删完 keyframes 报表：无死、无重复。

> **真正的大块不是死代码，是「作用域写错」。** SettingsPage / ConversionPage / OverlayPage 里有一批对话框样式写在父组件 scoped 块中，而使用它们的是子对话框组件 —— Vue 的 scoped 只给子组件**根元素**加父作用域 id，嵌套元素拿不到，所以这些规则**从来没生效过**。典型是 `.dialog-hint` / `.dialog-label` / `.dialog-input` / `.switch` / `.slider`；第一轮审查里 BackgroundDialog 的开关渲染成原生复选框就是这个原因。
>
> 这批不能删（删了等于把 bug 固化），应当**搬到 `shared.css`**。但它涉及 4 套并存的 `.switch` 实现，搬完要逐对话框视觉复核，属于独立的一次改动。

#### M4.4 结论：App.vue 的问题不是「逻辑上帝对象」

实测构成：

| 文件 | 总计 | script | template | style |
|---|---|---|---|---|
| **App.vue** | 2027 | 735 | 147 | **1143（56%）** |
| SettingsPage | 2242 | 766 | 738 | 736 |
| ConversionPage | 1469 | 569 | 191 | 707 |
| StartupPage | 1311 | 159 | 27 | **796（61%）** |
| ForayPage | 980 | 329 | 244 | 405 |

原判断是「App.vue 与参考项目的 `main.ts` 同构（prefs + DOM 接线 + 转场 + 键盘 + rAF 全塞一处）」。**不完全成立**：它 56% 的行数是 CSS，脚本只有 735 行 / 18 个函数，规模仍在合理范围。

建议拆分顺序（按收益/风险比）：

1. **先抽 CSS（零行为风险）**：`:root` token → `src/styles/tokens.css`；外壳布局 → `shell.css`；四套转场预设 → `transitions.css`。一次去掉约 1100 行且不动任何逻辑。
2. 再抽脚本（中等风险）：`useShellWindow`（窗口控制/焦点）、`useUpdaterToast`、`useConversionGuard`、`useDevMode`。
3. 顺带处理同样"样式占大头"的 `StartupPage`（796/1311 行是 CSS）。

---

### M5 — 修正「作用域写错」（M4.3 的后续，已完成）

M4.3 结尾标记的那批"规则从来没生效过"的对话框样式，这一轮处理掉了。

#### 先确认了一件反直觉的事

原以为子组件的根元素会同时带上父组件的作用域 id（Vue 文档里"子组件根节点会同时受父子两边 scoped 样式影响"是这么说的），所以 `.background-dialog { width: 520px }` 这类应该生效。**实测不是这样**：

```
BackgroundDialog 渲染出的根元素：
  classes: "dialog-content background-dialog"
  data-v 属性: ["data-v-c2c72567"]     ← 只有它自己的，没有 SettingsPage 的 b7c75e2a
```

于是父组件里**所有**指向子组件的 scoped 规则都不生效 —— 包括宽度、内边距这些肉眼可见的。这也解释了为什么 `:deep()` 在这里救不了：`:deep()` 编译成 `.a[data-v-parent] .b`，而 `.a` 上根本没有 `data-v-parent`。

**唯一的正确修法是搬进子组件自己的 scoped 块，或放进 `shared.css`。**

#### 检测方法（源码解析看不到这个问题）

之前基于源码的选择器提取测的是**原始**选择器，而浏览器实际应用的是**编译后**的 `.foo[data-v-x]` —— 作用域 bug 对它是隐形的。改用 CSSOM：

1. 开发模式下每个 SFC 的样式是独立的 `<style data-vite-dev-id="<绝对路径>?vue&type=style&scoped=HASH">`，由此建立 **hash → 源文件** 映射
2. 取每条 scoped 规则**主体复合体**（选择器最后一段）的类名
3. 检查该类名是否出现在**该文件自己的**模板里 —— 不出现就说明元素属于子组件，规则必然落空
4. 再在浏览器里跨 17 个状态实测这批判定，把"子组件根元素（其实生效）"和"真的落不到"分开

> 踩坑：`data-vite-dev-id` 带查询串（`?vue&type=style&...`），直接用 `readFileSync` 会 35/37 全部读取失败 —— 而失败被 `continue` 静默吞掉，报出"0 条问题"这种看着最舒服的假阴性。

#### 处理结果

分两类，**不是一律删除**：

| 类别 | 判据 | 数量 | 处理 |
|---|---|---|---|
| 冗余副本 | 子组件自己定义了同样的类 | 84 | 删 |
| 真的丢了 | 子组件没定义 | 2 | **搬进子组件** |

"真的丢了"的只有两条，都是对话框正文的间距：

- `.background-dialog .dialog-body { gap: 14px }` → 移入 `BackgroundDialog.vue`
- `.naming-dialog .dialog-body { display:flex; flex-direction:column; gap:12px }` → 移入 `NamingDialog.vue`

其余所谓"丢失"的 29 条，实测全是 `*-enter-active` / `*-leave-to` 这类**转场类名** —— 只在动画期间存在，静止时探测不到，属于假阳性，一条都不能删。

#### 顺带修掉的真实缺陷

`.switch` / `.slider` / `.dialog-hint` / `.dialog-label` / `.dialog-input` / `.dialog-input-row` / `.dialog-checkbox-*` 这组原子，原先写在 SettingsPage 的 scoped 块里，**却没有任何子组件自带定义** —— 所以是实打实的样式缺失：

| 元素 | 修复前 | 修复后 |
|---|---|---|
| BackgroundDialog 开关 | `13.0x25.6`，滑块 `0x21` 透明，`appearance: auto`（原生复选框）| `42.0x24.0`，滑块 absolute `#e9e9eb` 34px 圆角，原生框 `0x0` opacity 0 |
| `.dialog-hint` | `16px` / `rgb(29,29,31)` | `12px` / `rgb(148,163,184)` |
| `.dialog-input-row` | `display: block` | `display: flex; gap: 10px` |
| `.dialog-label` | 浏览器默认 | `12px` / `rgb(100,116,139)` |

原子本身移入 `shared.css`（`.dialog-content` 等骨架本来就在那里），设置页自己那 6 个开关的取值**逐条照抄**，因此它们零变化：实测修复前后都是 `42x24 / inline-block / absolute / rgb(233,233,235) / 34px`。

#### 验证

- 作用域检测复跑：scoped 规则 994 → **901**；"主体不在本文件模板中" 119 → **24**（剩下 20 条全是转场类名，4 条为保守保留）；"冗余副本" 84 → **0**
- 运行时对照：子对话框开关达到与设置页自身完全一致；`ConversionPage` 的开关仍是自己的 `#e5e7eb`（scoped 特异性更高，全局规则不干扰）
- 全部回归重跑通过：M2 / M3 / M1.6 / shell 五页 / 作用域专项
- 9 张关键界面截图人工核对：BackgroundDialog、NamingDialog、HistoryDialog、ThemeDialog、FactoryResetDialog、版本侧栏、覆盖包页与新建项目对话框，均无掉样式
- `vue-tsc -b` 与 `vite build` exit 0

#### 规模

`ConversionPage` 45 条、`SettingsPage` 43 条、`OverlayPage` 7 条，共 95 条规则：4 个文件 **+347 / −887**（净减 540 行），`shared.css` 增加原子定义。

---

### M6 — 动效观感改造 ✅ 已完成

> **这一节回应的是「感觉动效没变」。** M0–M5 改的是**动效系统**（取消架构、作用域、死代码），
> 按定义就是不可见的 —— 参考项目 RhineLabUI 本身一个时长/缓动 token 都没有。
> 真正「看得见」的是节奏与动效密度，那一块此前完全没动，本节补上。

#### 先把两条主观感受量化

| 感受 | 实测 |
|---|---|
| 「不连贯」 | ① 页面容器与子元素**同时**跑 opacity，两层相乘，中途实际只有约 25% 亮度 ② 全项目 25 处 `transition: all` ③ 178 个元素在用浏览器默认 `ease`，与系统曲线的 1148 个混用 |
| 「动效很少」 | `:hover` 规则 **74** 条 vs `:active` 规则 **8** 条 —— 悬停有反应，按下去没有任何反馈 |

#### 根因：入场错峰其实根本没有位移

```css
/* 改造前 */
@keyframes stagger-rise {
  from { opacity: 0; }
  to   { opacity: 1; }
}
```

名字叫 rise，实际**只动 opacity**。所以进入页面时：

- 页面容器在淡入（`opacity` + `blur`）
- 子元素也在淡入（`opacity`）

**两层 opacity 嵌套相乘**，中途内容实际只有约 25% 亮度 —— 既糊，又因为两层做的是同一件事而完全看不出"错峰"。这一条同时解释了用户的两个感受。

原注释写了为什么不用 transform：给页面根加 transform 会让内部 `position: fixed` 的极光/光斑改为相对它定位，露出白边。

**解法是用 `translate` 独立属性，而不是 `transform`**：

```css
@keyframes stagger-rise {
  from { translate: 0 14px; }
  to   { translate: 0 0; }
}
```

`translate` 与 `transform` 是**组合**关系，所以 `.dock-wrap` 的 `transform: translateX(-50%)` 居中不会被顶掉（实测仍精确居中）；`translate` 也不让元素成为 fixed 后代的包含块，极光/光斑照常相对视口定位。

同时把 `animation-fill-mode` 从 `forwards` 改成 **`backwards`**：`forwards` 会把结束帧永久留在元素上（等于给每个子元素挂一个永久 transform），`backwards` 只在延迟期间套用起始帧，动画一结束就回到干净的基础样式。

#### 四个改动

**① 一条时间线：容器管淡入，内容管位移**

| 层 | 职责 | 时长 |
|---|---|---|
| 页面容器 | 页面之间的交叉淡化（opacity + blur）| `--motion-page-enter` 240ms |
| 内容子元素 | 落位（translate 14px → 0）| `--motion-stagger-base` 90ms 起，单项 260ms，步进 32ms |

内容刻意**晚于**容器起步：先让页面亮起来，再看到内容落位。两者职责不重叠，不再有两层 opacity。

实测时间线（进入设置页）：

```
t= 12ms  容器 opacity=0.00 blur(4px)     子 translate=14px
t= 77ms  容器 opacity=0.69 blur(1.24px)  子 14px      ← 容器先亮
t=144ms  容器 opacity=0.96 blur(0.17px)  子 5.62px    ← 内容接着落位
t=214ms  容器 opacity=1.00               子 0.98px
t=342ms  容器 opacity=1.00               子 0.0006px  ← 落位完成
t=409ms                                 子 none      ← 无残留
```

**② 清除 `transition: all`（25 处，含 4 处隐形写法）**

`all` 会把 padding / width / margin 这类**布局属性**也一起缓动，于是一次 hover 会连带把尺寸变化也动起来 —— 这正是"不连贯"的来源之一。全部换成显式属性列表，只列视觉属性。

其中 4 处是**裸时长**写法 `transition: var(--dur-quick)` —— 它等价于 `transition: all var(--dur-quick) ease`，是隐形的 `all`，靠关键字搜索根本找不到，是逐条看 CSSOM 输出才发现的。

**③ 补齐缓动**

178 个元素此前没有指定缓动，走的是浏览器默认 `ease`（一条很弱的曲线），与系统曲线的 1148 个元素混在一起。现在全部显式指定。

结果：缓动种类 **4 → 3**，浏览器默认 `ease` **178 → 0**，系统曲线占比 **83% → 97.6%**。

> 检测这类问题要注意：**CSSOM 序列化时会省略 `ease`**（它是初始值），所以「没写缓动」和「写了 ease」在样式表里长得一模一样，必须回到源码查。
> 另外按逗号切分 transition 必须考虑括号深度，否则 `var(--x, fallback)` 会被切断而误报。

**④ 补齐按下反馈**

`:active` 此前只有 8 条（而且集中在启动页）—— 鼠标移上去有反应、按下去毫无回应，这是"不跟手"最主要的来源。

新增全局按下规则，覆盖 `.btn-text / .ghost-btn / .seg-btn / .dock-item / .action-tile / .setting-item.clickable / .version-row / .item-row / .dialog-close / .sidebar-close / .primary-btn` 等主要交互元素。用 **`scale` 独立属性**而非 `transform: scale()`，理由同 ①：与已有的 transform 组合而不覆盖。控件按下 0.97，大块卡片 0.99（否则像整页在晃）。

**⑤ 外壳标题随之交叉淡化**

外壳在页面转场之外，此前切页时标题是**瞬变**的。实测中间帧里标题已经完全清晰，而页面内容才到 0.69 不透明度 —— 这是最后残留的一处割裂。

过程中踩了两个坑：

- 先用 `mode="out-in"`：离场与进场之间有一帧两边都不存在，**外壳高度从 104px 掉到 74px，下方内容跟着跳**。改成同时叠放 + 离场标题绝对定位移出文档流，高度全程稳定在 104px。
- 新页面**尚未注册**的那一帧 `title()` 取到空串，加了淡化后会变成"旧标题淡出 → **空标题**淡入 → 真标题"。改为保留最后一份有效文案兜底。

#### 验证

| 项 | 结果 |
|---|---|
| 错峰位移 | 四个页面**每次**进入都是 14px 峰值（含 KeepAlive 二次进入）|
| 静止后残留 | `translate: none`，无永久 transform |
| `.dock-wrap` 居中 | 未被 `translate` 破坏 |
| 按下反馈 | `:active` 时 scale 0.97/0.99，松开回 1 |
| `transition: all` | 0（含 4 处隐形写法）|
| 缺失缓动 | 0 |
| 外壳标题 | 有中间态、高度不跳、从不为空 |
| 全量回归 | M2 / M3 / M1.6 / 作用域 / shell 五页 全部通过 |

#### 已知未做

- **按下反馈的「松开」在部分页内元素上是瞬时的**：这些元素的 `transition` 里没有 `scale`，而 CSS 无法向已有的过渡列表追加属性（会整体覆盖）。`shared.css` 里的原子已加；页内自定义元素要各自补。
- 错峰仍然只作用在 `.page-shell` 的**直接子元素**上。页面内部的卡片（如设置页各分组）不参与 —— 要做需要先解决 fixed 定位与逐层错峰的冲突。

---

### M7 — 视觉语言改造（参照 RhineLabUI 的设计，非其素材与标识）✅ 已完成

> 2-Pyramid 有自己的 logo 与品牌，这里取的是**设计语言**，不是视觉识别。
> 仓库的 MIT 只覆盖作者自己的代码；Arknights / Rhine Lab 的视觉识别、MiSans 与
> Novecento 字体在 README 里被明确排除，一律未使用。

#### 先把参考项目的语言量出来

从 `src/theme-ui.ts`、`src/style.css` 与截图里提取到的硬事实：

| 维度 | 事实 |
|---|---|
| 调色板 | 7 个角色（ink / muted / line / paper / panel / field / accent），每个都配一份 `-rgb` 伴随变量用于 `rgba(var(--x-rgb), α)` |
| 明暗 | `ink #080a08/#e0e3dc`、`paper #eae5e1/#11181b`、`panel #edebe4/#202a2f`、`field #e7e3d9/#2a363b`、`accent #9b7247/#c5a16b` |
| 圆角 | **全项目 `border-radius` 只出现过 `0` 与 `50%`** —— 全方角 |
| 结构 | 行与行之间一条 1px 横线（ruled rows）；层级靠线，不靠投影 |
| 字号 | 密集在 10–13px，标签 `letter-spacing: 1px`（22 处），字重以 400 为主 |
| 面板 | 完全扁平，无背景色差、无阴影；只有**交互控件**才有描边 |
| 选中态 | 实心反色（`background: #171713; color: #ece9e4`），不是主题色染色 |
| 切换 | 根元素一个属性切换整张调色板（`data-dark-surface`） |
| 开关 | 方形轨道 + 方形滑块，比常见尺寸小一号 |

**顺带发现：它自己的可达性并不达标** —— 浅色 `muted #77756d/paper` 只有 3.69:1、
`accent #9b7247/paper` 3.43:1，都没到 AA 的 4.5。所以我们取的是它的**结构与气质**，
色值按 AA 重新标定。

#### 我们的调色板（同结构，自己标定）

对比度全部用脚本校验（`.dsh-tmp/ui-review/palette.mjs`），**浅色 8 组、暗色 8 组全部 ≥4.5**：

| 角色 | 亮色 | 暗色 |
|---|---|---|
| ink | `#12140f` | `#e0e3dc` |
| muted | `#66655d` | `#a3acad` |
| line | `#b3aea2` | `#4e5a5f` |
| paper | `#efece6` | `#131a1d` |
| panel | `#f3f1ea` | `#1f272b` |
| field | `#e8e4da` | `#293338` |
| accent | `#86602f` | `#c6a06a` |

另外补了 `code-bg` / `code-ink` 两个角色：代码块不能用 `ink` 当底色 ——
暗色模式下 `ink` 会翻成近白，白底代码块是错的。

#### 关键手法：保留 alpha，只换颜色

```
rgba(0, 0, 0, α)        →  rgba(var(--c-ink-rgb), α)
rgba(255, 255, 255, α)  →  rgba(var(--c-panel-rgb), α)
```

亮色下 `ink` 近黑、暗色下 `ink` 近白，于是**同一句「叠一层 6% 的墨」在两个模式下都成立**，
不需要写第二套规则。这一条让 171 + 58 处叠色自动获得明暗适配。

#### 三个必须拦下的坑

1. **`#000` 大多出现在 `color-mix(..., #000)` 里**（用于压暗主题色，104 处）。
   换成 `ink` 会在暗色模式下反向变成**提亮**。已跳过 `color-mix()` 与 `gradient()` 内部。
2. **不能只对全文件做替换** —— 模板里有 `stroke="#000"` 的 SVG，脚本里有 `'#000000'`
   的调色数据。改为**只在 `<style>` 块内替换**。
3. **`index.html` 的启动画面定义了 `:root { --theme-color: #007bff }`**，它的位置在应用
   样式之后，同特异性的 `:root` 会由它胜出 —— 换掉默认色之后运行时**仍然是蓝的**。
   启动画面改用自己的一套 `--boot-*`，不再定义全局 token。

#### 落地范围

| 项 | 前 | 后 |
|---|---|---|
| 硬编码颜色 | 912 处 | **157 处** |
| token 引用 | 888 处 | **1589 处** |
| 圆角取值 | 10/12/8/14/20/999px… | **只剩 `0` 与 `50%`** |
| 分组卡片 | 玻璃浮起（背景+模糊+描边+阴影）| **扁平，结构由横线承担** |
| 卡片悬停 | 上浮 2px + 加重投影 | **描边加重**（浮动卡片 → 线条结构）|
| 暗色模式 | 无 | 亮 / 暗 / 跟随系统 |

#### 已知未做

- 中文字体未换（MiSans 被仓库排除，也不能用）。字重与字距向参考设计靠拢了，
  但没上压缩字体，所以标题没有那种「窄体超粗」的压迫感 —— 那需要另找一套可用授权的中文压缩字体。
- 图标列仍然保留（参考设计是纯文本行）。2-Pyramid 的内容更适合留图标，已把图标收小并转为 muted。

---

### M8 — 收尾三项 ✅ 已完成

M7 遗留 + M6 遗留，一次做完。

**① 界面配色入口**

M7 里 `applyThemeMode` 已接好并落 localStorage，但设置页没有那一行。现在补上：
「全局设置」组内新增 `界面配色`（亮色 / 暗色 / 跟随系统），走 props + emit，
与「动画速率」同一套模式。中英双语齐全，并加入了设置搜索索引。

实测（`.dsh-tmp/ui-review/verify-theme-mode.mjs`，全部 PASS）：

| 断言 | 结果 |
|---|---|
| 切暗色：`data-theme=dark` | ✓ |
| 切暗色：纸底 `#efece6 → #131a1d`、body 底色跟随 | ✓ |
| 切暗色：强调色 `#86602f → #c6a06a` | ✓ |
| 跨页保持 | ✓ |
| **刷新后仍是暗色**（持久化） | ✓ |
| 跟随系统：`data-theme=system` | ✓ |
| 切回亮色 | ✓ |

**② 按下反馈的「松开」不再瞬跳**

M6 里按下用 `scale`，但页内元素的 `transition` 列表里没有 `scale` —— 按下去有反馈、
松开会瞬跳回。CSS 无法向已有的过渡列表追加属性（只能整体覆盖），所以逐个定位并改写：
7 个文件、13 处补上 `scale var(--dur-instant) var(--ease-out)`。

**③ 第二层错峰**

原先错峰只作用在 `.page-shell` 的**直接子元素**上，卡片是静止出现的。现在加了第二层：

```
第一层  容器        0.090s 起
第二层  容器内的卡片 0.122s 起   ← 晚一个节拍
```

不给每张卡再编号是刻意的：两级已经形成「容器先到、内容随后」的两拍节奏，
再细分会让几十张卡各走各的，反而更碎。

选择器用 `> * .card`（至少一层嵌套）而不是裸 `.card`，避免把已被第一层接管的卡片重复计算。

⚠️ 这里的关键风险是**顶掉装饰动画**：`animation` 是单值属性，新规则会覆盖元素原有的动画。
实测确认首页的 `.blob`（`blob-float` 24s）与 `.breath-dot`（`breath` 2.8s）都完好无损 ——
因为它们不是 `.card`。已写成断言固化下来。

**回归**：M0 / M2 / M3 / M1.6 / 作用域 / 动效 / 外壳标题 / 配色模式 / 二层错峰 / shell 五页 —— 全部通过。

---

### M9 — 图标梳理 ✅ 已完成

先审计现状：全项目 **149 个 `-line` 图标，零 `-fill`**（那 11 个 `ri-spin` 是动画类名不是图标），
28 组「图标 ↔ 标签」**每个图标各用一次、无重复**。所以不存在「两套图标混用」的问题，
底子是好的。真正要动的是选得准不准、以及跨页的一致性。

**① 语义修正（4 处）**

只改确实含糊的，其余保持：

| 行 | 原图标 | 问题 | 改为 |
|---|---|---|---|
| 控件样式 | `ri-contrast-2-line` | 「对比度」表达不出玻璃/磨砂的**透光质感** | `ri-contrast-drop-2-line`（半透明水滴）|
| 界面配色 | `ri-sun-line` | 这项含亮/暗/跟随三种，单一个太阳只覆盖「亮」 | `ri-contrast-line`（半明半暗圆）|
| 通知位置 | `ri-layout-masonry-line` | 瀑布流布局表达不出「落在屏幕哪一角」 | `ri-layout-line`（四象限）|
| 界面动画 | `ri-magic-line` | 魔法棒偏随意 | `ri-animation-line`（专用动画图标）|

换名前逐个核对了打包的 `remixicon.css` —— 名字写错会渲染成空白方块。

**② 统一行内图标的颜色角色**

`.item-icon` 这个名字在四个文件里出现，但其实是**三种不同的东西**：

| 用途 | 现状 |
|---|---|
| 设置行标签图标（SettingsPage，36 处）| 30px 列 / 17px / `--c-muted` |
| 文件行图标（ConversionPage / ItemsListDialog）| 18px / `--c-ink` ← **与文件名同重，应该退到背景** |
| OOBE 徽标（StartupPage，9 处）| 36px 带底色方块 / 主题色（刻意保留，是引导流的设计装置）|

前两者统一到 `--c-muted`。

**③ 顺带挖出两个真 bug**

**(a) `ItemsListDialog` 的 `.item-icon` 从来没有样式。**
它自己定义了 `.item-row` / `.item-name` / `.item-actions` / `.item-size` / `.remove-item-btn`——
**唯独漏了 `.item-icon`**，指望 `ConversionPage` 的 scoped 规则，而那条到不了子组件内部。
结果文件图标既没有 18px 尺寸也没有 muted 颜色。已补进它自己的 scoped 块。

**(b) 8 个「删除 / 重置」按钮和普通按钮长得一模一样。**
`class="btn-text danger"` 全项目用了 9 处，其中 8 处在子对话框里
（FactoryReset / ClearConfig / DeleteProject / DeleteSource / BedrockWarn / Background / LogWindow）。
`.btn-text` 在 shared.css（生效），`.danger` 只写在父页面的 scoped 块里（不生效）——
于是**「重置全部配置」的确认键与「取消」视觉上无差别**。这是安全性问题，不只是美观。

修法同 M5：这类原子属于 `shared.css`，与 `.btn-text` 放在一起。
实测：取消 = `rgb(102,101,93)`（muted），删除 = `rgb(148,49,41)`（危险色），
悬停有危险色浅底。

**(c) `VersionSidebar` 缺 `.version-row.has-status`。**
`.version-row.has-status { padding-right: 8px }` 写在 `ConversionPage` 的 scoped 块里，
但行是 `VersionSidebar` 渲染的 —— 同样从来没生效。已搬进子组件。

**④ 新增一个检测器：`.dsh-tmp/ui-review/orphan-class.mjs`**

M5 的检测器找的是「规则匹配不到元素」，对 `.item-icon` 这种**反过来的情况是盲区**：
子组件用了某个类，却没有任何地方为它定义样式。新检测器专门找
「只在**别的** .vue 的 scoped 块里定义过」的类——正是作用域够不着的情形。

首轮报 8 处（6 处 `.danger` + `.has-status` + 一处误报），修完降到 1 处已知误报
（StartupPage 的 `.step` —— 它实际用的是 `.step-title` / `.step-desc`，都已定义）。

---

## 验收方式

| 指标 | 现状 | 目标 |
|---|---|---|
| token 采纳率 | 11.7 % | > 90 % |
| 时长种类 | 58 | ≤ 8 |
| 缓动种类 | 12 | ≤ 5 |
| `@keyframes` 定义数 | 34 | 23 |
| `ease` 占比 | 53 % | 0 % |
| 秒/毫秒混用 | 存在 | 消除 |
| `transitionend` 依赖 | 0（但用 `setTimeout` 代替）| 0 且无 `setTimeout` 完成回调 |
| 打断测试 | 侧栏关闭中途重开会回跳 | 连续 5 次快速切换，无回跳 |
| 速度档覆盖 | 仅页面转场 | 全部动效 |
| reduced-motion | 全局压零 | 三段分类生效，状态反馈保留 |

**打断测试做法**：对每个可关闭容器（侧栏、弹窗、抽屉）在其动画进行到约 40 % 与 80 % 时触发反向，采样 `getComputedStyle().transform` / `opacity` 检查是否存在跳变。参考项目有现成的对照页思路可借鉴（`reference/ui-transition-check.html`：读取前值、中途打开、断言增量连续）。

---

## 审查时请确认

1. ✅ **D1 shell 模型 = C（常驻外壳 + 内容井）** — 已定（2026-10-06）。落地结构见 §2.9。
2. **D2 切页是否保留页内状态与滚动位置？** C 已让这件事变可行（§2.9.6），但需要内容层改常驻 + `v-show`。若接受，M1.6 一并做；若不接受，C 先按「内容层仍交换」落地，后续再升级。
3. **D3 header 外提**已由 D1 隐含接受（§2.9.7 会改动四个页面的结构）。请确认 Foray / Overlay 的动作按钮用 `<Teleport>` 的方式可以接受。
4. **D4 是否认同「借架构不借节奏」？** 参考项目是六屏展示型应用，其 320ms 页面转场与 300ms 延迟入场级联在工具类应用里会明显拖慢高频往返。M0 的归并表已按工具场景重定档。
5. **D5 M0 是否先做？** M0 与 shell 决策完全解耦，且能把 token 采纳率从 11.7 % 提到 90 % 以上，无论 C 怎么落地都不返工。其中 0.6（遮罩去 blur）与 0.7（失焦暂停装饰动画）是独立的性能收益。
6. **D6 减少动效档位**：维持现有三态（开/关/跟随系统），还是引入参考项目那样的 14 项分组开关？后者工作量更大，但对「转换进度要动、极光不要动」这类诉求更准确。建议先按 §2.7 的三段分类落地，分组开关留到有实际诉求时再说。

> 建议执行顺序：**M0（解耦，可立即开始）→ M1（C 落地）→ M2（可中断原语）→ M3（减少动效语义）→ M4（清理）**。
> M0 与 M1 可由不同人并行，但需约定 `src/motion.ts` 与 `AppChrome.vue` 的写入范围不重叠。

---

## 附录 A：证据索引（2-Pyramid）

| 主题 | 位置 |
|---|---|
| 换页舞台 + `:key` | `src/App.vue:705-730` |
| 页面转场绝对定位 | `src/App.vue:1471-1518` |
| 入场错峰 + `nth-child(1..10)` | `src/App.vue:1528-1544` |
| 速度档位（仅 6 token）| `src/App.vue:1546-1562` |
| 减少动效全局压零 | `src/App.vue:870-889` |
| 动效偏好判定 + 系统监听 | `src/App.vue:126-131`、`629-633` |
| 4 套转场预设 | `src/App.vue:1450-1466` / `1578` / `1616` |
| 17 个动效 token 定义 | `src/App.vue:847-866` |
| 侧栏抽屉中断缺陷 | `src/composables/useSidebarSlide.ts:16-42, 61-68` |
| 绕开全局错峰的注记 | `src/composables/useSidebarSlide.ts:4-7` |
| 遮罩全屏模糊 | `src/styles/shared.css:136-145` |

## 附录 B：参考项目证据索引（RhineLabUI @ main）

| 主题 | 位置 |
|---|---|
| 票据 + 实时读值核心 | `src/ui-transitions.ts:34-58` |
| 默认 enter/exit 与位移 | `src/ui-transitions.ts:12-13, 40, 74` |
| `ContentTransition` 续接 | `src/ui-transitions.ts:87-98` |
| 双票据（视觉 / 异步）| `src/model-viewer.ts:305` / `:251` |
| 中断时读回当前淡入值 | `src/model-viewer.ts:353-357` |
| 拆除延后到退场之后 | `src/model-viewer.ts:393-413` |
| 距离错峰 + 冻结原点 | `src/theme-motion.ts:13-32` |
| 可重定向弹簧 `damp()` | `src/motion.ts:108-118` |
| epsilon 吸附 | `src/motion.ts:100-107` |
| 14 标志 / 5 组定义 | `src/motion-preferences.ts:1-98` |
| 优先级链 | `src/main.ts:158-163` |
| 派生预设 | `src/motion-preferences.ts:142-146` |
| 全局压零（反模式）| `src/style.css:1771-1776` |
| 全屏遮罩 blur（反模式）| `src/style.css:945-950` |
| 上帝模块（反模式）| `src/main.ts`（1337 行）|

**复现命令**

```sh
node .dsh-tmp/ui-review/motion-inventory.mjs code/.worktrees/motion/src
```
