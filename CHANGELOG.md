## [Unreleased]

（暂无）

## [2.5.0] - 2026-09-24（BUILD 20048）

### Added

- **Foray（Editor Mode）分析工作台**：设置开启 EM 后，主页拖入 zip 进入 Foray。ROM 树、五探针、轻量像素编辑、OpenAI 兼容 AI（档位 0–5）。默认关闭。详见 `docs/compose/spec/foray.md`。
- **MSI 安装包（固定名 `2-Pyramid-Installer.msi`）**：WiX x64；静默 `msiexec /i 2-Pyramid-Installer.msi /qn`。**不做 MSIX**。
- **安装器静默参数**：`--silent|/S|-s|--quiet` 等；`--dir`、`--relaunch`、`--shortcuts`、`--help`。
- **系统通知 Rust 路径**：`show_system_notification`（notify-rust / winrt）。

### Changed

- 打包线不再产出固定名 `2-Pyramid-Installer.exe` 别名；部署请用 **MSI** 或版本化 exe。

### Fixed

- **通知弹不出**：toast capability / 透明窗 / system 回退。
- **Foray**：导出字节保留与回滚；问题/编辑/报告弹窗；灯箱缩放；返回键；EM 分流。

### Security

- legal/（中英）：Foray AI 外部 API 免责与隐私；MSI 静默部署说明。

## [2.4.1] - 2026-09-18（BUILD 20046）

### Added

- **1.21 旋风系贴图生成（Tricky Trials）**：升版至 format 34+（1.21）时从包内已有材质近似生成——`breeze_rod`（←烈焰棒）、`wind_charge`/投掷物（←雪球）、`trial_key`/`ominous_trial_key`、`breeze_spawn_egg`、`wind_charged` 效果图标、`copper_bulb` 族（←红石灯/铜块，含氧化与蜡变体）；中等项：`heavy_core`（←铁块/深板岩）、`breeze` 实体（←烈焰人）、`flow` 纹饰模板、`ominous_bottle`。目标已存在则不覆盖；reverse(34→32) 删除。**不做** mace / trial_spawner / vault / crafter / 粒子（轮廓独特）。依据 1.21.11 原版资源均色与 Wiki。
- **2.4.0 发版说明补录 FolderOpenPatch**：见 `docs/compose/releases/2.4.0.md`（Fabric 补丁 mod，修 MC-311807）。

### Fixed

- **着色器适配：按 Minecraft Wiki 清理已移除的核心程序**：转换后资源包/材质重载失败的主因之一——目标版本已删除的 `shaders/core` 程序仍被打进包里。`adapt_java_shaders` 现按 Wiki 白名单处理：≥63（1.21.6）把 `rendertype_solid/cutout/…` → `terrain`、`rendertype_entity_*` → `entity`、glint 族 → `glint` 等；≥84（26.1）`rendertype_translucent_moving_block` → `block`、删除 `entity_alpha/decal`；≥97（26.3）`rendertype_clouds` → `clouds`、`rendertype_world_border` → `world_border`、删除 `text_background*`，并把 `#moj_import` 转为 `#include`（路径按 Wiki include 解析校正）。白名单外核心文件删除；旧 `shaders/post` 与现代 `assets/*/post_effect` 按目标只保留一侧；1.21.6+ 去掉 core JSON 中过期的 `uniforms`；include 文件补末尾空行（Wiki：否则着色器不加载）。


## [2.4.0] - 2026-09-16（BUILD 20045）

### Added

- **Java 26.3（Wilderness Bound，pack_format 97）**：版本选择器新增目标；与 26.2 之间走 `adapt_java_shaders` 边。Bedrock 中间态同步升到 26.3。
- **选择 26.3 时弹出已知问题确认**：材质包目录按钮无响应 / 未响应（MC-311807），需确认才继续。
- **FolderOpenPatch（Fabric 补丁 mod）**：配套修复上述 MC-311807——Mixin 拦截 `Blaze3D.openPath` 与 `PackSelectionScreen` 按钮回调，独立 daemon 线程在 Windows 走 `explorer.exe` 异步打开（macOS/Linux 分别 `open` / `xdg-open`），绕开 `SDL_OpenURL` / ShellExecute 阻塞点。依赖 Fabric Loader ≥0.19.5 + Fabric API `0.160.5+26.3`（Minecraft ~26.3 / Java ≥25）；产物 `folder-open-patch-1.0.0.jar`，见独立仓库 FolderOpenPatch。
- **自动检查更新开关**：设置 → 版本；关闭后启动不再自动查更新。OOBE 第 4 步可一并配置更新通道与该开关。
- **Poplar 木种生成（26.3）**：从橡木/丛林木生成全套——原木/木板/去皮、门/活板门/告示牌/悬挂告示牌/船/shelf/树苗，以及三色树叶近似。挂 `(88→97)`；reverse 降版时删除。基岩别名 `planks_poplar` 等已映射。垫子/层架蘑菇/红灌木不自动生成。

