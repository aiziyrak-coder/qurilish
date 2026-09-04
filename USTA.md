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
- `src/ifc.rs` — IFC o'qish · `src/roles.rs` — rollar · `src/package.rs` — almashish
- `src/llm.rs` — til modeli nuqtasi (ixtiyoriy `llm` xususiyati)
- `src/import.rs` — smeta importi · `src/db.rs` + `src/store.rs` — ombor
- `src/docgen.rs` — KS-2, KS-3, M-29, AOSR va Excel yozish · `src/pdf.rs` — PDF
- `src/ui/export.rs` — ekran jadvalini eksportga tayyorlash
- `src/actions.rs` — yordamchining takliflari va ularni bajarish
- `src/ui/` — har bir ekran alohida fayl · `src/i18n.rs` — uz/ru satrlar

Ishga tushirish: `cargo run --release` · Test: `cargo test` · Lint: `cargo clippy`
Tarmoq kodisiz yig'ish: `cargo build --release --no-default-features`
(OpenAI integratsiyasi yig'ilishga kiradi, lekin sozlamada yoqilib kalit
kiritilmaguncha ishlamaydi)

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
- [x] **XVIII Yordamchi** — savol-javob o'z bazasidan + OpenAI integratsiyasi;
      o'n bir xil qoralama amal, har javobda manba ko'rsatiladi
- [x] **XIX Sotuv — shaxmatka** · **XX Shartnomalar va to'lovlar**
- [x] **Umumiy ko'rinish**, **umumiy qidiruv** (Ctrl+K), **zaxira nusxa**
- [x] **II.1–2 Chizmani tanish** — IFC (ochiq format) o'qiladi va grafga tushadi
- [x] **Rollar** — 5 rol, ekran bo'yicha yozish huquqi, faqat-o'qish tasmasi
- [x] **Loyiha paketi** — qurilmalar orasida fayl orqali almashish
- [x] **Til modeli** — OpenAI bilan to'liq integratsiya, sukut bo'yicha o'chiq

### Chuqurlashtirilgan modullar (TZ bo'yicha ikkinchi qatlam)

- [x] **XI Ombor** — bir nechta ombor va ular orasida ko'chirish, partiyalar
      (sertifikat, yaroqlilik, FEFO), rezerv, inventarizatsiya, qaytarish
- [x] **XI–XII Sarf normalari** — normativ / fakt taqqoslash, ortiqcha sarf
      summasi (norma bajarilgan hajmga qarab hisoblanadi)
- [x] **XIII Tabel** — brigadalar, smenalar (koeffitsiyent bilan), yo'qliklar,
      bo'sh turish, ish tannarxi (ish haqi + material + texnika)
- [x] **X Xaridlar** — tijorat takliflari va ularni solishtirish, yetkazib
      beruvchilar tarixi, qisman yetkazish, bo'lim byudjeti
- [x] **IX Arizalar** — summaga qarab kelishuv marshruti, byudjet tekshiruvi,
      qaror tarixi, rad etish sababi
- [x] **XIV Sifat** — chek-listlar (normativ havolasi bilan), nazorat nuqtalari,
      Quality Score, brak tahlili, bosqichni yopishga to'siq
- [x] **XV Xavfsizlik** — ruxsatlar matritsasi, SIZ nazorati, naryad-dopusk va
      uni tekshirish, Safety Score
- [x] **XVI Texnika** — yo'l varaqalari, yoqilg'i normasi, rejali TX,
      foydalanish koeffitsiyenti, ishlatishga to'siq
- [x] **XVII Analitika** — oylik pul oqimi, kassa uzilishi, kunlik xulosa
- [x] **Hujjat generatsiyasi** — KS-2, KS-3, M-29 va yashirin ishlar
      dalolatnomasi `.xlsx` shaklida; son o'ylab topilmaydi, imzo joyi bo'sh
- [x] **Excel eksporti** — 15 ta ekran jadvali, `Ctrl+E`; sonlar son bo'lib
      chiqadi, sarlavha qatori qotadi va filtr qo'yiladi
