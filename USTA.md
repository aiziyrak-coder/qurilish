# USTA — loyiha yo'l xaritasi

## Maqsad

TZ «Construction Intelligence Platform» bo'yicha Windows uchun native desktop
ilova qurish: qurilish loyihasini boshqarish, loyiha va smetani AI tekshiruvi,
ijro hujjatlari, ta'minot zanjiri, resurs va sifat nazorati.

## Stack va struktura

- **Rust + egui/eframe** — native oyna, GPU orqali chiziladi, webview yo'q
- **SQLite (rusqlite, WAL)** — to'liq lokal, server kerak emas
- `src/model.rs` — I modul turlari · `src/domain.rs` — II–XVI turlari
- `src/cpm.rs` — tarmoq grafigi · `src/checks.rs` — tekshiruv qoidalari
- `src/import.rs` — smeta importi · `src/db.rs` + `src/store.rs` — ombor
- `src/ui/` — har bir ekran alohida fayl · `src/i18n.rs` — uz/ru satrlar

Ishga tushirish: `cargo run --release` · Test: `cargo test` · Lint: `cargo clippy`

## Yo'l xaritasi (TZ modullari)

- [x] **I.1 Obyekt pasporti** — bosqichlar, moliya, ishtirokchilar, hujjat va foto
- [x] **I.2 GPR** — Gantt, CPM, tahlil oynasi, klaviatura boshqaruvi
- [x] **I.3 PPR** — kartalar, resurs gistogrammasi, qoplanish, 6 qoida
- [x] **II AI loyiha tekshiruvi** — bilimlar grafi, 12 qoida, normativ reyestri, ACTION
- [x] **III AI smeta tekshiruvi** — import, 8 qoida, qiymat nazorati
- [x] **IV Ijro hujjatlari** — bo'lim bo'yicha talablar reyestri
- [x] **V Ishlar jurnali** — fotofiksatsiya, GPR ni fakt bo'yicha yangilash
- [x] **Umumiy ko'rinish** — S-egri, diqqat paneli, yaqin 14 kun
- [x] **Umumiy qidiruv** — Ctrl+K, barcha modullar bo'ylab
- [x] **XII Materiallar** — katalog, sertifikat muddati, qoldiq ko'rsatkichi
- [x] **XI Ombor** — kirim/chiqim/hisobdan chiqarish, qoldiq va uning qiymati
- [x] **IX Arizalar** — ehtiyoj → ariza → tasdiqlash, qoplanish nazorati
- [x] **X Xaridlar** — ariza → xarid → yetkazish → omborga kirim
- [ ] **XIV Sifat** — kirish/operatsion/qabul nazorati  <- HOZIR SHU YERDA
- [ ] **XV Xavfsizlik** — buzilishlar, instruktajlar, naryad-dopusk
- [ ] **XIII Tabel** — ishchilar, soatlar, ishlarga taqsimot
- [ ] **XVI Mashinalar** — texnika, motosoat, YoMM, TX
- [ ] **XVII AI analitika** — qolgan modullar to'lgandan keyin
- [ ] **VI Prorab mobil ilovasi** — BLOKLANGAN: mobil klient + server kerak
- [ ] **VII Texnik nazorat kabineti** — BLOKLANGAN: rollar + server kerak
- [ ] **VIII Buyurtmachi kabineti** — BLOKLANGAN: rollar + server kerak
- [ ] **XVIII AI Copilot** — BLOKLANGAN: LLM integratsiyasi kerak
- [ ] **II.1–2 Chizmani tanish** — BLOKLANGAN: PDF/DWG/IFC kutubxonasi kerak

## Muhim qarorlar

- **Desktop klient native holicha qoladi.** Server qismi kerak bo'lganda
  ilova unga API orqali murojaat qiladi, o'zi webview ga aylanmaydi.
- **AI normativni o'ylab topmaydi** (TZ II.17). Har bir qoidaning me'yoriy asosi
  alohida reyestrga qo'lda kiritiladi; kiritilmagan bo'lsa xatoda «muhandis
  tekshiruvi talab qilinadi» deb yoziladi.
- **Dastur hukm chiqarmaydi** (TZ III.32). Fakt, hisob va xulosa ajratilgan;
  «oshirilgan» emas, «oshirilgan bo'lishi mumkin» deb yoziladi.
- **Har bir hisob bitta funksiyadan** — masalan resurs talabi tekshiruvda ham,
  gistogrammada ham `checks::resource_demand` dan olinadi, shunda diagramma va
  xato matni bir-biriga zid chiqmaydi.
- **Ombor qoldig'i saqlanmaydi, hisoblanadi** — `checks::stock_balances` kirim,
  chiqim va hisobdan chiqarishdan chiqaradi. Shunda hujjat bilan qoldiq hech
  qachon bir-biriga zid bo'lmaydi; manfiy qoldiq esa hujjatdagi xatoni ko'rsatadi.
- **Ta'minot zanjiri raqam orqali bog'lanadi** — xarid omborga kirim qilinganda
  kirim harakatining hujjat raqami xarid raqami bo'ladi. Shu sabab «kirim
  qilinganmi?» degan savolga javob alohida bayroqsiz, ma'lumotning o'zidan chiqadi
  va tugma ikki marta bosilsa ham takror kirim bo'lmaydi.
- **Baza migratsiyasi qo'shimcha ustunlar orqali** — eski baza ochilaveradi,
  ma'lumot yo'qolmaydi (test bilan qoplangan).

## Ochiq savollar / xavflar

- **Server va rollar** — VI, VII, VIII modullari shusiz mumkin emas. Qaror kerak:
  qachon va qanday texnologiyada quramiz?
- **Chizmani tanish** — II modulning haqiqiy qiymati shunga bog'liq. IFC dan
  boshlash mantiqan to'g'ri (ochiq format, grafga to'g'ridan-to'g'ri tushadi).
- **LLM** — «AI javob beradi» deb yozilgan joylar qoidalar dvigateli bilan
  yopilmaydi. Qaysi model, qayerda ishlaydi, ma'lumot chetga chiqadimi?
