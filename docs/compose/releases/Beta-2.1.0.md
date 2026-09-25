✨ 2.1.0 测试版更新（BUILD 20035）

这是 2-Pyramid 的测试版（tag: Beta-2.1.0）。可通过应用内「更新通道 → 测试版 / 全部」检查到，
或直接在 Releases 下载 2-Pyramid-Installer-2.1.0-beta.20035.exe。

⚠ 基岩版转换仍为实验性功能：资源包转换后的模型、部分 UI 与键级映射无法与原版 100% 等价，
请以测试反馈为主，勿用于对完整性要求极高的生产包。

✨ 新增

界面动画系统重做 + 开关：统一动效节奏；页面切换改为交叉淡入淡出并带轻微模糊衔接，
弹层只对内容面板做软缩放、遮罩仅淡入淡出。设置页新增「界面动画」——开启 / 跟随系统 /
关闭；关闭或系统开启了「减少动态效果」时会压掉装饰性动画（加载转圈等状态反馈保留）。
动画速率三档（慢 / 优雅 / 快）现在会作用于完整的页面切换时长。

基岩版双向转换（j2b / b2b）：转换逻辑拆分为 converters/bedrock 子模块，经调度器版本边挂载。
输入接受 .mcpack 与 .zip；检测到基岩资源包时会先转成 Java 结构，再转到你选择的 Java 目标版本。
选择「Bedrock Latest」时会先把包转到 Java 1.21.11，再生成 .mcpack。

j2b 贴图索引：自动生成 textures/textures_list.json、terrain_texture.json、item_texture.json；
colormap 目录改名为 colormaps；音效定义同时写入 sounds/sound_definitions.json 与包根 sounds.json；
动态贴图 flipbook 与 atlas 短名对齐。

🔧 修复 / 改进

可见贴图路径对齐 vanilla Bedrock：药水改为 potion_bottle_*；彩色床 {color}_bed → bed_*
并复制到 items/ 供物品栏图标使用；草方块 grass_block_* → grass_*；桶 water_bucket → bucket_water
等；弓 bow → bow_standby、弩 crossbow → crossbow_standby。物品图集对床 / 桶 / 弓弩等使用
vanilla 数组短名（bed、bucket、bow_pulling 等），避免游戏仍按旧短名查找导致黑紫块。

撤销错误的下界合金改名：Bedrock 与 Java 对 netherite_* 命名一致，不再映射为 *_netherite。

UI 贴图路径：gui/** 合并进 textures/ui/ 后，将 ui/container/* 扁平到 ui/（widgets、icons
等可被基岩正常加载）。Bedrock 转换中间态跳过 GuiSurgeon，避免干扰输出。

页面切换动画速率此前只影响列表 stagger，现已作用于进出场时长；快速切页时旧页叠影
（退场模糊与淡出时序拆分）已改善。

发版说明：本 Release 需同时上传 2-Pyramid-Installer-2.1.0-beta.20035.exe 与
2-Pyramid-Installer-2.1.0-beta.20035.exe.sha256 两个资产（更新器 SHA-256 校验依赖后者）。

如何反馈问题：在仓库 Issues 提交即可，最好附上「设置 → 开发者模式 → 导出日志」的日志文件，
以及原资源包版本与目标版本（Java / Bedrock）。