- [x] **Amallar tarixi** — har bir yozish `store.rs` ning uchta chorrahasidan
      o'tadi, shuning uchun bironta o'zgarish jurnaldan chetda qolmaydi
- [x] **XVIII Yordamchi — operatsion qatlam** — bazadagi holatdan taklif
      chiqaradi (ariza, xarid, kirim, ijro hujjati, muddat, naryad, marshrut),
      tasdiqdan keyin bajaradi va rol huquqini hurmat qiladi
- [x] **XII Materiallar** — smeta va spetsifikatsiya havolasi, tasdiqlangan
      analoglar, narx tarixi, brak va yetkazib beruvchiga qaytarish, taqiq,
      to'liq kuzatuvchanlik (yetkazuvchi → partiya → ish → hujjat) va ish
      boshlanishidan oldin material yetarliligini tekshirish

Qolgan ishlar `TODO.md` da modul bo'yicha ro'yxatlangan. Xulosa jadvali
belgilardan hisoblanadi va qulflangan (server, mobil, LLM, OCR talab
qiladigan) bandlar alohida ustunda ko'rinadi.

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
- **Yordamchi sonni o'ylab topmaydi** (XVIII). Javob shu bazadagi hisobdan
  chiqadi; har javobda sonlar qaysi yozuvlardan olingani yoziladi va
  «Tekshirish» tugmasi manba ekranini ochadi. Til modeli yoqilganda ham
  sonlar unga **tayyor holda** biriktiriladi — model hisoblamaydi.
- **Baza migratsiyasi qo'shimcha ustunlar orqali** — eski baza ochilaveradi,
  ma'lumot yo'qolmaydi (test bilan qoplangan).
- **Zaxira nusxa `VACUUM INTO` orqali.** WAL rejimida `.db` faylini shunchaki
  ko'chirish yetarli emas — yozilmagan tranzaksiyalar tushib qolishi mumkin.

## Uch yo'nalish bo'yicha qabul qilingan yechim

TZ ning uchta «bloklangan» qismi ilova ichida hal qilindi. Har birida chegara
ochiq aytilgan — nima qilinadi va nima qilinmaydi.

**1. Chizmani tanish (II.1–2) → IFC.** IFC ochiq matnli format, shuning uchun
`src/ifc.rs` da STEP parseri yozildi: 30 dan ortiq IFC turi element turlari va
bo'limlarga moslashtirildi, bog'lanishlar (qavat → element, devor → teshik →
eshik/deraza) grafga tushadi. **DWG va RVT yopiq formatlar** — ular hujjat
sifatida biriktiriladi, tanilmaydi.

**2. Rollar va ko'p qurilma.** Rol (`src/roles.rs`) ekranni yashirmaydi, balki
**yozish huquqini** belgilaydi: buyurtmachi ko'radi lekin o'zgartirmaydi, prorab
ijroni to'ldiradi lekin smetani emas. Qurilmalar orasida ma'lumot **fayl orqali**
ko'chadi (`src/package.rs`): maydonchada to'ldirilgan kunlik ijro paketga
chiqariladi va ofisdagi bazaga qo'shiladi. **Bu parol bilan himoya emas** va
jonli sinxronizatsiya emas — baza fayli ochiq, buni sozlamalar ham aytadi.
Haqiqiy kirish nazorati server qismi bilan keladi.

**3. Til modeli.** `src/llm.rs` — OpenAI bilan to'liq integratsiya: ko'p
bosqichli suhbat, fon so'rovi, xatolarni aniq ko'rsatish, token hisobi.
**Sukut bo'yicha o'chiq**: sozlamada yoqilib API kalit kiritilmaguncha ilova
hech qayerga ulanmaydi; kalit faqat `Authorization` sarlavhasida ketadi va
logga tushmaydi. Tarmoq kodi umuman kerak bo'lmasa —
`cargo build --no-default-features`. Yoqilganda savol va unga biriktirilgan
sonlar tashqi xizmatga jo'natiladi — sozlamalarda bu ochiq ogohlantirish bilan
yozilgan va yoqishni foydalanuvchi o'zi tanlaydi. Modelga beriladigan ko'rsatma
qat'iy: **sonni o'ylab topma, bilmasang ochiq ayt**. So'rov tuzish va javobni
o'qish tarmoqsiz sinaladi.

