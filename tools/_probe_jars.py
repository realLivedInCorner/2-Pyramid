# 临时探针：检查官方客户端 jar 里 1.21.4+ 物品定义与模型资源的实际形态。
# 不写入任何产物，只打印统计。用完即删。
import json
import os
import zipfile

MC = os.path.join(os.environ["APPDATA"], ".minecraft", "versions")
VERSIONS = ["1.21.4", "1.21.8", "1.21.10", "1.21.11", "26.2"]

for v in VERSIONS:
    jar = os.path.join(MC, v, f"{v}.jar")
    if not os.path.isfile(jar):
        print(f"{v}: JAR MISSING")
        continue
    with zipfile.ZipFile(jar) as z:
        names = z.namelist()
        items = sorted(n for n in names if n.startswith("assets/minecraft/items/") and n.endswith(".json"))
        models = sorted(n for n in names if n.startswith("assets/minecraft/models/item/") and n.endswith(".json"))
        try:
            raw = z.read("version.json").decode("utf-8")
            pv = json.loads(raw)
            pf = pv.get("pack_format")
            pvname = pv.get("id") or pv.get("name")
        except KeyError:
            pf, pvname = None, None
        print(f"{v}: pack_format={pf} version_json_id={pvname} items={len(items)} models_item={len(models)}")
        # 采样一个最简定义，确认 schema 根形状
        if items:
            sample = json.loads(z.read(items[0]).decode("utf-8"))
            print(f"   sample {items[0]}: {json.dumps(sample, ensure_ascii=False)[:200]}")
        # 统计所有 type 值
        from collections import Counter
        c = Counter()

        def walk(o):
            if isinstance(o, dict):
                t = o.get("type")
                if isinstance(t, str):
                    c[t] += 1
                for x in o.values():
                    walk(x)
            elif isinstance(o, list):
                for x in o:
                    walk(x)

        for n in items:
            walk(json.loads(z.read(n).decode("utf-8")))
        print(f"   types: {dict(c.most_common())}")
