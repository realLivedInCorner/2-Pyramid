# 分析官方 items/*.json：哪些是"真正带调度的定义"，哪些只是自指 model。
import json
import os
import zipfile
from collections import Counter

MC = os.path.join(os.environ["APPDATA"], ".minecraft", "versions")


def load(v):
    with zipfile.ZipFile(os.path.join(MC, v, f"{v}.jar")) as z:
        return {
            n: json.loads(z.read(n).decode("utf-8"))
            for n in z.namelist()
            if n.startswith("assets/minecraft/items/") and n.endswith(".json")
        }


def trivial(defn, name):
    m = defn.get("model")
    if not isinstance(m, dict):
        return False
    if set(m.keys()) != {"type", "model"}:
        return False
    return m.get("type") == "minecraft:model" and m.get("model") == f"minecraft:item/{name}"


for v in ["1.21.4", "1.21.8", "1.21.10", "1.21.11", "26.2"]:
    items = load(v)
    trivial_n = [n for n in items if trivial(items[n], os.path.basename(n)[:-5])]
    nontrivial = {n: d for n, d in items.items() if n not in trivial_n}
    print(f"=== {v}: total={len(items)} trivial={len(trivial_n)} non-trivial={len(nontrivial)}")

# 对 1.21.4 详细展开非平凡定义的"顶层形状"
items = load("1.21.4")
nt = {n: d for n, d in items.items() if not trivial(d, os.path.basename(n)[:-5])}


def top_shape(d):
    m = d.get("model")
    if not isinstance(m, dict):
        return f"model={type(m).__name__}"
    t = m.get("type", "?")
    extra = sorted(k for k in m if k not in {"type", "model"})
    return f"{t} {extra}"


c = Counter(top_shape(d) for d in nt.values())
for k, n in c.most_common():
    print(f"  {n:5d}  {k}")

print()
print("=== 非平凡里，model.type == minecraft:model（只是带 tints/special）的清单 ===")
for n, d in sorted(nt.items()):
    m = d["model"]
    if isinstance(m, dict) and m.get("type") == "minecraft:model":
        print(" ", n, sorted(m.keys()))

print()
print("=== 带 select/range_dispatch/condition/composite 的清单（真正需要模板的）===")
for n, d in sorted(nt.items()):
    m = d["model"]
    if isinstance(m, dict) and m.get("type") in {
        "minecraft:select", "minecraft:range_dispatch", "minecraft:condition", "minecraft:composite",
    }:
        print(" ", n, m.get("type"), m.get("property"))