## Holat (2026-09-04)

TZ bo'yicha **638 / 700 band to'liq**, 16 tasi qisman (har birida nima
bloklagani TODO.md da yozilgan), 46 tasi tashqi omilga bog'liq.

Tekshiruv: `cargo test` — 350 sinov o'tadi (`--no-default-features` bilan ham),
`cargo clippy --all-targets` — ogohlantirishsiz, `cargo fmt --check` — toza,
`cargo build --release` — yig'iladi.

Qo'riqchi sinovlar (regressiyani ushlaydi):

- har bir ekran va uning har bir tabi oynasiz chiziladi — namuna bazasi,
  bo'sh baza, faqat-o'qish roli, tor oyna va chekka ma'lumotda
- eksport qilinadigan har bir ekranda jadval bor va ustunlar mos
- i18n: takroriy kalit yo'q, ishlatilgan kalitning tarjimasi bor, ruscha
  matn nusxa emas, sanaladigan qiymatlarning nomi bor
- rollar: jurnalga yozadigan rol uni haqiqatda o'zgartira oladi
- yordamchi: har javobda manba bor, har ma'lumot ekranidan kirish bor

## Server (`server/`)

Alohida krat: `axum` + SQLite. Desktop ilovaga uning kutubxonalari kirmaydi.

- **Kirish nazorati** — login va Argon2id parol xeshi, 32 baytli seans belgisi,
  rol bo'yicha huquq (ko'rish / yozish / imzolash / boshqarish). Bu yerda rol
  **haqiqiy chegara**: bazaga faqat server tegadi.
- **Almashish** — qurilma paketni yuboradi, boshqalari tartib raqamidan keyingi
  paketlarni oladi. Server paketni ochmaydi: format desktop ilovaniki.
- **Masofadan imzolash** — kim, qachon va qaysi matnni tasdiqlagani; matn xeshi
  bilan bog'lanadi. Bu davlat ERI si emas va shunday deb atalmaydi.
- **Mobil ko'rinish** — telefon brauzeri uchun sahifalar (JavaScript yo'q):
  obyektlar, o'zgarishlar, imzo va kunlik yozuv formasi.

Ishga tushirish: `qurai-server --add-user prorab "Ism" foreman`, keyin
`QURAI_BIND=0.0.0.0:8080 qurai-server`. Server HTTP beradi — internetga
chiqarilganda HTTPS proksi ortida turishi shart (dastur buni ishga tushirishda
ham aytadi).

## Chizma va hujjatlarni o'qish

- **IFC** (`src/ifc.rs`) — ochiq BIM formati: elementlar va bog'lanishlar.
- **DXF** (`src/dxf.rs`) — CAD ning ochiq formati: matnlar (marka, xona nomi) va
  qatlamlar. Geometriya olinmaydi va bu ochiq aytiladi.
- **PDF** (`src/pdfread.rs`) — jadval matn parchalarining joylashuvidan
  tiklanadi; shrift `ToUnicode` jadvali orqali o'qiladi.
- **Skan** (`src/ocr.rs`) — mahalliy Tesseract ga ko'prik; o'rnatilmagan bo'lsa
  nima kerakligi aytiladi. Hech narsa internetga chiqmaydi.
- **DWG/RVT** — yopiq formatlar: CAD dan DXF eksport qilinadi.

## Keyingi qadamlar (ilovadan tashqarida)

1. **Native mobil ilova** — hozir telefon brauzeri orqali ishlanadi.
2. **Push bildirishnomalar** — SMS/Telegram yoki mobil ilova kerak.
3. **Davlat ERI si** — kalitlar va akkreditatsiya masalasi.
4. **Til modeli tanlovi**: qaysi model, qayerda ishlaydi (lokal yoki bulut),
   ma'lumot chetga chiqishi bo'yicha tashkiliy qaror.
