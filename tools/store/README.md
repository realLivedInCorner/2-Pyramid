# Microsoft Store 提交材料（tools/store）

这里存放**提交 Microsoft Store 时粘贴的文案**，按版本归档：
`whats-new-<版本>.md` = Partner Center 的「**此版本的新增功能 / What's new in this version**」字段内容。

> 本目录只放**待粘贴的文本**。MSIX 包体、测试签名证书等本地材料放在仓库根 `store/`（已被 `.gitignore` 忽略，不入库）。

## 字段规则（提交前自查）

| 字段 | 上限 | 格式 |
|---|---|---|
| 此版本的新增功能 | **1500 字符**（含换行） | **纯文本**：不支持 Markdown / HTML / 图片 / 链接 |
| 简短说明 | 1000 字符 | 纯文本 |
| 描述 | 10000 字符 | 纯文本（分段用空行） |

- 商店为**每个语言**单独维护这些字段；MSIX 内含中英资源，因此 **zh-CN 与 en-US 都要填**。
- 文案里的数字请用「约」，并以 `tools/benchmark/index.html` 的实测值为准，避免与用户机器表现差距过大。
- **只写"可比口径"的倍数**。若某个新数字没有与上一版同口径的历史记录，就只报绝对值、不给倍数
  （2.200.0 的 release 读数即属此类：旧版没记录过同口径的值）。
- 面向**用户**而非开发者：不写内部结构、模块名、闸门术语；性能只写用户能感知的部分。
- 每份文案末尾附**字数实测表**（含换行与空格，注明测法），提交前核对余量。

## 归档

| 版本 | 文件 | 主题 |
|---|---|---|
| 2.200.0 | [`whats-new-2.200.0.md`](whats-new-2.200.0.md) | 转换内核重写（可比口径约 6.0s → 约 3.6s）、输出不变、批量与长任务提速、许可证正文随包 |
| 2.7.0 | [`whats-new-2.7.0.md`](whats-new-2.7.0.md) | 无损提速（真实包 2.23×）、性能档位、结构分析 |

## 提交流程（与 `tools/release/README.md` 配套）

1. 用 `release\2-Pyramid-<版本>.0.msix`（Partner Center → 新提交 → 程序包）——2.200.0 对应
   `release\2-Pyramid-2.200.0.0.msix`（本版已构建并核验 `AppxManifest Version=2.200.0.0`、
   `Identity=2-PyramidStudio.2-Pyramid`；GitHub Releases 侧只发 EXE，不发 MSIX）；
2. Identity `2-PyramidStudio.2-Pyramid`、Publisher `CN=7BC328FB-A6A3-42E9-A750-326D3BDC3F5F`、版本 `x.y.z.0`；
3. 「此版本的新增功能」按本目录对应文件粘贴（中英各一份）；
4. 提交后记录审核结果；若被要求改措辞，以**同一约束**（纯文本 / 1500 字符）重写后归档为新版本条目。
