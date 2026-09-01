# USTA — loyiha yo'l xaritasi

## Maqsad

TZ «Construction Intelligence Platform» bo'yicha Windows uchun native desktop
ilova: qurilish loyihasini boshqarish, loyiha va smetani tekshirish, ijro
hujjatlari, ta'minot zanjiri, resurs va sifat nazorati, kesishgan tahlil hamda
kvartiralarni sotish.

## Stack va struktura

- **Rust + egui/eframe** — native oyna, GPU orqali chiziladi, webview yo'q
- **SQLite (rusqlite, WAL)** — to'liq lokal, server kerak emas
- `src/model.rs` — I modul turlari · `src/domain.rs` — II–XX turlari
- `src/cpm.rs` — tarmoq grafigi · `src/checks.rs` — tekshiruv qoidalari
- `src/sales.rs` — sotuv hisobi · `src/analytics.rs` — kesishgan tahlil
- `src/copilot.rs` — savol-javob · `src/backup.rs` — zaxira nusxa
- `src/import.rs` — smeta importi · `src/db.rs` + `src/store.rs` — ombor
- `src/ui/` — har bir ekran alohida fayl · `src/i18n.rs` — uz/ru satrlar

Ishga tushirish: `cargo run --release` · Test: `cargo test` · Lint: `cargo clippy`

Diagnostika: `QURAI_SCREEN=<ekran>` kerakli ekranda ochadi, `QURAI_DB=<yo'l>`
boshqa bazada ishga tushiradi (sinov uchun).

## Yo'l xaritasi

Barcha TZ modullari va sotuv bo'limi qurilgan:

- [x] **I.1 Obyekt pasporti** · **I.2 GPR** (CPM, Gantt) · **I.3 PPR**
- [x] **II AI loyiha tekshiruvi** — bilimlar grafi, 12 qoida, normativ reyestri
- [x] **III AI smeta tekshiruvi** — import, 8 qoida, qiymat nazorati
- [x] **IV Ijro hujjatlari** · **V Ishlar jurnali** (fotofiksatsiya)
- [x] **VI Prorab ish o'rni** — kunlik ekran: ishlar, jurnal, tabel, smenalar
- [x] **VII Texnik nazorat kabineti** — ko'rib chiqish navbati va qarorlar
- [x] **VIII Buyurtmachi kabineti** — faqat o'qish uchun hisobot ko'rinishi
- [x] **IX Arizalar** · **X Xaridlar** — ehtiyoj → ariza → xarid → ombor
- [x] **XI Ombor** · **XII Materiallar** — qoldiq harakatlardan hisoblanadi
- [x] **XIII Tabel** · **XIV Sifat** · **XV Xavfsizlik** · **XVI Texnika**
- [x] **XVII AI analitika** — 8 yo'nalish, 24 qoida, sog'lomlik indeksi, hisobot
- [x] **XVIII Yordamchi** — savol-javob, faqat o'z bazasidan (til modelisiz)
- [x] **XIX Sotuv — shaxmatka** · **XX Shartnomalar va to'lovlar**
- [x] **Umumiy ko'rinish**, **umumiy qidiruv** (Ctrl+K), **zaxira nusxa**

## Muhim qarorlar

- **Desktop klient native holicha qoladi.** Server qismi kerak bo'lganda
  ilova unga API orqali murojaat qiladi, o'zi webview ga aylanmaydi.
- **AI normativni o'ylab topmaydi** (TZ II.17). Har bir qoidaning me'yoriy asosi
  alohida reyestrga qo'lda kiritiladi; kiritilmagan bo'lsa xatoda «muhandis
  tekshiruvi talab qilinadi» deb yoziladi.
- **Dastur hukm chiqarmaydi** (TZ III.32). Fakt, hisob va xulosa ajratilgan;
  «oshirilgan» emas, «oshirilgan bo'lishi mumkin» deb yoziladi.
- **Har bir hisob bitta funksiyadan.** Resurs talabi tekshiruvda ham,
  gistogrammada ham `checks::resource_demand` dan; sotuv qarzi ekranda ham,
  hisobotda ham `sales::deal_state` dan olinadi.
- **Hosilaviy holat saqlanmaydi, hisoblanadi.** Ombor qoldig'i — harakatlardan,
  kvartira holati — shartnomadan, ish haqi — tabel va stavkadan. Shuning uchun
  hujjat bilan ko'rsatkich hech qachon bir-biriga zid bo'lmaydi.
- **Yordamchida til modeli yo'q** (XVIII). Javob shu bazadagi hisobdan chiqadi va
  «Tekshirish» tugmasi manba ekranini ochadi. Model qo'shilganda shu funksiyalar
  unga asbob bo'lib beriladi — sonlar baribir bazadan olinadi.
- **Baza migratsiyasi qo'shimcha ustunlar orqali** — eski baza ochilaveradi,
  ma'lumot yo'qolmaydi (test bilan qoplangan).
- **Zaxira nusxa `VACUUM INTO` orqali.** WAL rejimida `.db` faylini shunchaki
  ko'chirish yetarli emas — yozilmagan tranzaksiyalar tushib qolishi mumkin.

## Qolgan yo'nalishlar (arxitektura qarori kerak)

Bular ilovaning ichida emas, undan tashqarida turadi:

1. **Server va ko'p foydalanuvchi.** VI–VIII kabinetlari hozir shu kompyuterdagi
   bazada ishlaydi. Bir nechta qurilma o'rtasida sinxronizatsiya, rollar bo'yicha
   kirish va masofadan imzolash uchun server qismi kerak.
2. **Chizmani tanish (TZ II.1–2).** PDF, DWG, DXF, RVT, IFC ni o'qish tashqi
   kutubxonani talab qiladi. Hozir loyiha elementlari qo'lda kiritiladi.
   IFC dan boshlash mantiqan to'g'ri — ochiq format, grafga to'g'ridan-to'g'ri tushadi.
3. **Til modeli (LLM).** Erkin matnli savol-javob va hujjat matnini tahlil qilish
   uchun. Qaror kerak: qaysi model, qayerda ishlaydi, ma'lumot chetga chiqadimi.
