"""Brauzer (wasm32) uchun C kompilyatori: zig cc orqali clang.

cc kutubxonasi `--target=wasm32-unknown-unknown` beradi; zig uni
`wasm32-freestanding` deb tushunadi. Qolgan bayroqlar o'zgarmaydi.
"""
import subprocess, sys
args = []
skip = False
for a in sys.argv[1:]:
    if skip:
        skip = False
        continue
    if a.startswith("--target=") or a.startswith("-target="):
        continue
    if a == "-target":
        skip = True
        continue
    args.append(a)
cmd = [sys.executable, "-m", "ziglang", "cc", "-target", "wasm32-freestanding"] + args
sys.exit(subprocess.call(cmd))
