"""Brauzer yig'ilishi uchun arxivator: zig ar."""
import subprocess, sys
sys.exit(subprocess.call([sys.executable, "-m", "ziglang", "ar"] + sys.argv[1:]))
