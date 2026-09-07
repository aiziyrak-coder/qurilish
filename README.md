# QURAi — Construction Intelligence Platform

Windows uchun native desktop ilova: **Rust + egui/eframe**, GPU orqali chiziladi.
Hech qanday webview, HTML yoki o'rnatilgan brauzer yo'q — oyna to'g'ridan-to'g'ri chiziladi.

Texnik topshiriqning **I–V modullari** amalga oshirilgan: qurilish loyihasini
boshqarish (obyekt pasporti, GPR va PPR), loyihani bo'limlararo tekshirish,
smetani tekshirish, ijro hujjatlari va kundalik ishlar jurnali.

*Нативное десктоп-приложение для Windows на Rust + egui. Реализованы модули I–V ТЗ:
паспорт объекта, ГПР с расчетом критического пути, ППР, межразделная проверка
проекта, проверка смет, исполнительная документация и журнал работ.*

## Ishga tushirish

```bash
cargo run --release
```

Tayyor fayl: `target/release/qurai.exe` (~19 MB, tashqi bog'liqliksiz —
SQLite, Excel va PDF yozish, IFC/DXF/PDF o'qish hammasi ichida).
Ma'lumotlar bazasi — exe yonidagi `data/qurai.db`; birinchi ishga tushirishda
namoyish obyekti yaratiladi: «Navro'z» TJM, 20 ta ish, 25 ta bog'lanish,
22 ta loyiha elementi, 10 pozitsiyali smeta, 4 ta PPR kartasi, jurnal yozuvlari
va ijro hujjatlari.

Testlar (CPM, tekshiruv qoidalari, import, ombor):

```bash
cargo test
```

Baza fayli sukut bo'yicha dastur yonidagi `data/qurai.db` da. `QURAI_DB` muhit
o'zgaruvchisi bilan boshqa yo'lni ko'rsatish mumkin — sinov yoki portativ
ishlatish uchun qulay.

Sozlamalarda **zaxira nusxa** olish mumkin: bazaning izchil nusxasi bitta faylga
yoziladi (`VACUUM INTO`), shuning uchun WAL rejimida ham hech narsa tushib
qolmaydi. Kod git bilan qaytadi, ma'lumot esa qaytmaydi — nusxani muntazam oling.

## Umumiy qidiruv

`Ctrl+K` yoki `/` — barcha modullar bo'ylab qidiradi: GPR ishlari, loyiha
elementlari, smeta pozitsiyalari, nomuvofiqliklar, PPR kartalari va bo'lim
nomlari. Natija tanlanganda tegishli ekran ochiladi va yozuv ajratiladi.
Yuqori panelda tugmasi ham bor.

Tekshiruvlar modul ekrani birinchi ochilganda **avtomatik** ishga tushadi —
bo'sh ro'yxat o'rniga natija darhol ko'rinadi. Saqlangan natija bo'lsa yoki
foydalanuvchi yozuv holatini o'zgartirgan bo'lsa, u qaytadan hisoblanmaydi.

## Til va ko'rinish

- **Ikki til: o'zbekcha va ruscha.** Standart til — **o'zbekcha**.
  Interfeysdagi barcha satrlar `src/i18n.rs` da bitta ro'yxatda saqlanadi.
- **Mavzu: yorug' (standart) va qorong'i.**
- Til, mavzu va boshqa sozlamalar **Sozlamalar** sahifasida o'zgartiriladi va
  `settings` jadvalida saqlanadi — ilova qayta ochilganda tiklanadi.

Bo'lim qisqartmalari tilga moslashadi: o'zbekchada `AR / KJ / KM / VK / OV / EOM / SS / PB`,
ruschada `АР / КЖ / КМ / ВК / ОВ / ЭОМ / СС / ПБ`. Bazada esa tilga bog'liq bo'lmagan
barqaror kod saqlanadi, shuning uchun til almashtirish ma'lumotni buzmaydi.

## Amalga oshirilgan bo'limlar

### Obyekt pasporti (TZ I.1)
Bosqichlar chizig'i (loyihalash → tender → qurilish → yakunlangan, alohida
«to'xtatilgan» holati), obyekt kartochkasi (turi, qavatlar, umumiy maydon),
muddatlar vaqt chizig'i bilan, moliyalashtirish — shartnoma summasi, to'langan
summa va to'lov polosasi. Ishtirokchilar TZ dagi olti rol bo'yicha guruhlanadi,
qo'shilmagan rol darrov taklif qilinadi. O'ng panelda obyekt holati (boshqa
modullardan yig'ilgan sonlar) va pasport to'ldirilishi nazorati.
Hujjatlar va foto: fayllar ko'chirilmaydi, bazada yo'li saqlanadi; rasmlar
galereyada ko'rinadi, fayl joyidan olinsa buni ro'yxat ochiq ko'rsatadi.

### GPR — ishlar grafigi (TZ I.2)
- **Gantt diagrammasi** o'z chizmasi bilan: kun / hafta / oy masshtabi.
  Vaqt o'qi sarlavhasi ikki qatorli va zumga moslashadi: pastda kunlar bo'lsa
  yuqorida oylar, pastda oylar bo'lsa yuqorida yillar turadi. Qadam masshtabga
  emas, haqiqiy piksel kengligiga qarab tanlanadi — yozuvlar hech qachon
  bir-birining ustiga chiqmaydi.
- **Aylantirish**: g'ildirak — ishlar ro'yxati bo'ylab, ro'yxat oynaga to'liq
  sig'sa g'ildirak vaqt o'qini suradi; `Shift`+g'ildirak yoki gorizontal
  g'ildirak — doim vaqt o'qi; `Ctrl`+g'ildirak — zum; diagrammaning bo'sh
  joyini sichqoncha bilan sudrash — panorama; pastda gorizontal polosa.
- **CPM** — to'g'ri va teskari o'tish, erta/kech sanalar (ES/EF/LS/LF),
  umumiy zaxira, kritik yo'l qizil rangda ajratiladi.
- **To'rt xil bog'lanish** (FS, SS, FF, SF) musbat va manfiy lag bilan; strelkalar
  diagrammada chiziladi. Sikl hosil qiluvchi bog'lanish qabul qilinmaydi —
  qo'shilgan bog'lanish darhol qaytariladi va foydalanuvchiga aytiladi.
- **Drag & drop**: polosani surish ishni siljitadi (u avtomatik mahkamlanadi, aks holda
  CPM uni qaytarib qo'yardi), chekkasidan tortish davomiylikni o'zgartiradi.
  Sudrash turi sudrash boshlanishida qulflanadi. Grafik darhol qayta hisoblanadi.
- **Plan/fakt**: bugungi rejadagi foiz va haqiqiy foiz, muddati o'tgan ishlar,
  bugun bajarilayotgan ishlar, tugash prognozi va kechikish kunlari.
- **Jadval**: VBS, nom, bo'lim, CPM bo'yicha boshlanish va tugash sanalari,
  davomiylik, bajarilish (son va polosa), zaxira.
- **Tahlil oynasi** TZ I.2 savollariga javob beradi: qaysi ishlar muddati o'tgan,
  mas'ullar kesimida kim qancha kun kechiktirgan, joriy sur'atda obyekt qancha
  kechikadi, vaqtida topshirish uchun sur'atni necha barobar oshirish kerak va
  kritik yo'ldagi navbatdagi ishlar qaysilar.
- **Klaviatura**: strelkalar — tanlov, `Ctrl`+strelka — ishni bir kunga surish,
  `Alt`+strelka — ro'yxatdagi tartib, `Delete` — o'chirish, `Esc` — bekor qilish.
- Bo'lim, kritik yo'l, muddati o'tganlar va matn bo'yicha filtrlar.

### PPR — ishlar rejasi (TZ I.3)
To'rt bo'limdan iborat: **kartalar** (PPR, texnologik, sifat va xavfsizlik
kartalari — raqami, nomi, bo'limi, GPR ishi, ishlab chiqqan, talab qilinadigan
odam va texnika, tasdiq sanasi, biriktirilgan fayl), **resurslar**,
**ishlar qoplanishi** va **nomuvofiqliklar**.

**Resurslar** — kunlik talab gistogrammasi: grafikning har bir kuni uchun o'sha
kuni ketayotgan ishlarning kartalaridagi odam va texnika yig'indisi. Mavjud
quvvat yashil punktir chiziq bilan ko'rsatiladi, undan oshgan kunlar qizil
bo'yaladi va nechta kun ortiqcha yuklangani yoziladi. Hisob tekshiruv bilan
bir xil funksiyadan olinadi — diagramma va xato matni bir-biriga zid chiqmaydi.

**Ishlar qoplanishi** — har bir GPR ishi qarshisida uning kartasi bor-yo'qligi,
tasdiqlanganmi va kritik yo'ldami. Kartasi yo'q ish uchun uni shu yerdan bir
bosishda yaratish mumkin.

TZ I.3 dagi beshta tekshiruv ham bajariladi:
- **PPR loyihaga mos keladimi** — boshlangan ishda karta bormi, karta mavjud
  ishga bog'langanmi;
- **odam va texnika yetadimi** — kunlik cho'qqi mavjud resurs bilan solishtiriladi
  (resurs ko'rsatilmagan bo'lsa, xulosa chiqarilmaydi);
- **ketma-ketlik** — ish boshlangan, lekin undan oldin tugashi kerak bo'lgan ish
  tugallanmagan (oldingisi yarmigacha yetmagan bo'lsa — kritik);
- **risklar** — kritik yo'ldagi ish 30 kun ichida boshlanadi, tasdiqlangan
  texnologik kartasi esa yo'q.

### AI loyiha tekshiruvi (TZ II)
- **Loyiha bilimlar grafi**: element (xona, deraza, teshik, quvur, rigel, kabel…)
  va ular orasidagi bog'lanish (`tarkibida`, `xizmat qiladi`, `kesib o'tadi`,
  `tayanadi`, `quvvat oladi`). Elementlar va bog'lanishlar ekrandan tahrirlanadi.
- **Bo'limlararo tekshiruv (II.11)**: teshigi ko'zda tutilmagan deraza yoki eshik
  (AR ↔ KJ), muhandislik ta'minotisiz xona (AR ↔ VK / OV / EOM), konstruksiyani
  teshiksiz kesib o'tuvchi tarmoq, kanalizatsiya ukloni, takrorlangan markalar,
  varaqsiz va o'lchamsiz elementlar, grafda yolg'iz qolgan elementlar.
- **O'zgarish ta'siri (II.18)**: element o'zgarganda qaysi bo'limlar va nechta
  element ta'sirlanishi grafni aylanib chiqib ko'rsatiladi.
- **Normativ reyestri (II.17)**: dastur normativni **o'ylab topmaydi**. Har bir
  qoidaning me'yoriy asosi (hujjat, tahriri, band, matn, chegara qiymati, manba)
  reyestrga qo'lda kiritiladi. Kiritilmagan bo'lsa, nomuvofiqlikda me'yoriy asos
  maydoni bo'sh qoladi va «muhandis tekshiruvi talab qilinadi» deb yoziladi.
  Chegara qiymati tasdiqlanmagan bo'lsa, qoida vaqtinchalik qiymatdan foydalanadi
  va buni xatoning o'zida ochiq aytadi.
- **Bilimlar grafi ko'rinishi (II.18)**: elementlar bo'limlar bo'yicha ustunlarga
  joylashtiriladi, bog'lanishlar chiziq bilan chiziladi. Tugun tanlansa, uning
  aloqalari ajratiladi va qolganlari xiralashadi — bo'limlararo bog'lanishlar
  shu ko'rinishda bir qarashda o'qiladi.
- **Bajarish rejasi (II.19 ACTION)**: ochiq nomuvofiqliklar mas'ullar bo'yicha
  guruhlanadi — kim nima qilishi va qachongacha. Har bir yozuvga bartaraf etish
  muddati qo'yiladi, muddati o'tgani qizil bilan belgilanadi, holat shu yerdan
  o'zgartiriladi. Muddat qayta tekshiruvda saqlanib qoladi.

### AI smeta tekshiruvi (TZ III)
**Import (TZ III.2)**: XLSX, XLS, XLSB, ODS va CSV. Sarlavha qatori faylning
boshida bo'lishi shart emas — ustunlar nomi bo'yicha ikkala tilda topiladi,
«Итого» qatori yakuniy summaga tushadi, bo'lim sarlavhalari pozitsiya sifatida
sanalmaydi. Summa ustuni yo'q bo'lsa, u miqdor × narxdan hisoblanadi, shunda
arifmetika tekshiruvi soxta xato bermaydi.

Tekshiruvlar: arifmetika (miqdor × narx = summa), yakuniy summaning mosligi,
takrorlangan pozitsiyalar, o'lchov birliklari, loyiha hajmi bilan solishtirish,
narx bo'yicha chetlanish, nol va manfiy qiymatlar, GPR da bor-u smetada yo'q
ishlar. Pozitsiyalar ekranda tahrirlanadi, «Hisoblangan» va «Farq» ustunlari
darhol qayta hisoblanadi.

**Qiymat nazorati (TZ III.31)** — smetaning pul ko'rinishidagi xulosasi:
tekshirishga arziydigan umumiy summa, takrorlangan pozitsiyalar qiymati,
loyihadan oshgan hajmning narxi, arifmetik farqlar yig'indisi va narxni
tenglashtirishdan kelib chiqadigan tejam. Har bir raqamning yonida u qanday
hisoblangani yozilgan, pastda esa bo'limlar kesimidagi qiymat diagrammasi.

TZ III.32 talabiga ko'ra dastur hukm chiqarmaydi: fakt, hisob va xulosa ajratilgan,
sarlavhalar «oshirilgan» emas, «oshirilgan bo'lishi mumkin» deb yoziladi;
qiymat nazoratidagi sonlar ham «xato» emas, «tekshirishga arziydi» deb beriladi.

### Ijro hujjatlari (TZ IV)
Hujjat turi, raqami, sanasi, GPR ishi, holati (qoralama → ko'rib chiqishda →
imzolangan) va mas'ul.

Modulning asosiy g'oyasi — TZ IV.1 dagi «tizim har bosqichda qaysi hujjat kerakligini
bilishi kerak» — **talablar reyestri** bilan bajarilgan: hujjat turi ishning bo'limiga
qarab aniqlanadi (KJ va KM uchun yashirin ishlar dalolatnomasi va ijro sxemasi,
VK/OV/EOM/SS uchun yashirin ishlar va sinov bayonnomasi, PB uchun sinov va qabul,
AR uchun qabul dalolatnomasi). Yon panelda uch holat ko'rsatiladi: hujjat yo'q va
ish tugagan (**kechikdi**), hujjat yo'q va ish ketmoqda (**kutilmoqda**), hujjat bor
lekin **imzolanmagan**. Yo'q hujjat bir bosishda kerakli tur bilan yaratiladi.

### Kundalik ishlar jurnali (TZ V)
Har kungi yozuv: sana, muallif, ob-havo va harorat, ishchilar va texnika soni,
bajarilgan ish va hajm, e'tirozlar va **fotofiksatsiya** — fotolar yozuvga
biriktiriladi va galereyada ko'rinadi (fayl ko'chirilmaydi, yo'li saqlanadi).
Yuqorida jurnal holati: oxirgi yozuv qachon, oxirgi 30 kunning nechtasi
to'ldirilgan, o'rtacha ishchi soni va biriktirilgan foto soni.
«GPR ni fakt bo'yicha yangilash» tugmasi
jurnaldagi hajmlar yig'indisini ishning bajarilish foiziga aylantiradi va
boshlanish sanasini qo'yadi — shundan keyin plan/fakt haqiqiy ma'lumotga tayanadi.
Hajmi ko'rsatilmagan ish tegilmaydi va bu haqda ochiq aytiladi.

### Nomuvofiqliklar bilan ishlash
Uchala tekshiruv (loyiha, smeta, PPR) bitta ro'yxat va bitta kartochkani ishlatadi: muhimlik darajasi,
ichki kod, bo'lim, element, joylashuv, varaq, tavsif, me'yoriy asos, tavsiya va
mas'ul. Holat `Ochiq → Ishlanmoqda → Tuzatilgan / Rad etilgan` bo'yicha yuritiladi;
holati o'zgartirilgan yozuv qayta tekshiruvda tiklanmaydi. Xato kodlari ishga
tushirishlar orasida barqaror — buni test tekshiradi.

### Arizalar (TZ IX)
Ta'minot zanjirining birinchi bo'g'ini: ehtiyoj → ariza → tasdiqlash. Har bir
arizada raqam, sana, holat, muhimlik, tur, nomi, material, miqdor, kerak bo'lgan
sana va so'ragan shaxs. Ustunlar nazorat tartibida joylashgan — avval holat va
qoplanish, keyin tafsilotlar.

«Qoplanish» ustuni bog'langan xaridlardan hisoblanadi: qancha buyurtma qilingan,
qanchasi kelgan, rejalashtirilgan yetkazish ehtiyoj sanasidan kechikadimi.
«Zaxira bo'yicha ariza» tugmasi qoldig'i minimal zaxiradan past bo'lgan har bir
material uchun ariza ochadi — miqdor zaxirani tiklashga yetadigan qilib olinadi;
o'sha material bo'yicha ochiq ariza bo'lsa dubl yaratilmaydi.

### Xaridlar (TZ X)
Ikkinchi bo'g'in: ariza → xarid → yetkazish. Yetkazib beruvchi, miqdor, narx,
summa, yetkazish sanasi va holat. «Arizalar bo'yicha xarid» tasdiqlangan, ammo
xaridi ochilmagan arizalar uchun xarid yaratadi.

Yetkazilgan xarid omborga tushishi kerak. «Ombor» ustuni har bir xarid uchun
kirim qilinganmi yoki yo'qmi ko'rsatadi, «Omborga kirim qilish» tugmasi esa
kirim harakatlarini yaratadi. Bog'lanish hujjat raqami orqali: kirim harakati
xarid raqami bilan yoziladi, shuning uchun tugma ikki marta bosilsa ham takror
kirim bo'lmaydi. Miqdori arizadagidan oshib ketgan xaridlar jadval ostida fakt
sifatida qayd etiladi (TZ III.32: dastur hukm chiqarmaydi).

### Materiallar (TZ XII)
Obyektning yagona material katalogi: kod, nomi, birlik, bo'lim, texnik tavsif,
sertifikat raqami va amal qilish muddati, minimal zaxira va narx. Yuqorida beshta
ko'rsatkich: katalog hajmi, ombor qiymati, zaxiradan kam tushgan pozitsiyalar,
sertifikat muddati o'tganlar (yaqin 30 kun ichida tugaydiganlar bilan) va
sertifikati kiritilmaganlar. Har bir qator yonida ombordagi joriy qoldiq
ko'rsatiladi, shunda katalog va ombor bir-biridan ajralib qolmaydi.

### Ombor (TZ XI)
Ikki ko'rinish. **Qoldiqlar** — material bo'yicha joriy holat, minimal zaxiraga
nisbatan polosa, birlik narxi, qoldiq qiymati, kirim/chiqim yig'indisi va oxirgi
harakat sanasi. **Harakatlar** — kirim, chiqim va hisobdan chiqarish jurnali:
sana, tur, material, miqdor, narx, hujjat, kontragent va bog'liq ish.

Qoldiq alohida saqlanmaydi — u har safar harakatlardan hisoblanadi
(`checks::stock_balances`), shuning uchun hujjat bilan qoldiq hech qachon
bir-biriga zid bo'lmaydi. Birlik narxi kirimlarning vaznlangan o'rtachasidan
olinadi; kirimda narx ko'rsatilmagan bo'lsa katalogdagi narx ishlatiladi.
Chiqim kirimdan ko'p yozilsa qoldiq manfiy bo'ladi va bu hujjatdagi xato deb
alohida ajratib ko'rsatiladi.

### Tabel (TZ XIII)
Haftalik jadval: qatorlar — ishchilar, ustunlar — hafta kunlari, katakda soat.
Bugungi kun va dam olish kunlari sarlavhada ajratilgan; kun bo'yicha va ishchi
bo'yicha yig'indilar chetda turadi. Ish haqi soat va soatlik stavkadan
hisoblanadi — alohida kiritilmaydi, shuning uchun tabel bilan hech qachon zid
bo'lmaydi. Ko'rsatkichlar: faol ishchilar, haftalik soat, kunlik o'rtacha va ish
haqi fondi.

### Sifat nazorati (TZ XIV)
Uch bosqich: **kirish** (material qabuli), **operatsion** (ish jarayonida) va
**qabul** (bosqich yakuni). Har yozuvda nazorat obyekti, ish yoki material,
tekshiruvchi, natija (mos / shartli mos / mos emas), nuqson tavsifi va bartaraf
etish muddati. Muddati o'tgan, bartaraf etilmagan nuqson alohida ko'rsatkichda
va jadvalda qizil undov bilan ajraladi.

### Mehnat xavfsizligi (TZ XV)
Hodisalar jurnali: buzilish, xavfli holat, baxtsiz hodisa, tekshiruv va
instruktaj. Har yozuvda muhimlik darajasi, joy, tavsif, ko'rilgan chora, mas'ul,
muddat va holat. Yopilmagan va muddati o'tgan yozuvlar ko'rsatkichlarda alohida.
TZ III.32 ga muvofiq ekran fakt qayd etadi, aybdorni belgilamaydi.

### Texnika (TZ XVI)
Ikki ko'rinish. **Park** — texnika ro'yxati, turi, davlat raqami, holati
(ishlamoqda / bo'sh / ta'mirda / o'chirilgan), egasi, operatori, soatlik stavkasi,
texnik ko'rik muddati va oxirgi 30 kundagi motosoati. **Smenalar** — kunlik
motosoat va yoqilg'i jurnali, ishga bog'lanishi bilan. Motosoat, yoqilg'i va
xarajat smenalardan hisoblanadi; texnik ko'rik muddati tugagan yoki 30 kun ichida
tugaydigan texnika alohida ogohlantiriladi.

### Sotuv — shaxmatka (XIX)
Bino sotuv kesimida: **blok (podez) → qavat → kvartira**. Shaxmatkada vertikal o'q
qavatlar (yuqoridan pastga), gorizontal o'q qavatdagi kvartiralar; har katakda
raqam, xonalar soni (tijorat joyida — turning qisqartmasi) va maydon, katak rangi
esa holatni bildiradi: **bo'sh, band qilingan, shartnoma, sotilgan, sotuvda emas**.
Muddati o'tgan to'lovi bor kvartira o'ng pastda qizil nuqta bilan belgilanadi.

Katak bosilganda o'ng panelda kvartira kartochkasi ochiladi: raqam, qavat, o'rni,
turi, xonalar, umumiy va yashash maydoni, 1 m² narxi, umumiy narx, holat,
planirovka. Shu yerdan bir bosishda band qilish ochiladi yoki mavjud shartnomaga
o'tiladi. «Kvartiralarni yaratish» blokning har bir qavati uchun bir xil
kvartiralarni hosil qiladi — band raqamlar o'tkazib yuboriladi, shuning uchun
tugma ikki marta bosilsa dubl bo'lmaydi.

«Ro'yxat» ko'rinishi barcha bloklardagi kvartiralarni jadval sifatida beradi:
narx va holatni to'g'ridan-to'g'ri tahrirlash, mijoz ustuni bilan.

### Shartnomalar va to'lovlar (XX)
Shartnoma kvartiraga bog'lanadi. Shartlar: mijoz, telefon, hujjat, **to'lov turi**
(naqd, muddatli to'lov, kredit/ipoteka, subsidiya, barter, aralash), narx,
chegirma, boshlang'ich to'lov, muddat va menejer. Holat: band qilingan →
imzolangan → to'liq to'langan (yoki bekor qilingan).

Kvartira holati alohida saqlanmaydi — u shartnoma holatidan kelib chiqadi, shuning
uchun shaxmatkadagi rang shartnoma bilan hech qachon zid bo'lmaydi. Shartnoma
o'chirilsa kvartira yana bo'sh bo'ladi.

**To'lov grafigi** shartnoma shartlaridan quriladi: birinchi qator — boshlang'ich
to'lov, qolgani teng oylik ulushlar; yaxlitlash qoldig'i oxirgi oyga qo'shiladi,
shuning uchun grafik summasi shartnoma summasiga aniq to'g'ri keladi (mos kelmasa
ekran buni ochiq aytadi). Har qatorda reja va fakt yonma-yon; muddati o'tgan
to'lanmagan qator qizil undov bilan ajraladi. Qarz shartnoma summasidan
hisoblanadi, grafikdan emas — grafik to'ldirilmagan bo'lsa ham qarz to'g'ri
ko'rinadi.

### Chizmadan o'qish — IFC (TZ II.1–2)
«AI loyiha tekshiruvi» ekranidagi **«IFC dan o'qish»** tugmasi IFC faylini o'qib,
elementlarni bilimlar grafiga qo'shadi. IFC — ochiq matnli format (ISO 10303-21),
shuning uchun u tashqi kutubxonasiz o'qiladi.

O'qiladigan narsa: devor, eshik, deraza, ustun, rigel, plita, teshik, xona,
truba, vozduxovod, kabel va qurilmalar — jami 30 dan ortiq IFC turi. Har element
o'z bo'limiga (AR, KJ, VK, OV, EOM…) tushadi, qavat nomi va xona `IFC` manbasi
belgisi bilan saqlanadi. Bog'lanishlar ham quriladi: qavat → element,
devor → teshik → eshik/deraza, xona → chegaradagi elementlar.

Import **qo'shadi, o'chirmaydi**; qayta import dublikat yaratmaydi (marka
bo'yicha solishtiriladi). Buzilgan qator butun faylni yo'qotmaydi. **DWG va RVT
yopiq formatlar** — ular tanilmaydi va hujjat sifatida biriktiriladi.

### Rollar (TZ VI–VIII)
Yuqori panelda rol tanlanadi. Rol ekranni **yashirmaydi** — nimani o'zgartirish
mumkinligini belgilaydi, chunki yashirilgan ma'lumot ishonchni yo'qotadi.

| Rol | Nimani o'zgartiradi |
|---|---|
| Administrator | Hammasini |
| Prorab | Jurnal, tabel, texnika smenalari, grafik, arizalar, ombor, xavfsizlik |
| Texnik nazorat | Ijro hujjatlari, sifat, xavfsizlik, tekshiruv natijalari, PPR, smeta |
| Sotuv | Kvartiralar, shartnomalar, to'lovlar |
| Buyurtmachi | Hech narsani — faqat ko'radi |

O'zgartirish mumkin bo'lmagan ekranda tepada sariq tasma chiqadi. Foydalanuvchilar
sozlamalarda qo'shiladi. **Bu parol bilan himoya emas**: baza fayli ochiq turibdi,
shuning uchun rol — ish taqsimoti vositasi. Haqiqiy kirish nazorati server qismi
bilan keladi va sozlamalarda shu ochiq yozilgan.

### Loyiha paketi — qurilmalar orasida almashish
Server yo'q, shuning uchun ma'lumot **fayl orqali** ko'chadi. Sozlamalardagi
«Paketga chiqarish» maydonchada to'ldirilgan **kunlik ijroni** (jurnal, tabel,
texnika smenalari, sifat, xavfsizlik) matn fayliga yozadi; «Paketdan olish» uni
boshqa kompyuterdagi bazaga qo'shadi.

Grafik, smeta va shartnomalar paketga **kirmaydi** — ular ofisda yuritiladi va
ikki tomondan tahrirlansa ziddiyat tug'iladi. Import qo'shadi, mavjud yozuvlarni
qayta yozmaydi: ikki marta olib kirilsa dublikat bo'lmaydi. Paket odam o'qiy
oladigan matn — nima ko'chganini ko'zdan kechirish mumkin.

### AI analitika (TZ XVII)
Barcha modullardan yig'ilgan bitta ko'rinish. Yuqorida **sog'lomlik indeksi**:
100 dan topilmalar og'irligi ayriladi (kritik −15, jiddiy −8, ogohlantirish −4) —
hisob ochiq yozilgan, chunki yopiq ball ishonchsiz.

Sakkiz yo'nalish (muddat, pul, hujjatlar, ta'minot, sifat, xavfsizlik, resurslar,
sotuv) bo'yicha ko'rsatkichlar — har biri bosilsa o'z ekranini ochadi. Pastda
24 qoida bo'yicha topilmalar, muhimligi bo'yicha tartiblangan. Har topilma uch
qismga ajratilgan: **fakt** (nima kuzatildi), **hisob** (qaysi sonlardan chiqdi)
va **tavsiya** (nima qilish mumkin) — TZ III.32 ga muvofiq dastur hukm
chiqarmaydi. Har qoidaning barqaror kodi bor (`AN-S2`, `AN-P5`…), shuning uchun
hisobotlarni ishga tushirishlar orasida solishtirish mumkin.

«Hisobotni saqlash» ko'rsatkichlar va topilmalarni matn fayliga chiqaradi.

### Prorab ish o'rni (TZ VI)
Bir kunlik ekran: bugun ketayotgan ishlar (bajarilishni shu yerda o'zgartirish
mumkin), kunlik jurnal yozuvi, brigada soatlari («butun brigadaga smena» bir
bosishda), texnika smenalari va diqqat talab qiladigan holatlar — ochiq
xavfsizlik yozuvlari va zaxiradan kam materiallar. Kunni yopish uchun boshqa
bo'limlarga o'tish shart emas.

### Texnik nazorat kabineti (TZ VII)
Ko'rib chiqish navbati: imzo kutayotgan ijro hujjatlari (imzolash va rad etish
tugmalari bilan), talabga mos kelmagan sifat yozuvlari, yopilmagan jiddiy
nomuvofiqliklar va muddati o'tgan xavfsizlik choralari.

### Buyurtmachi kabineti (TZ VIII)
Faqat o'qish uchun ko'rinish: bajarilish va prognoz, shartnoma muddati, bo'limlar
kesimidagi bajarilish, moliya (shartnoma, smeta, bajarilgan ish qiymati,
to'langan va to'lanmagan qism), oxirgi ish kunlari va sotuv holati.

### Yordamchi (TZ XVIII)
Ekranda ikki xil javob bor va ular ataylab **ajratilgan**: «Savol-javob» tabida
javob shu bazadagi ma'lumotdan hisoblanadi, «AI suhbat» tabida esa OpenAI
modeli javob beradi (quyida). 13 ta tayyor savol bor; erkin
yozilgan savol kalit so'zlar bo'yicha shulardan biriga bog'lanadi. Aniq mavzuli
savol umumiy so'zlardan ustun turadi — «ombor holati qanday?» ombor haqidagi
savol deb tushuniladi. Savol tanilmasa javob **o'ylab topilmaydi**: ekran buni
ochiq aytadi. Har javobda «Tekshirish» tugmasi manba ekranini ochadi.

### OpenAI integratsiyasi (TZ XVIII)

Yordamchidagi **«AI suhbat»** tabi OpenAI modeliga ulanadi. Suhbat ko'p
bosqichli: oldingi savol-javoblardan oxirgi sakkiztasi so'rovga qo'shiladi,
shuning uchun «unda nima qilay?» degan savol ham tushuniladi.

**Sukut bo'yicha o'chiq.** Sozlamada yoqilib API kalit kiritilmaguncha ilova
hech qayerga ulanmaydi. Yoqilganda savol va unga biriktirilgan **hisoblangan
sonlar** tashqi xizmatga jo'natiladi — sozlamalarda bu ochiq ogohlantirish
bilan yozilgan. Tarmoq kodi umuman kerak bo'lmasa:

```
cargo build --release --no-default-features
```

shunda `llm` moduli kompilyatsiyaga ham kirmaydi va sozlamada buning sababi
yozib qo'yiladi.

**Model son hisoblamaydi.** Har so'rovga obyekt bo'yicha tayyor sonlar —
yordamchining o'z javoblari — biriktiriladi va modeldan faqat shularga tayanish
so'raladi. Ko'rsatma qat'iy: ma'lumotda yo'q sonni yoki normativni o'ylab
topmaslik, bilmagan narsani ochiq aytish, aybdorni belgilamaslik. Model javobi
alohida ramkada chiqadi — u ilovaning hisobi emasligi ko'rinib tursin.

**Kalit hech qayerga chiqmaydi.** U bazada saqlanadi, ekranda yopiq
ko'rsatiladi (`••••abcd`) va faqat `Authorization` sarlavhasida boradi: so'rov
tanasiga ham, xato matniga ham tushmaydi. Zaxira nusxada esa u ham bo'ladi —
bu sozlamalarda ochiq yozilgan.

**So'rov interfeysni to'xtatmaydi.** U alohida oqimda ketadi, ekranda kutish
belgisi turadi, javob esa qaysi bo'limda bo'lishingizdan qat'i nazar olinadi.
Xato bo'lsa sababi aniq aytiladi — kalit qabul qilinmadimi, limit tugadimi,
tarmoq yo'qmi — va qaytadan urinish ma'noli bo'lgan holatlarda tugma chiqadi.

| Sozlama | Sukut qiymati |
|---|---|
| Manzil | `https://api.openai.com/v1/chat/completions` |
| Model | `gpt-4o-mini` (ro'yxatdan tanlanadi yoki qo'lda yoziladi) |
| Kutish muddati | 45 soniya (5–180) |
| Kontekst chegarasi | 12 000 belgi |
| Suhbat tarixi | oxirgi 8 gap |

Sozlamada **«Ulanishni sinash»** tugmasi bitta qisqa savol jo'natadi: kalit va
model to'g'riligi birinchi haqiqiy savoldan oldin ma'lum bo'ladi.

### Umumiy ko'rinish
Obyekt bo'yicha xulosa, muddati o'tgan ishlar mas'ullari va kechikish miqdori bilan
(bosilsa GPR da ochiladi), bo'limlar kesimidagi bajarilish, bugungi ishlar.

### Sozlamalar
Interfeys tili, mavzu, interfeys masshtabi (80–160 %), GPR standart masshtabi,
dam olish kunlarini ajratish, standart valyuta va yangi ish davomiyligi,
baza fayli yo'li va papkani ochish, namoyish obyektini yaratish, dastur haqida ma'lumot.

## Tuzilishi

| Fayl | Vazifasi |
|---|---|
| `src/i18n.rs` | Ikki tilli satrlar ro'yxati, joriy til |
| `src/theme.rs` | Yorug' va qorong'i palitra |
| `src/model.rs` | I modul turlari: obyekt, ishtirokchilar, ishlar, bog'lanishlar, bo'limlar |
| `src/domain.rs` | II–XX modullar turlari: nomuvofiqlik, element, smeta, jurnal, ombor, kvartira, shartnoma… |
| `src/cpm.rs` | Tarmoq grafigi hisobi: CPM, zaxiralar, plan/fakt, prognoz |
| `src/checks.rs` | Tekshiruv dvigateli: loyiha (II), smeta (III) va PPR (I.3) qoidalari, normativ reyestri |
| `src/import.rs` | Smeta importi: XLSX/XLS/ODS/CSV, ustunlarni nom bo'yicha aniqlash |
| `src/db.rs` | SQLite: asosiy sxema, so'rovlar, sozlamalar, namoyish ma'lumoti |
| `src/store.rs` | II–XVI modullar sxemasi va CRUD |
| `src/app.rs` | Ilova holati, sozlamalar, UI va omborni bog'lash |
| `src/ui/gantt.rs` | GPR ekrani: jadval, Gantt, drag & drop, inspektor |
| `src/ui/passport.rs` | Obyekt pasporti ekrani |
| `src/ui/dashboard.rs` | Umumiy ko'rinish ekrani |
| `src/ui/ppr.rs` | PPR va texnologik kartalar, resurs yetarliligi |
| `src/ui/aicheck.rs` | AI loyiha tekshiruvi: nomuvofiqliklar, elementlar, bog'lanishlar, normativlar |
| `src/ui/estimate.rs` | AI smeta tekshiruvi: import, pozitsiyalar va natijalar |
| `src/ui/execdocs.rs` | Ijro hujjatlari va rasmiylashtirilmagan ishlar |
| `src/ui/journal.rs` | Kundalik ishlar jurnali |
| `src/ui/documents.rs` | Obyekt hujjatlari va foto galereyasi |
| `src/ui/timesheet.rs` | Tabel: haftalik soat jadvali va ish haqi |
| `src/ui/quality.rs` | Sifat nazorati: uch bosqich va nuqson muddati |
| `src/ui/safety.rs` | Mehnat xavfsizligi hodisalari jurnali |
| `src/ui/machines.rs` | Texnika parki va smenalar |
| `src/ifc.rs` | IFC (ISO 10303-21) o'qish va bilimlar grafiga o'girish |
| `src/roles.rs` | Rollar va ekran bo'yicha yozish huquqi |
| `src/package.rs` | Qurilmalar orasida almashish paketi |
| `src/llm.rs` | OpenAI integratsiyasi (sukut bo'yicha o'chiq) |
| `src/analytics.rs` | Kesishgan tahlil: 24 qoida, sog'lomlik indeksi, hisobot |
| `src/copilot.rs` | Savol-javob: niyatni tanish va javob hisobi |
| `src/backup.rs` | Bazaning izchil zaxira nusxasi |
| `src/docgen.rs` | KS-2, KS-3, M-29, AOSR va jadvallarni `.xlsx` ga yozish |
| `src/pdf.rs` | Ekran jadvalini PDF ga chiqarish (A4 albom, sahifalash) |
| `src/actions.rs` | Yordamchining qoralama amallari va ularni bajarish |
| `src/dxf.rs` | DXF chizmasini o'qish (CAD ning ochiq formati) |
| `src/pdfread.rs` | PDF dagi jadvalni matn joylashuvidan tiklash |
| `src/ocr.rs` | Skan uchun mahalliy Tesseract ko'prigi va sertifikat maydonlari |
| `src/sync.rs` | Server bilan sinxronizatsiya (sukut bo'yicha o'chiq) |
| `server/` | Server: kirish nazorati, paket navbati, imzo, mobil ko'rinish |
| `src/ui/analytics.rs` | AI analitika ekrani |
| `src/ui/copilot.rs` | Yordamchi ekrani |
| `src/ui/foreman.rs` | Prorab ish o'rni |
| `src/ui/supervision.rs` | Texnik nazorat kabineti |
| `src/ui/client.rs` | Buyurtmachi kabineti |
| `src/sales.rs` | Sotuv hisobi: qarz, to'lov grafigi, kvartira holati |
| `src/ui/sales.rs` | Sotuv shaxmatkasi va kvartira kartochkasi |
| `src/ui/deals.rs` | Shartnomalar, to'lov turlari va to'lov grafigi |
| `src/ui/requests.rs` | Arizalar, ehtiyoj va qoplanish nazorati |
| `src/ui/purchases.rs` | Xaridlar, yetkazish va omborga kirim |
| `src/ui/materials.rs` | Material katalogi, sertifikat nazorati |
| `src/ui/warehouse.rs` | Ombor: qoldiqlar va harakatlar |
| `src/ui/issues.rs` | Nomuvofiqliklar uchun umumiy jadval va kartochka |
| `src/ui/settings.rs` | Sozlamalar ekrani |
| `src/ui/mod.rs` | Mavzu, navigatsiya, dialoglar, umumiy vidjetlar |

## Ombor

SQLite (`rusqlite`, WAL rejimi), to'liq lokal — server kerak emas.
Sxema kelajakdagi sinxronizatsiyaga tayyorlangan: har bir yozuvda `updated_at` bor,
identifikatorlar qayta ishlatilmaydi. Obyekt o'chirilganda bog'liq ma'lumot
`FOREIGN KEY … ON DELETE CASCADE` bilan ketadi.

## Ishlab chiqarishga tayyorlash

### Yig'ish

```bash
cargo build --release
```

Natija ikkita fayl:

| Fayl | Nima |
|---|---|
| `target/release/qurai.exe` | Desktop ilova. Tashqi bog'liqliksiz ishlaydi |
| `target/release/qurai-server.exe` | Server (ixtiyoriy): kirish nazorati, sinxronizatsiya, telefon ko'rinishi |

Tarmoq kodisiz yig'ish (ilova hech qayerga ulanmasligi kafolatlansin):

```bash
cargo build --release --no-default-features
```

### Tarqatish va birinchi ishga tushirish

Ilovani ko'chirish uchun **bitta fayl** yetarli. U birinchi ochilishda yonidagi
`data/qurai.db` faylini yaratadi va namoyish obyektini to'ldiradi. Namuna kerak
bo'lmasa — «Sozlamalar → Namunani o'chirish».

Papkaga yozish huquqi bo'lmasa (masalan `Program Files`), baza foydalanuvchi
papkasiga tushadi. Aniq yo'l «Sozlamalar → Baza fayli» da ko'rinadi.

### Ma'lumotni saqlash

- **Zaxira nusxa** — «Sozlamalar → Zaxira nusxa». Nusxa `VACUUM INTO` orqali
  olinadi: WAL rejimida ham hech narsa tushib qolmaydi. **Har kuni oling** —
  kod git bilan qaytadi, ma'lumot qaytmaydi.
- **Yangilash** — eski baza ustiga yangi versiyani qo'yish yetarli: sxema
  o'zgarishi qo'shimcha ustunlar orqali qilinadi va eski baza ochilaveradi
  (sinov bilan qoplangan).
- **Ko'chirish** — `data/qurai.db` faylini nusxalash kifoya. Boshqa yo'l kerak
  bo'lsa: `QURAI_DB=D:\qurai\baza.db qurai.exe`.

### Server (ixtiyoriy)

Server kerak bo'ladi, agar: bir necha qurilma bir obyektda ishlasa, telefondan
kunlik yozuv kiritilsa yoki hujjat masofadan imzolansa.

```bash
qurai-server --add-user prorab "Ism Familiya" foreman
QURAI_BIND=0.0.0.0:8080 QURAI_DB=/var/qurai/server.db qurai-server
```

Ilovada: «Sozlamalar → Server bilan sinxronizatsiya» — manzil, login va parol.
Parol saqlanmaydi, faqat seans belgisi.

**Xavfsizlik chegarasi ochiq aytiladi:**

- Server **HTTP** beradi. Internetga chiqarilganda **HTTPS beruvchi teskari
  proksi ortida** turishi shart (nginx, Caddy) — aks holda parol va seans
  belgisi tarmoqda ochiq ketadi. Dastur buni ishga tushirishda ham eslatadi.
- Ilova ichidagi rol — **ish taqsimoti**, himoya emas: baza fayli ochiq va uni
  har kim o'qiy oladi. Haqiqiy kirish nazorati serverda: parol Argon2id bilan
  xeshlanadi, seans belgisi tasodifiy, huquq har so'rovda tekshiriladi.
- Til modeli (OpenAI) **sukut bo'yicha o'chiq**. Yoqilganda savol va unga
  biriktirilgan sonlar tashqi xizmatga ketadi — buni foydalanuvchi o'zi
  tanlaydi. Kalit faqat `Authorization` sarlavhasida ketadi va logga tushmaydi.

### Tekshiruv

Har o'zgarishdan keyin:

```bash
cargo test --workspace
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Sinovlar orasida **haqiqiy ishni** tekshiradiganlari bor: har ekran oynasiz
chiziladi (namuna, bo'sh baza, faqat-o'qish roli, tor oyna va chekka ma'lumot),
server haqiqiy so'rovlarni qabul qiladi, desktop mijozi haqiqiy server bilan
gaplashadi, PDF va Excel fayllari haqiqatda yoziladi va qayta o'qiladi.

## Ilova ichida hal bo'lmaydigan narsalar

Bular kod bilan emas, tashqi shart bilan hal bo'ladi va shu sababli ochiq
qoldirilgan:

1. **DWG va RVT** — yopiq formatlar. CAD dan **DXF** eksport qilinadi va u
   o'qiladi; IFC ham o'qiladi.
2. **3D ko'rinish (BIM)** — geometriya yadrosi talab qiladi. IFC dan elementlar,
   bog'lanishlar va kolliziyalar ro'yxati olinadi.
3. **Push bildirishnoma** (SMS, Telegram) — tashqi xizmat va shartnoma kerak.
   Ilova ichidagi bildirishnomalar markazi va telefon sahifasi ishlaydi.
4. **Davlat elektron raqamli imzosi** — kalitlar va akkreditatsiya masalasi.
   Serverdagi imzo kim, qachon va qaysi matnni tasdiqlaganini qayd etadi.
5. **Ovozli kiritish** — mikrofon va nutqni tanish xizmati kerak.
6. **Native mobil ilova** — hozir telefon brauzeri orqali ishlanadi.
7. **Skanni tanish (OCR)** — mahalliy Tesseract chaqiriladi; u o'rnatilmagan
   bo'lsa ilova buni ochiq aytadi va taxmin qilmaydi.