### Fixed

- **26.3 pack.mcmeta 兼容**：目标 ≥69 时只写 `min_format`/`max_format`（26.3 为 `[97,0]`–`[97,1]`），去掉 `pack_format:34` + 跨大版本 `supported_formats`（会导致游戏判不兼容）。
- **Poplar 树叶色相**：不再相对 shift（橡木叶近灰会漂成蓝紫），改为钉在红 8° / 橙 28° / 黄 45° 并强制饱和。
- **debug 编译**：`ClientToScreen` / `RGBQUAD` 的 windows-sys 路径修正（`cfg(debug_assertions)` 动作监视注入与截图）。

## [2.3.1] - 2026-09-15（BUILD 20044）

### Changed

- **物品语言编辑改为宽面板双列平铺**：去掉与全局 `.dialog-*` 撞名的类，避免被居中弹窗皮肤挤成「大卡片 + 竖排一行一条」；条目改为紧凑双列卡片（短 key + 输入 + 预览）。

### Fixed

- **Overlay 三个编辑子页排版**：`ItemName` / `ItemSize` / `Visual` 改用独立类名（`name-panel` / `size-panel` / `visual-panel`），不再继承全局居中弹窗样式。
- **Overlay 弹层退出黑幕残留**：leave 时遮罩先透明、再滑出面板，并关掉 `backdrop-filter` 与 enter 动画。
- **法律信息侧栏点外关闭**：补齐 `.sidebar-overlay` 遮罩样式（此前无定位/背景，点空白无效）。
- **安装器已安装即进覆盖更新**：不再强依赖 `--from-app`；双击新安装包时同样进入 3 步更新流，而不是完整全新安装向导。
- **debug 编译**：`ClientToScreen` / `RGBQUAD` 的 windows-sys 路径修正（`cfg(debug_assertions)` 动作监视注入与截图，release 不编译，`cargo check` / dev 才暴露）。

## [2.3.0] - 2026-09-15（BUILD 20043）

### Added

- **应用内覆盖更新向导**：主程序拉起安装器时带 `--from-app`；检测到已安装时进入 3 步更新流（确认 → 覆盖中 → 完成），锁定原安装目录，保留用户数据与快捷方式；主程序仍在运行时提示退出。
- **按 major.minor.patch 判定更新级别**：`bumpKind` 区分 major / minor / patch；**major 与 `Safe-*` tag 强制更新**，minor / patch 为可选。
- **更新弹窗级别徽章**：重大 / 功能 / 修复，并附简短说明。
- **设置页版本与作者摘要**：列表右侧显示 `v{version}+{build}` 与主作者。

### Changed

- **前端模块化**：Settings / Conversion / Overlay 弹窗与版本侧栏拆到 `settings/`、`conversion/`、`overlay/`；Minecraft 版本目录独立为 `data/minecraftVersions.ts`。
- **主页底部 Dock / OOBE 药丸**：保留上浮，曲线更稳，去掉缩放弹跳。
- **覆盖包返回层级**：编辑器 → 项目列表 → 首页，不再从编辑器直接回主页。
- **开发者选项 UI**：动作监视状态条（录制指示、帧数、实时流端口）。
- **依赖安全**：`vue-i18n` 10.0.8（CVE-2025-53892）、`vite` 6.4.3（多条 dev-server 路径穿越）、`rustls` 0.23.45（RUSTSEC-2026-0285）。

### Fixed

- 移除 `vite.config.js`（`vue-tsc -b` 误产物，且 Vite 会优先加载它）。
- Overlay 页误删的自定义内容侧栏恢复挂载。
- 动作监视协议补全：`SHOT` / `CLICK` / `STATUS` / `CLEAR` / `MON ON|OFF`；导出 `.2amr` 写入真实视口尺寸与多事件类型（click/input/change/keydown）。

