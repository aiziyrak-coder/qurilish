# -*- coding: utf-8 -*-
"""TODO.md dagi umumiy hisob jadvalini belgilardan qayta hisoblaydi."""
import io, re, sys

s = io.open('TODO.md', encoding='utf-8').read()
lines = s.split('\n')

mods = []          # (sarlavha, done, part, open, lock)
cur = None
for ln in lines:
    if ln.startswith('## '):
        # Har bir «## » sarlavha oldingi bo'limni yopadi; hisobga faqat TZ
        # modullari va «Umumiy» bo'limi kiradi.
        m = re.match(r'^## ([IVX]+)\. (.+)$', ln)
        if m:
            cur = [f"{m.group(1)}. {m.group(2)}", 0, 0, 0, 0]
            mods.append(cur)
        elif ln.startswith('## Umumiy ('):
            cur = ["Umumiy (TZ dan tashqari)", 0, 0, 0, 0]
            mods.append(cur)
        else:
            cur = None
        continue
    if cur is None:
        continue
    m = re.match(r'^- \[([x~ ])\] ', ln)
    if not m:
        continue
    k = m.group(1)
    # Bir qator bir necha TZ bandini qamrashi mumkin: «12-20. ...».
    rng = re.search(r'^- \[[x~ ]\] (?:\U0001F512 )?(\d+)[–-](\d+)\.', ln)
    n = int(rng.group(2)) - int(rng.group(1)) + 1 if rng else 1
    cur[1 if k == 'x' else 2 if k == '~' else 3] += n
    if k == ' ' and '\U0001F512' in ln:
        cur[4] += n

rows = []
tot = [0, 0, 0, 0]
for name, d, p, o, l in mods:
    rows.append(f"| {name} | {d+p+o} | {d} | {p} | {o} | {l} |")
    tot = [tot[0]+d, tot[1]+p, tot[2]+o, tot[3]+l]
total = tot[0] + tot[1] + tot[2]
rows.append(f"| **Jami** | **{total}** | **{tot[0]}** | **{tot[1]}** | **{tot[2]}** | **{tot[3]}** |")

head = "| Modul | Talab | ✅ | 🟡 | ⬜ | shundan 🔒 |\n|---|---:|---:|---:|---:|---:|\n"
new = head + "\n".join(rows)

start = s.index("| Modul | Talab |")
end = s.index("\n\n", s.index("| **Jami**"))
s = s[:start] + new + s[end:]

pct = lambda n: round(n / total * 100)
old = re.search(r"Ya'ni \*\*~\d+ % to'liq\*\*, \*\*~\d+ % qisman\*\*, \*\*~\d+ % hali yo'q\*\*\.", s)
if old:
    s = s[:old.start()] + (
        f"Ya'ni **~{pct(tot[0])} % to'liq**, **~{pct(tot[1])} % qisman**, "
        f"**~{pct(tot[2])} % hali yo'q**."
    ) + s[old.end():]

io.open('TODO.md', 'w', encoding='utf-8', newline='\n').write(s)
sys.stdout.write(f"jami={total} done={tot[0]} part={tot[1]} open={tot[2]} lock={tot[3]}\n")
