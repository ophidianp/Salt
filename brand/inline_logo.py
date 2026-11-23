"""Copies the wordmark path from brand/logo.json into the <symbol id="salt"> of each page.
Run after make_assets.py: python -I brand/inline_logo.py"""
import os, re, json

HERE = os.path.dirname(os.path.abspath(__file__)); SITE = os.path.join(HERE, "..")
L = json.load(open(os.path.join(HERE, "logo.json")))
sym = (f'<symbol id="salt" viewBox="{-L["pad"]} {-L["pad"]} {L["w"]} {L["h"]}">'
       f'<path fill="currentColor" d="{L["word"]}"/></symbol>')
for page in ("index.html", "404.html"):
    p = os.path.join(SITE, page)
    s = open(p, encoding="utf-8").read()
    s2, n = re.subn(r'<symbol id="salt".*?</symbol>', lambda m: sym, s, flags=re.S)
    open(p, "w", encoding="utf-8", newline="\n").write(s2)
    print(page, "symbols replaced:", n)
print("aspect-ratio for .lgs:", f'{L["w"]}/{L["h"]}')
