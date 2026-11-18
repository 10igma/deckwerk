#!/usr/bin/env bash
# Übernimmt ausgewählte Screenshots aus ui/.shots/ in die Dokumentation (docs/bilder/).
# Vorher: pnpm build && pnpm shots
# Das Bild der echten Anwendung (anwendung-linux.png) entsteht nicht hier, sondern aus einem
# Lauf der Desktop-Hülle; siehe docs/fortschritt.md, Abschnitt „Befehle".
set -euo pipefail
cd "$(dirname "$0")/.."
ziel=../docs/bilder
mkdir -p "$ziel"
cp .shots/mixen-uebergang-1920x1080.png   "$ziel/mixen-uebergang.png"
cp .shots/mixen-geladen-1920x1080.png     "$ziel/mixen-geladen.png"
cp .shots/zustand-bibliothek-leer-1280x720.png "$ziel/mixen-leer.png"
cp .shots/zustand-analyse-1280x720.png    "$ziel/bibliothek-analyse.png"
cp .shots/zustand-schutz-stopp-1280x720.png "$ziel/schutz-stopp.png"
echo "Bilder nach $ziel übernommen."