### Removed

- 未引用组件：`OtherOptionsDialog` / `BorderColorDialog` / `CustomNameDialog`。

## [2.2.0] - 2026-09-14（BUILD 20042）

### Added

- **法律与合规文档**：`legal/` 下新增 EULA、隐私说明、免责声明、第三方组件声明、安全策略与贡献指南；README 增加索引。
- **安装器「使用须知」步骤**：安装前可滚动阅读；勾选「已阅读并知悉」后继续（阅读确认，不附加 MIT 之外的许可限制）。
- **法律文件随安装包释放**：安装后位于 `<安装目录>/legal/`。
- **设置 →「法律信息」**：独立设置项；右侧滑出侧栏，可在 6 份文件中选择阅读，也可打开 legal 文件夹。
- **`get_install_dir` / `read_legal_file`**：定位安装目录、读取法律文本。

### Changed

- 产品标语与 i18n 润色（见 2.1.4）；`package.json` license 字段修正为 MIT。
- `npm test` 使用 `--manifest-path`，Windows 下更稳。
- 取消跟踪 Tauri 生成的 `gen/schemas`；`.gitignore` 补全运行时文件。

### Fixed

- 安装器安装成功后停在 100% 不进入完成页。
- 安装失败重试返回到正确的「安装中」步骤。
- 版本详情弹窗恢复原布局（法律入口移出后不再挤版）。
- 修正作者信息：五香蛋e 标注为「前项目发起者，现项目灵感提供者」；作者列表拆为独立设置项。

## [2.1.4] - 2026-09-14（BUILD 20040）

### Added

- **覆盖包分享码 2PYR-**：前缀改为 `2PYR-`；载荷先剔除 false/null/空对象再 zlib + URL-safe base64，文本更短。旧 `HRCN-` 码仍可导入。

### Changed

- **产品标语**：中文「现代化通用多版本资源包转换器」；英文 `Modern Universal Multi-Version Resource Pack Converter`。
- **i18n 用语**：统一「资源包」而非「材质包」；弱化基岩测试警告措辞；缩短导入清单/进度等短文案；OOBE 与主页标语对齐。

## [2.1.3] - 2026-09-14（BUILD 20039）

### Added

- **静默更新**：应用内更新不再弹出安装向导；下载完成后一键「应用并重启」，安装器以 `--silent --dir <当前目录> --relaunch` 覆盖文件并自动启动新版本。
- **着色器适配实验开关**：转换页可关闭 `adapt_java_shaders`（默认开启）。关闭后原样保留 shaders，便于对比试转。
- **`npm test` / `npm run test:offline`**：一键跑 Rust `cargo test --lib`。

### Changed

- **converters 按领域分模块**：`ui/` `textures/` `reverse/` `color/` `audio/` `shaders/`，文件名去掉 `fix_`/`generate_` 等 Python 前缀。
- **hurray 轻量优化**：Task 名 `Arc<str>`、分层执行少克隆、贴图缓存 `Arc<RgbaImage>`。
- **创造栏 tabs 切片对齐 pack.py**：始终裁 168 宽、6 片，第 7 = 第 6；非 256/512/1024/2048 尺寸跳过。

### Fixed

- **图层修复真正生效**：前端 `fixAlphaLayers` 接到 `process_zip` / Scheduler（此前勾选无效）。
- Rule 7 误用当前像素 RGB 写入邻居；Rule 7 被 Rule 6 的 else-if 吞掉；Rule 9 与 Rule 5 串联互相覆盖。
- 1.8 图集 168–196 残留像素导致 tabs 误裁「假第 7 页」。

## [2.1.2] - 2026-09-12（BUILD 20038）

### Added

- **覆盖包双色循环描边（N 卡）**：新增 core_gradient_outline 着色器模板，两色随时间循环；视觉设置可调颜色 A / B，并带实时预览条。
- **HSV 取色器组件**：从设置主题色抽出 HsvColorPicker，双色描边与主题色共用。
- **1.8 → 1.21.10 新木种补全**：红/诡异、红树木、樱花、竹、苍白橡木的木板 / 原木 / 竹块映射与旧包升版时的贴图生成；26.3 未发布木种暂不加入。
- **覆盖包打包询问输出路径**：开始打包时弹出系统保存对话框，可自定义 zip 位置。

