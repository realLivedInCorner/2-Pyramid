import io, json
W = r"D:\develop\2pyr\2pyr-desktop\code\.worktrees\motion\src\locales\\"
z = json.load(io.open(W+"zh-CN.json", encoding="utf-8"))
e = json.load(io.open(W+"en-US.json", encoding="utf-8"))
def flat(d,p=""):
    o={}
    for k,v in d.items():
        key=f"{p}.{k}" if p else k
        o.update(flat(v,key) if isinstance(v,dict) else {key:v})
    return o
fz,fe=flat(z),flat(e)
print(f"  键数 zh {len(fz)} / en {len(fe)}  差集 {set(fz)^set(fe) or '无 ✓'}")
print(f"  JSON 均可解析 ✓")
