#!/bin/sh
# Copies built Salt jars for a version folder into every salt-mods location
# the launcher and smoke tests read.  usage: tools/stage.sh <folder> [target...]
set -e
F=$1; shift
T=${*:-$F}
B=~/SaltLauncherBuild/mods-build/$F
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
for t in $T; do
  for d in "$ROOT/dist/salt-mods/$t" ~/AppData/Roaming/SaltLauncher/salt-mods/$t ~/SaltLauncherBuild/smoke-data/salt-mods/$t ~/AppData/Local/Packages/Claude_pzs8sxrjxfjjc/LocalCache/Roaming/SaltLauncher/salt-mods/$t; do
    mkdir -p "$d"
    for old in "$d"/salt*.jar; do [ -e "$old" ] && mv "$old" "$old.old" && rm -f "$old.old"; done
    for j in "$B"/*/libs/salt*.jar; do case "$j" in *-sources.jar) ;; *) cp "$j" "$d/";; esac; done
  done
  echo "staged $F -> $t: $(ls "$ROOT/dist/salt-mods/$t" | tr '\n' ' ')"
done