### Changed

- **覆盖包页布局对齐转换页**：双栏卡片网格（项目列表 + 快速开始 / 项目配置 + 自定义内容）；列表 ↔ 编辑淡入淡出切换。
- **自定义内容改为右侧滑入侧栏**（名称 / 大小 / 视觉），与版本选择一致；去掉右上角关闭，底栏「保存 + 返回」，关闭时向右滑出。
- **实体描边选项修正**：彩虹描边拆为 N 卡 / 其他显卡两档（core_rainbow_outline / core_rainbow_outline_hexian）；仅「自定义颜色描边」可改色。
- **樱花木板 / 原木 / 告示牌调色**：色相从 -80°（偏品红）改为 -45° 并降饱和提亮，更接近原版浅粉。

### Fixed

- 覆盖包页缺失 i18n（快速开始等原始键直接显示）。
- 列表项悬停上浮导致描边被父容器裁切。
- 侧栏退出无滑出动画（scoped 样式未 :deep 到子组件面板）。
- 转换页曾被误改布局，已还原。

## [2.1.1] - 2026-08-23（BUILD 20037）

正式版。包含 **2.1.0 / Beta-2.1.0** 的全部内容（界面动画、Java ↔ 基岩双向转换基础链路等），另加：

### Added

- **j2b HUD / 容器 UI 补全**：快捷栏 / 心 / 饥饿 / 护甲 / 经验条按标准 UV 处理；容器界面写入 `textures/ui/` 并按实际尺寸垫 POT；无源 icons 时从 1.20+ sprites 拼 `textures/gui/icons.png`。
- **j2j 着色器适配（1.20 → 26.x）**：关键版本边不再整目录删除；补 core JSON、`mat2/mat3→mat4`、`#moj_import` 命名空间、按需注入 `globals.glsl`、`fog_distance` 提示。
- **j2b 着色器结构落盘**：复制到 `shaders/glsl|hlsl` 并生成 `materials/*.material` stub（Render Dragon 下可能无效）。

### Changed

- Bedrock 中间态统一到 **Java 26.2（format 88）**；边 `(88,1000)` / `(1000,88)`。
- 转换页与三页返回按钮视觉对齐全局玻璃 / 主题色。

### Fixed

- 床手持、桶 / 弓弩、木板·原木、盔甲层路径；不再误删 `models/armor`。
- `textures/container` 与 Java 1.20+ `gui/sprites/*`、`ui/heart` 等正确并入 `ui/`。
- 停止写入自生成 `item_texture` / `terrain_texture`（会破坏物品栏图标）。
- 药水铺开到全部 `potion_bottle_*` 与 splash；音效根 `sounds.json` 回退。
- 覆盖包 **small_item** 路径改为 `models/item/`（此前缩小物品不生效）。

## [2.1.0] - 2026-08-23（BUILD 20035）

### Added

- **界面动画系统重做 + 开关**：统一动效 token；页面交叉淡入淡出 + 轻微模糊衔接；弹层只缩放面板、遮罩仅淡入淡出；设置新增「界面动画」（开启 / **跟随系统** / 关闭），关闭或系统 `prefers-reduced-motion` 时压掉装饰动效（加载转圈保留）；动画速率三档同步作用于页面切换时长。
- **基岩版双向转换（j2b / b2j）模块化**：`converters/bedrock/{mod,mapping,textures,metadata,fsutil,j2b,b2j}`，经 Scheduler `Exclusive+Surgeon` 版本边 `(84,1000)` / `(1000,84)` 挂载；`invoke_conversion` 仅注册任务。输入接受 `.mcpack`/`.zip`；检测到基岩源时先 b2j 再转 Java 目标格式。
- **j2b 贴图索引**：生成 `textures/textures_list.json`、`terrain_texture.json`、`item_texture.json`；`colormap→colormaps`；音效定义写入 `sounds/sound_definitions.json` **与包根** `sounds.json`；flipbook 动画对齐 atlas shortname。
- **Bedrock 中间态跳过 GuiSurgeon**：避免 Java 75 中间包被 sprite 手术干扰 j2b 输出。

### Changed

