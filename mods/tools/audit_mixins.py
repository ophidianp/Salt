"""Checks every Salt mixin's target method (and redirected call) exists in a
Minecraft jar, using javap.

    python -I mods/tools/audit_mixins.py <mods folder> <minecraft jar> [javap]

Name-level only for @Inject; for @Redirect/INVOKE it also checks the called
method's owner/name/descriptor appears inside the target method's bytecode.
Works on mojmap jars (26.x, and 1.21.x dev jars from Loom's cache).
"""
import pathlib
import re
import subprocess
import sys

folder, jar = pathlib.Path(sys.argv[1]), sys.argv[2]
javap = sys.argv[3] if len(sys.argv) > 3 else "javap"
cache = {}


def disasm(cls):
    if cls not in cache:
        r = subprocess.run([javap, "-p", "-c", "-cp", jar, cls], capture_output=True, text=True)
        cache[cls] = r.stdout if r.returncode == 0 else None
    return cache[cls]


def methods(text):
    """name -> bytecode text for each method in javap -c output."""
    out = {}
    cur = None
    for line in text.splitlines():
        m = re.match(r"\s{2}\S.*?\b([\w$<>]+)\(.*\);?$", line)
        if m and not line.startswith("    "):
            cur = m.group(1)
            out.setdefault(cur, "")
        elif cur:
            out[cur] += line + "\n"
    return out


bad = 0
for f in sorted(folder.glob("*/src/main/java/**/mixin/**/*.java")):
    src = f.read_text(encoding="utf-8")
    imports = dict((m.group(2), m.group(1)) for m in re.finditer(r"import ([\w.]+\.(\w+));", src))
    pkg = re.search(r"package ([\w.]+);", src).group(1)
    consts = dict(re.findall(r'String (\w+) = "([^"]+)"', src))
    mm = re.search(r"@Mixin\((?:value\s*=\s*)?([\w.]+)\.class", src)
    if not mm:
        continue
    name = mm.group(1)
    head = name.split(".")[0]
    cls = imports.get(head, head)
    if "." in name:  # inner class like MenuTabBar.MenuTabButton
        cls = imports.get(head, head) + "$" + name.split(".", 1)[1].replace(".", "$")
    text = disasm(cls)
    rel = f.relative_to(folder)
    if text is None:
        print(f"MISSING CLASS {cls}  ({rel})")
        bad += 1
        continue
    ms = methods(text)
    for a in re.finditer(r"@(Inject|Redirect|ModifyVariable|ModifyArg|ModifyArgs|WrapOperation|ModifyReturnValue|ModifyExpressionValue)\((.*?)\)\s*\n\s*(?:private|public|protected)", src, re.S):
        body = a.group(2)
        for meth in re.findall(r'method\s*=\s*\{?\s*"([^"]+)"', body):
            mname = meth.split("(")[0]
            if mname not in ms:
                print(f"MISSING METHOD {cls}#{mname}  ({rel})")
                bad += 1
                continue
            t = re.search(r'target\s*=\s*(?:"([^"]+)"|(\w+))', body)
            if t and "INVOKE" in body:
                tgt = t.group(1) or consts.get(t.group(2), "")
                tm = re.match(r"L([\w/$]+);([\w$<>]+)(\(.*)", tgt)
                if tm:
                    owner, n, desc = tm.groups()
                    needle = f"{owner}.{n}:{desc}"
                    code = ms[mname].replace('"', "")
                    own = owner.replace("/", ".") == cls.replace("$", "$")
                    if needle not in code and not (own and f"Method {n}:{desc}" in code):
                        print(f"MISSING CALL {needle} in {cls}#{mname}  ({rel})")
                        bad += 1
print(f"{bad} problem(s)")
