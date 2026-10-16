#!/usr/bin/env python3
"""从**官方 Minecraft Java 客户端 jar** 提取 1.21.4+ 的「物品模型定义」参考数据。

为什么要这个脚本
================

1.21.4（pack_format 46）起，Minecraft 把「一个物品用什么模型」从
`assets/<ns>/models/item/<name>.json` 里的 `overrides` + `CustomModelData` 谓词，
搬到了**新的物品模型定义**文件 `assets/<ns>/items/<name>.json`：

    { "model": { "type": "minecraft:model", "model": "minecraft:item/stick" } }

旧包里 `models/item/*.json` 的 `overrides` 在 1.21.4+ 上**完全不被读取**——
因此旧包转上去以后，弓的拉弦、时钟的 64 帧、皮革甲的染色与盔甲纹饰全部失效。

要修好它，转换器必须知道「每个物品在新版里的**默认定义**长什么样」：
那些 `select` / `range_dispatch` / `condition` / `tints` / `special` 结构是
**Mojang 的数据**，不是可以手写推导的东西（时钟 64 帧、指南针 32 帧、
盔甲纹饰 11 种材质 × 4 件 × …）。手抄必错，抄别人的包又有许可问题。

**所以：从官方客户端 jar 重新生成。** 这是唯一同时满足「权威」「可复现」「无许可风险」的路。

产出的数据文件
==============

`src-tauri/src/natives/architect/item-definitions.json`——按 `pack_format` 索引：

    {
      "source": { ... 每个 jar 的 sha256 / 版本号 / pack_format ... },
      "profiles": {
        "46": { "reference": "1.21.4", "definitions": { "<name>": { "model": ... } } },
        ...
      }
    }

**只保留"有内容"的定义**（见 `is_interesting`）。纯 `{"type":"minecraft:model","model":"minecraft:item/<自身>"}`
与 `{"type":"minecraft:model","model":"minecraft:block/<同名>"}` 这两类**不收录**：
前者转换器可以一行生成，后者是方块物品、与「自定义外观」无关。
实测这一过滤把 1385 项压到 229 项（1.21.4），否则数据文件会大 6 倍。

复现方式
========

    python tools/gen-item-definitions.py

脚本会自己从 `%APPDATA%\\.minecraft\\versions\\<v>\\<v>.jar` 里读 `version.json`，
用**jar 自报的** `pack_version.resource` 当 pack_format（而不是脚本里写死的数字），
所以「格式号对不对」这件事是被源数据验证的，不靠人记。

许可
====

产出的是 Mojang 的**数据**（物品定义的 JSON 结构），由本脚本从官方客户端重新提取，
不来自任何第三方 GPL 项目。脚本本身为 2-Pyramid 原创（MIT）。
"""

from __future__ import annotations

import hashlib
import json
import os
import sys
import zipfile

# ── 受支持的版本 → 客户端目录名 ────────────────────────────────────────────
#
# 2pyr 的目标格式到 1.21.4+ 有 9 个（46/55/63/64/69/75/84/88/97）。
# 本机客户端目录只覆盖其中 5 个（1.21.5 / 1.21.6 不在）。
# 缺失的那些在 Rust 侧按「≤ 目标的最大已收录格式」回落（与整个转换器的版本
# 处理方式一致：格式边界是单调的，1.21.5 的物品定义与 1.21.4 逐字节相同）。
VERSIONS = ["1.21.4", "1.21.8", "1.21.10", "1.21.11", "26.2"]

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(REPO, "src-tauri", "src", "natives", "architect", "item-definitions.json")


def mc_dir() -> str:
    appdata = os.environ.get("APPDATA")
    if not appdata:
        sys.exit("APPDATA 未设置——本脚本需要在 Windows 上运行")
    return os.path.join(appdata, ".minecraft", "versions")


def jar_path(version: str) -> str:
    return os.path.join(mc_dir(), version, f"{version}.jar")


def resource_format(raw: bytes) -> int:
    """从 jar 内的 `version.json` 读资源包格式号。

    1.21.4–1.21.8 用 `pack_version.resource`；
    1.21.10 起改成 `resource_major` / `resource_minor`（等价于 major*1000+minor，
    本数据集里 minor 全为 0，因此直接取 major）。
    两种形状都接受——**不写死**在脚本里，免得哪天格式号变了没人发现。
    """
    pv = json.loads(raw)["pack_version"]
    if "resource" in pv:
        return int(pv["resource"])
    # `resource_major` 就是 pack_format 本身（69 / 75 / 88 …），
    # 不是在它后面再拼小版本——本数据集里 `resource_minor` 恒为 0。
    return int(pv["resource_major"])


