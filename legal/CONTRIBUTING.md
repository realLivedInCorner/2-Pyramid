# 贡献指南 / Contributing

感谢你对 2-Pyramid 的兴趣！欢迎 Issue 与 Pull Request。

---

## 1. 开始之前

- 先搜 [Issues](https://github.com/realLivedInCorner/2-Pyramid/issues) 与 Discussions，避免重复
- 较大的功能改动请先开 Issue 对齐方案，再动手
- 行为准则：保持尊重；聚焦技术；不接受人身攻击或无关政治争论

## 2. 开发环境

```bash
git clone <repo>
cd 2-Pyramid          # 或你的本地路径（仓库内为 code/）
npm install
npm run 2pyr          # Tauri dev（Rust + Vite）
```

常用命令：

| 命令 | 说明 |
|------|------|
| `npm test` | Rust 单元测试（`cargo test --lib`） |
| `npm run build` | 仅前端构建 |
| `npm run buildrelease` | 正式版完整构建（会递增 BUILD） |

依赖：Node.js、Rust stable、Windows（当前仅官方支持 Windows 桌面目标）。

## 3. 代码规范

### Rust

- 运行 `cargo fmt`（若仓库未强制 rustfmt 配置，保持与邻近代码一致）
- `cargo clippy` 能过则过；新代码避免新增 `unwrap()` 于非测试路径
- 转换逻辑请放在 `src-tauri/src/converters/` 对应子模块（`ui/` / `textures/` / `reverse/` 等），并注册到 Scheduler，**不要**把业务写进 `invoke_conversion.rs`
- 新增或修改图像/路径逻辑时，尽量补 `#[cfg(test)]` 单测

### 前端 / Vue

- TypeScript + `<script setup>`
- 文案走 `src/locales/zh-CN.json` 与 `en-US.json`，**两边都要写**
- 视觉与交互对齐现有全局玻璃卡片与主题色 token

### 提交信息

- 使用祈使句简述意图，例如：`fix(overlay): correct share code prefix` / `feat(ui): …`
- 一个 PR 聚焦一件事；无关格式化请单独提交

## 4. Pull Request

1. Fork 后从 `master` 拉出分支（如 `feat/xxx`）
2. 确保 `npm test` 通过；涉及 UI 时说明如何验证
3. PR 描述写清：动机、改动点、测试情况、相关 Issue
4. 保持 diff 可读；大文件或生成物请说明来源

维护者可能要求调整命名、拆分提交或补充测试，请尽量配合。

## 5. 贡献内容的许可

除非另有书面约定，你提交的代码默认按 **MIT 许可证** 向公众授权，允许 2-Pyramid Studio **及所有后续接收者**使用、修改与再分发。提交即表示你有权做此授权（例如不违反雇佣协议或第三方版权）。

**请勿提交：**

- 未授权的 Minecraft 官方素材、他人资源包或破解内容
- 体积过大的二进制（截图请压缩；测试贴图尽量自造）
- 含个人信息的路径、日志或账号数据

## 6. 报告 Bug

请尽量附上：

- 2-Pyramid 版本与 BUILD（设置 → 版本信息）
- 源包版本 / 目标版本（Java pack_format 或 Bedrock）
- 复现步骤与期望结果
- 「设置 → 开发者模式 → 导出日志」

转换类问题若能提供**可公开的小样例包**，修复会快很多。

## 7. 功能建议

请说明：使用场景、现有流程为何不够、你期望的交互。带截图或草图更佳。

---

**English summary:** Discuss large changes in an issue first. Put converters under the domain modules, keep zh/en locales in sync, run `npm test`, and use clear commit messages. Contributions are MIT-licensed to the public (the Studio is only the maintainer); do not commit unauthorized Minecraft assets or personal data.