- **j2b/b2j 别名对齐 vanilla Bedrock**：药水 `potion_bottle_*`；床 `{color}_bed→bed_{color}`（并从 `blocks/` 复制到 `items/`）；草 `grass_block_*→grass_*`；桶 `water_bucket→bucket_water` 等；`bow→bow_standby`、`crossbow→crossbow_standby`；item atlas 使用 vanilla 数组 shortname（`bed` / `bucket` / `bow_pulling` 等）。
- **撤销错误的 netherite 改名**：Bedrock 与 Java 对 `netherite_*` 同名，不再映射为 `*_netherite`。
- **UI 贴图路径**：`gui/**` 合并进 `textures/ui/` 后，将 `ui/container/*` 扁平到 `ui/`（widgets / icons 可被基岩加载）。
- 平台独有内容按方向剥离（Java model/blockstates/shaders 等 ↔ Bedrock attachables/fog/geo 等）；b2j 删除 flipbook / textures_list / atlas 索引。

### Fixed

- 页面切换动画速率未作用于 enter/leave/blur（仅 stagger）。
- 快速切页时旧页叠影：退场 blur 与淡出时序拆分。
- 药水 / 桶 / 弓弩 / 床 / 草方块等可见贴图在基岩目标下丢失或错误路径。

## [2.0.5] - 2026-08-22（BUILD 20034）

### Added

- **更新源切换 + 测速**：设置页新增「更新源」（镜像源 `cdn.5eggpack.top` / GitHub 官方），附「测速」按钮（测量两源的延迟与下载速率）与「使用最快源」一键切换。
- **SHA-256 完整性校验**：更新器下载安装包时，若 release 附有同名 `.sha256` 资产则边下载边校验哈希，不匹配即拒绝并删除文件；打包流水线（`build_release.py`）自动在安装包旁生成同名 `.sha256` 文件（发版需一并上传）。
- **下载域名白名单 + 资产名校验**：安装包与哈希文件只允许从 `github.com` / `objects.githubusercontent.com` / `cdn.5eggpack.top` 下载；安装包名必须符合 `2-Pyramid-Installer-` 前缀且包含所选版本号。

### Changed

- **安装器目录自动对齐**：检测到本渠道已安装的 2-Pyramid 时，安装目录自动对齐到现有安装位置（重装/升级沿用上次选择的路径），界面给出提示。
- **安装器移除「快捷栏（任务栏）」选项**：任务栏固定受 Windows 版本限制且不稳定，安装选项只保留桌面快捷方式与开始菜单。
- 清理 CSP 中无用的镜像域名（Rust 侧请求不受 CSP 约束）。

## [2.0.4] - 2026-08-22（BUILD 20033）

### Added

- **更新通道「全部」选项**：设置页更新通道为三态——稳定版 / 测试版 / **全部**（同时接受两个通道的更新内容，取最高版本）。此前代码虽已包含该选项，但本版本重新构建确保安装包内完整生效。

## [2.0.3] - 2026-08-22（BUILD 20032）

正式版。包含 Beta-2.0.2 的全部内容，另加：

### Added

- **更新日志 Markdown 渲染**：更新对话框的 Release 说明改为 Markdown 视图（标题 / 列表 / 引用 / 代码块 / 加粗 / 链接），链接经系统浏览器打开；渲染器对不可信内容做 HTML 转义与链接白名单防护。

### 自 Beta-2.0.2 起包含

