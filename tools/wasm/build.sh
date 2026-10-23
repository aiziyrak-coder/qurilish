#!/usr/bin/env bash
# QURAi brauzer to'plamini yig'adi: dist/web/ (index.html, qurai.js, qurai_bg.wasm).
#
# Kerak: rustup target add wasm32-unknown-unknown; wasm-bindgen-cli (Cargo.lock
# dagi wasm-bindgen versiyasi bilan bir xil); C kompilyatori — zig
# (`pip install ziglang`), sozlamasi env.sh da.
set -euo pipefail
cd "$(dirname "$0")/../.."
source tools/wasm/env.sh
cargo build --release --target wasm32-unknown-unknown -p qurai
rm -rf dist/web
mkdir -p dist/web
wasm-bindgen --target web --no-typescript --out-dir dist/web --out-name qurai \
  target/wasm32-unknown-unknown/release/qurai.wasm
cp web/manifest.json web/icon.svg dist/web/
# Versiya belgisi fayl manzillariga qo'shiladi: brauzer keshida eski
# ilova qolib ketmasin (har yig'ilishda yangi manzil).
python - <<'PY'
import pathlib, time
v = str(int(time.time()))
html = pathlib.Path("web/index.html").read_text(encoding="utf-8")
html = html.replace('"./qurai.js"', '"./qurai.js?v=' + v + '"')
html = html.replace('"./qurai_bg.wasm"', '"./qurai_bg.wasm?v=' + v + '"')
pathlib.Path("dist/web/index.html").write_text(html, encoding="utf-8")
print("versiya", v)
PY
ls -la dist/web
# Oldindan siqilgan nusxalar: server brauzerga shularni beradi.
python - <<'PY'
import gzip, shutil, pathlib
for name in ["qurai_bg.wasm", "qurai.js"]:
    src = pathlib.Path("dist/web") / name
    with open(src, "rb") as f, gzip.open(str(src) + ".gz", "wb", compresslevel=9) as g:
        shutil.copyfileobj(f, g)
    print(name + ".gz", (pathlib.Path(str(src) + ".gz")).stat().st_size)
PY