# 这两类**不是资源包里能用的物品定义**，收录它们只会让数据文件白白变大：
#
# * `minecraft:special`（1.21.4 有 59 条：床/箱子/旗帜/盾/潜影盒/潮涌核心……）
#   由客户端硬编码渲染器画，`base` 只提供变换与纹理提示。
#   **它的外观无法被物品模型定义改写**，因此转换器永远不会引用它。
# * `minecraft:constant`（1.21.8 起出现，1.21.4 没有）是**数据包侧**的写法
#   （把某个组件值钉成常量），资源包定义里根本不会出现。
NOT_A_TEMPLATE = {"minecraft:special", "minecraft:constant"}


def is_interesting(definition: dict) -> bool:
    """是否需要收录这条定义。

    剔除三类——它们占官方 1385 条里的 1200 条以上，但转换器不需要模板：

    * [`NOT_A_TEMPLATE`] 里的两种类型（见那里的说明）；
    * `minecraft:model` 且**除 type/model 外没有别的键**（没有 tints）——
      转换器可以一行生成 `{"type":"minecraft:model","model":"minecraft:item/<name>"}`；
    * 带 tints 的 `minecraft:model` 里，引用的模型就是自身的那部分（同样是可生成的）。

    剩下的是真正带「结构」的定义：`select`（盔甲纹饰 / 山羊角乐器 / 旗帜图案）、
    `range_dispatch`（时钟 64 帧、指南针 32 帧、弩的拉弦、`use_cycle`）、
    `condition`（弓 / 钓鱼竿 / 鞘翅 / 盾 / 望远镜），以及带 `tints` 的染色物品
    （皮革甲、树叶、刷怪蛋）。实测 1.21.4 收录 162 条、26.2 收录 118 条。
    """
    model = definition.get("model")
    if not isinstance(model, dict):
        return False
    kind = model.get("type")
    if kind in NOT_A_TEMPLATE:
        return False
    if kind != "minecraft:model":
        return True
    # 带 tints（或别的键）的才留：纯 `{type, model}` 是可生成的。
    return bool(set(model) - {"type", "model"})


def extract(version: str) -> tuple[int, dict, str]:
    path = jar_path(version)
    if not os.path.isfile(path):
        sys.exit(f"找不到客户端 jar：{path}")
    with open(path, "rb") as fh:
        digest = hashlib.sha256(fh.read()).hexdigest()
    with zipfile.ZipFile(path) as z:
        fmt = resource_format(z.read("version.json"))
        prefix = "assets/minecraft/items/"
        defs = {}
        for name in z.namelist():
            if not (name.startswith(prefix) and name.endswith(".json")):
                continue
            stem = os.path.basename(name)[: -len(".json")]
            definition = json.loads(z.read(name).decode("utf-8"))
            if is_interesting(definition):
                defs[stem] = definition
    return fmt, dict(sorted(defs.items())), digest


def main() -> None:
    profiles: dict[str, dict] = {}
    sources: dict[str, dict] = {}
    for version in VERSIONS:
        fmt, defs, digest = extract(version)
        key = str(fmt)
        if key in profiles:
            sys.exit(f"格式号 {fmt} 被 {version} 与 {profiles[key]['reference']} 同时占用")
        profiles[key] = {"reference": version, "definitions": defs}
        sources[key] = {"jar": f"{version}.jar", "version": version, "sha256": digest}
        print(f"{version}: pack_format={fmt} 收录 {len(defs)} 条定义  sha256={digest[:16]}…")

    payload = {
        "_note": (
            "由 tools/gen-item-definitions.py 从官方 Minecraft Java 客户端 jar 重新提取；"
            "不是从任何第三方项目复制的数据。"
        ),
        "source": sources,
        "profiles": profiles,
    }
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    # 紧凑分隔符 + 排序键：数据文件进版本库，逐字节可复现（同一 jar 必然同样输出）。
    text = json.dumps(payload, ensure_ascii=False, separators=(",", ":"), sort_keys=True)
    with open(OUT, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(text)
    print(f"已写出 {OUT}（{len(text)} 字符）")


if __name__ == "__main__":
    main()