- **基岩版转换（Bedrock Latest）**：版本选择器新增特殊目标「Bedrock Latest」——选中后先把包转换到 Java 1.21.11（pack_format 75），再按原 Python 版设计做基岩结构重组（pack.png→pack_icon.png、textures/font 提升、textures 提升、item→items 与 golden/wooden 改名、gui/container→ui、creative_inventory 提取、清理空 assets、生成 manifest.json），输出 `.mcpack`。**⚠ 功能未完成、存在严重问题，仅用于测试**（选择该目标时会弹出警示确认；java_ui 模板因兼容性不佳已移除）。
- **动态贴图转换**：`textures/item` / `textures/items` 下与 png 同名的 `.png.mcmeta` 若还是老版 `{"animation": {}}` 格式，会读取同名贴图尺寸推导帧数（横条 = 宽/高，竖条 = 高/宽），改写为高版本适配格式 `{"animation": {"frametime": <帧数>, "interpolate": true}}`；已声明 frametime 的保持原样，尺寸无法整除的跳过不瞎猜。
- **更新通道三态**：稳定版 / 测试版 / 全部——「全部」同时接受两个通道的更新内容（取最高版本），「测试版」现在只收测试更新（原行为是包含全部）。
- **动作记录导出 (.2amr)**：开发者选项新增「导出动作记录」，把动作监视记录的点击序列（含所在 Vue 页面）导出为 2amr 文件，供内部工具 **Action Mon3tr**（不开源，不入仓库）逐帧回放调试。
- **实时动作流（仅 dev）**：debug 构建时 2-Pyramid 在 127.0.0.1:24159 提供本地动作流（支持按需抓取主窗口截图），供 Action Mon3tr 接管实时动作。
- **输入防护**：材质包选择不再接受 `.mcpack` 文件（Bedrock 输入功能未完善）。

### Fixed（自 Beta-2.0.2 起）

- **更新安装器拉起修复**：更新下载后不再静默安装（会因旧进程文件锁无声失败），改为拉起图形安装向导。
- **测试通知修复**：通知总开关关闭时，「测试通知」按钮仍可正常预览效果（不再静默无反馈）。
- 更新检查器读取的 mcmeta 兼容 UTF-8 BOM。
- 对话框关闭按钮不再被统一按钮皮肤覆盖（去掉奇怪的玻璃底色）。

## [Beta-2.0.2] - 2026-08-22（BUILD 20031）

### Added

- **基岩版转换（Bedrock Latest）**：版本选择器新增特殊目标「Bedrock Latest」——选中后先把包转换到 Java 1.21.11（pack_format 75），再按原 Python 版设计做基岩结构重组（pack.png→pack_icon.png、textures/font 提升、textures 提升、item→items 与 golden/wooden 改名、gui/container→ui、creative_inventory 提取、清理空 assets、生成 manifest.json），输出 `.mcpack`。**⚠ 功能未完成、存在严重问题，仅用于测试**（选择该目标时会弹出警示确认；java_ui 模板因兼容性不佳已移除）。
- **动态贴图转换**：`textures/item` / `textures/items` 下与 png 同名的 `.png.mcmeta` 若还是老版 `{"animation": {}}` 格式，会读取同名贴图尺寸推导帧数（横条 = 宽/高，竖条 = 高/宽），改写为高版本适配格式 `{"animation": {"frametime": <帧数>, "interpolate": true}}`；已声明 frametime 的保持原样，尺寸无法整除的跳过不瞎猜。
- **动作记录导出 (.2amr)**：开发者选项新增「导出动作记录」，把动作监视记录的点击序列（含所在 Vue 页面）导出为 2amr 文件，供内部工具 **Action Mon3tr**（不开源，不入仓库）逐帧回放调试。
- **实时动作流（仅 dev）**：debug 构建时 2-Pyramid 在 127.0.0.1:24159 提供本地动作流（支持按需抓取主窗口截图），供 Action Mon3tr 接管实时动作。
- **输入防护**：材质包选择不再接受 `.mcpack` 文件（Bedrock 输入功能未完善）。

### Changed

- 开发者选项分组与其他分组行为统一（淡入动画、搜索联动、i18n 标签）。
- 移除 java_ui UI 模板（基岩 UI 兼容性不佳）。
- **更新通道三态**：稳定版 / 测试版 / 全部——「全部」同时接受两个通道的更新内容（取最高版本），「测试版」现在只收测试更新（原行为是包含全部）。

### Fixed

- **更新安装器拉起修复**：更新下载后不再静默安装（会因旧进程文件锁无声失败），改为拉起图形安装向导。
- 更新检查器读取的 mcmeta 兼容 UTF-8 BOM。
- 对话框关闭按钮不再被统一按钮皮肤覆盖（去掉奇怪的玻璃底色）。
- **测试通知修复**：通知总开关关闭时，「测试通知」按钮仍可正常预览效果（不再静默无反馈）。

## [Beta-2.0.1] - 2026-08-18（BUILD 20029）

> 这是 2-Pyramid 的 Beta 测试版（tag: Beta-2.0.1）。可通过应用内「更新通道 → 测试版」检查到，或直接在 Releases 下载 `2-Pyramid-Installer-2.0.1-beta.{BUILD}.exe`。
> Beta 与正式版可并存安装：独立注册表、独立目录（%LOCALAPPDATA%\2-Pyramid-Beta）、程序内带 Beta 标识。正式版请用 Stable-/v 前缀的发布。

### 新增

- **安装器三快捷方式选项**：安装时可勾选「桌面快捷方式 / 加入开始菜单 / 加入快捷栏（任务栏）」；卸载时自动清理快捷方式与任务栏固定。
- **Beta 双渠道构建**：`npm run betabuild` 一键产出 Beta 安装包；主程序从启动画面、首页到设置页均有 Beta 标识，窗口标题与任务栏同步显示。
- **更新检查识别测试版**：Release tag 带 `Beta-` / `UnStable-` 前缀即视为测试版更新，仅测试版通道可见，更新对话框带「测试版」徽章。
- **目录规整防呆**：`pack.mcmeta.txt` 等任意后缀的文件，只要内容是合法 mcmeta（能解析出 format 数值），自动统一改名为 `pack.mcmeta` 并提升到压缩包根目录，再正常转换。
- **版本详情页改版**：只保留主版本 + 构建号两项，干净直观。

### 修复

- **卸载器彻底修复**：双击 uninstall.exe 或从控制面板卸载，直接进入卸载流程（不再打开安装界面）；卸载过程有明确的进度反馈，完成动画播完后自动关窗，退出时自行清理残留文件。
- **背景透色调整**：调整透色强度/展示方式不再需要重新选择背景图片。
- **输出文件名版本前缀替换**：`[Java 1.x-y]` 标签出现在名称任意位置都能被替换，再转换时不再出现新旧前缀堆叠。
- **BUILD 构建号**：每次发布构建只递增一次（此前会 +2）。
- **卸载器自清理**：不再反复 ping 127.0.0.1，改为进程句柄等待，安静无痕。

### 其他

- README 全面更新，新增根目录 CHANGELOG.md。
- 更新源切换到 2-Pyramid 官方仓库 Releases。

## [2.0.0] - 2026-08-17

### Added

- **Tauri 2 桌面版**：覆盖 1.6 → 26.1+ 共 26 个目标版本区间的任意互转，批量并行（1–4 线程）转换。
- **自研安装器**（`installer-app`）：OOBE 分步向导、实时进度、GitHub 介绍卡片；可选创建桌面 / 开始菜单 / 任务栏快捷方式；写入控制面板卸载入口（HKCU，无需管理员权限）。
- **卸载器**：以 `uninstall.exe --uninstall` 或双击卸载器直接进入卸载流程；删除程序文件与注册表、清理快捷方式，用户数据（`~/.2pyr`）保留；退出时自行清理残留。
- **Beta 构建渠道**：`npm run betabuild` 输出 `2-Pyramid-Installer-{版本}-beta.{BUILD}.exe`；Beta 与正式版注册表 / 目录 / 快捷方式全部隔离，可并存；主程序与安装器界面带 Beta 标识。
- **更新系统**：基于 GitHub Releases 的检查 / 下载 / 静默安装，支持 `Safe-` / `Stable-` / `UnStable-` 前缀优先级与稳定 / 测试双通道。
- **界面定制**：自定义背景（自动提取主题色）、玻璃 / 磨砂控件皮肤、动画速率三档、中英双语。
- **输出命名模板**：`[Ver]` / `[Name]` / `[Time]` / `[Date]` 占位符 + 实时预览。
- **其他**：转换历史记录、桌面 Toast 通知、单实例端口锁、动作监视（`--action-monitor` / `2PYR_ACTION_MONITOR=1`）、OKAY 风格日志、无边框窗口（关闭即退出）。

### Changed

- 移除托盘常驻与「关闭最小化到托盘」行为，主窗口关闭即退出。
- 移除右键菜单与 COM 服务器。
- 发布产物改为便携版内嵌安装器的单文件安装包（不再使用 Inno / NSIS）。

### Fixed

- 各种窗口生命周期 / Toast 堆叠 / 对话框过渡 / 背景图层问题（详见提交历史）。

## [1.0.0] - 2026-07-10

- 初始提交：基于原 Python 版 `pack.py`（Hurricane）移植的 Tauri + Rust + Vue 桌面版。
