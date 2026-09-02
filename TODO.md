# QURAi — TZ bo'yicha to'liq talablar ro'yxati

TZ hujjatidagi **har bir raqamlangan talab** shu yerda. Belgilar:

- `[x]` — **bajarilgan**, ishlaydi va sinovdan o'tgan
- `[~]` — **qisman**: asosi bor, TZ dagi to'liq talab emas
- `[ ]` — **yo'q**
- 🔒 — tashqi narsa kerak (server, mobil klient, LLM, OCR, BIM geometriya, tashqi ma'lumot bazasi)

Har bir qatordagi *kursiv* izoh — nima bor va nima yetishmayotgani.

---

## Umumiy hisob

| Modul | Talab | ✅ | 🟡 | ⬜ | shundan 🔒 |
|---|---:|---:|---:|---:|---:|
| I. Loyihani boshqarish | 3 | 3 | 0 | 0 | 0 |
| II. AI loyiha tekshiruvi | 19 | 8 | 7 | 4 | 0 |
| III. AI smeta tekshiruvi | 34 | 8 | 8 | 18 | 2 |
| IV. Ijro hujjatlari | 30 | 10 | 10 | 10 | 3 |
| V. Kunlik jurnal | 34 | 17 | 7 | 10 | 6 |
| VI. Prorab (mobil) | 37 | 16 | 9 | 12 | 8 |
| VII. Texnik nazorat | 38 | 13 | 10 | 15 | 3 |
| VIII. Buyurtmachi | 37 | 10 | 13 | 14 | 3 |
| IX. Arizalar | 42 | 21 | 8 | 13 | 1 |
| X. Xaridlar | 48 | 16 | 16 | 16 | 2 |
| XI. Ombor | 48 | 26 | 9 | 13 | 1 |
| XII. Materiallar | 41 | 24 | 9 | 8 | 2 |
| XIII. Tabel | 44 | 18 | 11 | 15 | 6 |
| XIV. Sifat | 41 | 19 | 9 | 13 | 2 |
| XV. Xavfsizlik | 41 | 21 | 8 | 12 | 3 |
| XVI. Mashinalar | 50 | 28 | 7 | 15 | 3 |
| XVII. AI analitika | 51 | 22 | 12 | 17 | 0 |
| XVIII. AI Copilot | 45 | 15 | 23 | 7 | 7 |
| **Jami** | **683** | **295** | **176** | **212** | **52** |

Ya'ni **~23 % to'liq**, **~16 % qisman**, **~61 % hali yo'q**.

Asosiy sabab: qurilgan qism — har modulning **yadrosi** (ma'lumot, ekran, hisob).
Yetishmayotgani — asosan **AI tahlili, tashqi manbalar, mobil klient, hujjat
generatsiyasi va modullararo avtomatik zanjirlar**.

---

## I. Loyihani boshqarish

- [x] 1. Obyekt pasporti — *karta, bosqichlar, moliya, ishtirokchilar, hujjat va foto*
- [x] 2. GPR — *CPM, Gantt, drag&drop, kritik yo'l, plan/fakt, prognoz*
- [x] 3. PPR — *kartalar, resurs gistogrammasi, qoplanish, 6 qoida*

---

## II. AI loyiha tekshiruvi

- [x] 1. Umumiy vazifa — *bilimlar grafi va qoidalar dvigateli*
- [x] 2. Ishlash tamoyili — *fakt/hisob/xulosa ajratilgan (III.32)*
- [~] 3. **AR — arxitektura** — *xona, eshik, deraza, devor turlari bor; TZ dagi barcha AR tekshiruvlari emas*
- [~] 4. **KJ — temir-beton** — *ustun, rigel, plita, teshik; armatura va yuklama tekshiruvi yo'q*
- [~] 5. **KM — metall** — *rigel va ustun; tugun va payvand tekshiruvi yo'q*
- [~] 6. **VK — suv va kanalizatsiya** — *truba, uklon, kesish; diametr hisobi yo'q*
- [~] 7. **OV — isitish va ventilyatsiya** — *vozduxovod, qurilma; havo hisobi yo'q*
- [~] 8. **EOM — elektrika** — *kabel, qurilma, quvvat; kesim hisobi yo'q*
- [ ] 9. **SS — kuchsiz tok** — *bo'lim bor, tekshiruv qoidalari yo'q*
- [ ] 10. **PB — yong'in xavfsizligi** — *bo'lim bor, qoidalar yo'q*
- [x] 11. **CROSS CHECK** — *bo'limlararo tekshiruv: AR↔KJ, VK↔KJ, EOM↔OV va h.k.*
- [ ] 12. Spetsifikatsiyalarni tekshirish — *spetsifikatsiya modeli yo'q*
- [x] 13. Hajmlarni tekshirish — *element o'lchamlaridan hisob*
- [ ] 14. Qurilish amalga oshirilishini tekshirish — *texnologik imkoniyat tahlili yo'q*
- [x] 15. Yakuniy hisobot — *bo'limlar kesimida muhimlik bo'yicha*
- [x] 16. Har xatoning kartochkasi — *kod, bo'lim, element, joy, varaq, me'yor, tavsiya, mas'ul*
- [x] 17. AI ga majburiy talab — *normativni o'ylab topmaydi (II.17)*
- [x] 18. Arxitektura — *bilimlar grafi tugun va qirralar bilan*
- [~] 19. Interfeys: PROYEKT / AI CHECK / CLASH / ACTION — *4 tabdan 3 tasi; CLASH alohida emas*
- [x] 🔒 **IFC dan o'qish** — *ochiq format o'qiladi; DWG/RVT uchun kutubxona kerak*
- [ ] 🔒 PDF va DWG dan chizmani tanish
- [ ] 🔒 3D geometriya bo'yicha haqiqiy kolliziya (clash) hisobi

---

## III. AI smeta tekshiruvi

- [x] 1. Modul maqsadi
- [~] 2. Hujjat yuklash — *XLSX/XLS/ODS/CSV; PDF va skan yo'q*
- [x] 3. Smeta tuzilmasini tanish — *ustunlarni nom bo'yicha aniqlash, ikki tilda*
- [x] 4. Arifmetikani tekshirish
- [x] 5. O'lchov birliklarini tekshirish
- [x] 6. Hajmlarni tekshirish — *loyiha bilan solishtirish*
- [~] 7. Loyiha bo'yicha hajmni avtomatik hisoblash — *elementdan oddiy hisob; to'liq emas*
- [x] 8. Dublikatlarni tekshirish
- [ ] 9. Kompleks rasrenkalarni tekshirish
- [~] 10. Tushib qolgan ishlarni tekshirish — *loyihada bor, smetada yo'q holati*
- [ ] 11. Texnologik ketma-ketlikni tekshirish
- [~] 12. Materiallarni tekshirish — *nom bo'yicha; katalog bilan bog'lanish yo'q*
- [ ] 13. Marka va xarakteristikani tekshirish
- [~] 14. Narxlarni tekshirish — *bir xil ish uchun narx farqi topiladi*
- [ ] 🔒 14.1–14.4. Tarixiy narx, taklif narxi, bozor diapazoni bilan solishtirish
- [ ] 🔒 15. **Narxlar bazasi (PRICE DATABASE)**
- [ ] 16. Tijorat takliflari (KP)
- [ ] 17. Koeffitsiyentlarni tekshirish
- [ ] 18. Ustama xarajatlarni tekshirish
- [ ] 19. Foydani tekshirish
- [ ] 20. QQS ni tekshirish
- [ ] 21. Bir necha smeta variantini solishtirish
- [~] 22. Smetani shartnoma bilan solishtirish — *analitikada AN-C4 qoidasi*
- [ ] 23. Smetani byudjet bilan solishtirish
- [ ] 24. Smeta → Xaridlar (avtomatik)
- [ ] 25. Smeta → Ombor (avtomatik)
- [ ] 26. Smeta → GPR (avtomatik bog'lanish)
- [ ] 27. Smeta → Prorabning kunlik hisoboti
- [x] 28. Ortiqcha sarfni nazorat qilish — *sarf normalari bilan*
- [ ] 29. AI yakuniy qiymat prognozi
- [~] 30. Tejashni izlash — *eng past narxga keltirish hisobi*
- [~] 31. AI-smetachining bosh ekrani — *KPI bor, to'liq COST CONTROL paneli yo'q*
- [x] 32. Eng muhim talab — *hukm chiqarmaslik (III.32)*
- [ ] 33. Yakuniy arxitektura: smeta → arizalar → xaridlar → ombor → fakt → foyda

---

## IV. Ijro hujjatlari

- [x] 1. Modul maqsadi
- [x] 2. Ijro hujjati kartochkasi — *tur, raqam, nom, sana, ish, holat, mas'ul*
- [x] 3. Kerakli hujjatlar avtomatik ro'yxati — *bo'lim bo'yicha reyestr*
- [~] 4. **AI Document Matrix** — *talablar jadvali bor; to'liq matritsa emas*
- [x] 5. Hujjatlarni avtomatik yaratish — *KS-2, KS-3, M-29, AOSR bazadan*
- [x] 6. **AOSR** — *ish, ishlatilgan material, sertifikat, imzo joylari*
- [ ] 7. Ijro sxemalari
- [~] 8. Fotolar — *jurnalda foto bor; hujjatga biriktirish yo'q*
- [ ] 🔒 9. Geolokatsiya va vaqt
- [~] 10. Material sertifikatlari — *katalogda sertifikat va muddati*
- [ ] 🔒 11. AI sertifikat tekshiruvi (OCR)
- [ ] 12. Beton pasporti
- [ ] 13. Laboratoriya sinovlari
- [~] 14. Jurnallar — *umumiy ishlar jurnali bor; maxsus jurnallar yo'q*
- [x] 15. Prorabning kunlik hisoboti
- [~] 16. Yashirin ishlar nazorati — *tur sifatida bor; bloklash yo'q*
- [~] 17. Kelishuv workflow — *holatlar bor; marshrut yo'q*
- [ ] 🔒 18. Elektron imzo
- [ ] 19. Versiyalilik
- [ ] 20. Imzolashdan oldin AI tekshiruvi
- [~] 21. Loyiha bilan solishtirish
- [~] 22. Haqiqiy qurilish bilan solishtirish — *AN-D1 qoidasi*
- [x] 23. Texnik nazorat kabineti
- [ ] 24. Mualliflik nazorati kabineti
- [x] 25. Buyurtmachi kabineti
- [~] 26. To'liqlikni AI nazorati — *rasmiylashtirilmagan ishlar topiladi*
- [ ] 27. Obyekt arxivi
- [~] 28. AI orqali qidiruv — *umumiy qidiruv (Ctrl+K)*
- [x] 29. Boshqa modullar bilan bog'lanish
- [x] 30. Modulning bosh funksiyasi

---

## V. Kunlik ishlar jurnali

- [x] 1. Modul maqsadi
- [~] 2. Kim ishlaydi — *rollar bor; har rol uchun alohida ko'rinish yo'q*
- [x] 3. Kunlik hisobot kartochkasi
- [x] 4. Ob-havo sharoiti — *qo'lda kiritiladi*
- [x] 5. Odamlar
- [x] 6. Tabel — *alohida modul, bog'langan*
- [x] 7. Bajarilgan ishlar
- [x] 8. Fotofiksatsiya
- [ ] 🔒 9. Fotolarni AI tahlili
- [~] 10. Materiallar — *ombor orqali; jurnalda to'g'ridan-to'g'ri emas*
- [~] 11. Material sarfini nazorat — *ombor chiqimi bor; norma bilan solishtirish yo'q*
- [x] 12. Texnika
- [ ] 🔒 13. GPS bilan avtomatik bog'lanish
- [x] 14. GPR bajarilishi — *jurnal hajmi ishga o'tadi*
- [x] 15. Kechikish tahlili — *AN-S2, AN-S3, AN-S4*
- [ ] 16. Ertangi kunga reja
- [ ] 17. Avtomatik arizalar — *zaxira bo'yicha ariza bor, jurnaldan emas*
- [~] 18. Muammolar — *xavfsizlik va sifat modullarida*
- [x] 19. Kritik muammolarni AI aniqlashi — *analitika topilmalari*
- [x] 20. Texnik nazorat izohlari
- [~] 21. Yashirin ishlar nazorati
- [x] 22. Sifat nazorati
- [x] 23. Mehnat muhofazasi
- [ ] 24. Direktorning kunlik hisoboti
- [ ] 🔒 25. AI prorabga savol beradi
- [ ] 🔒 26. Ovozli rejim
- [ ] 🔒 27. Geolokatsiya nazorati
- [ ] 🔒 28. Hisobotni imzolash
- [x] 29. Tarix
- [~] 30. AI qidiruv
- [x] 31. Boshqa modullar bilan bog'lanish
- [ ] 32. Soxta hisobotlardan himoya
- [~] 33. Kunning yakuniy holati — *prorab ekranida*
- [x] 34. Asosiy g'oya

---

## VI. Prorab ilovasi

> 🔒 TZ da bu **mobil ilova**. Hozir desktopdagi «Prorab ish o'rni» sifatida
> qurilgan: bir kunlik ekran, hamma narsa bir joyda. Mobil klient alohida
> loyiha va server qismini talab qiladi.

- [x] 1. Ilova maqsadi — *desktop varianti*
- [~] 2. Avtorizatsiya — *rol tanlash; parol yo'q*
- [x] 3. Bosh ekran
- [x] 4. Mening vazifalarim — *bugungi ishlar*
- [~] 5. Mobil GPR — *desktop GPR bor*
- [x] 6. Kunlik hisobot
- [ ] 🔒 7. Ovozli kiritish
- [~] 8. Foto — *jurnalda*
- [ ] 🔒 9. Fotoni AI tahlili
- [ ] 🔒 10. Video
- [ ] 🔒 11. QR-kod
- [~] 12. Chizmalar bilan ishlash — *hujjat sifatida biriktiriladi*
- [~] 13. BIM bilan ishlash — *IFC o'qiladi; 3D ko'rinish yo'q*
- [x] 14. Bajarilgan ishlar
- [x] 15. Materiallar
- [x] 16. Materialga ariza
- [~] 17. AI ariza taklifi — *zaxira bo'yicha avtomatik ariza*
- [x] 18. Ishchilar hisobi
- [ ] 🔒 19. Geolokatsiya
- [x] 20. Texnika
- [ ] 21. Texnika buzilishi
- [~] 22. Izohlar — *xavfsizlik va sifat orqali*
- [x] 23. Texnik nazorat izohlari
- [x] 24. Ijro hujjatlari
- [ ] 25. Ishni yopishdan oldin ogohlantirish
- [x] 26. Texnika xavfsizligi
- [x] 27. Instruktaj
- [ ] 🔒 28. Internetsiz ishlash — *lokal baza; sinxronizatsiya paket orqali*
- [ ] 🔒 29. Bildirishnomalar
- [~] 30. AI-yordamchi — *qoidalarga asoslangan yordamchi*
- [x] 31. AI aniq obyektni bilishi
- [ ] 🔒 32. Ofis bilan chat
- [ ] 33. Ish kunining avtomatik yakunlanishi
- [ ] 34. Hisobotni yuborishdan oldin AI tekshiruvi
- [x] 35. Prorabning bosh paneli
- [~] 36. Platforma bilan bog'lanish — *fayl orqali paket*
- [x] 37. Ishlab chiqishning bosh tamoyili

---

## VII. Texnik nazorat kabineti

- [x] 1. Modul maqsadi
- [x] 2. Bosh ekran — *ko'rib chiqish navbati*
- [ ] 3. Tekshiruvlar kalendari
- [ ] 4. Tekshiruvga arizalar
- [~] 5. Yashirin ishlar tekshiruvi
- [ ] 6. Jismoniy tekshiruv
- [ ] 🔒 7. AR / vizual tekshiruv
- [~] 8. Loyihaga muvofiqlikni tekshirish — *AI tekshiruvi orqali*
- [x] 9. Materiallarni tekshirish
- [x] 10. Kirish nazorati
- [ ] 🔒 11. AI sertifikat tekshiruvi
- [ ] 12. Betonni tekshirish
- [~] 13. Hajmlarni nazorat qilish
- [ ] 14. Geodeziyani tekshirish
- [x] 15. Izohlar — *nomuvofiqliklar*
- [x] 16. Izoh toifalari
- [~] 17. AI izohni tasniflashi — *muhimlik darajasi*
- [ ] 18. Chizmadagi izoh
- [x] 19. Bartaraf etishni nazorat qilish
- [ ] 20. «Oldin/keyin» fotosi
- [x] 21. Muddatlarni nazorat qilish
- [~] 22. Izohlar jurnali
- [x] 23. Ijro hujjatlari
- [x] 24. AOSR ni tekshirish — *imzolash/rad etish*
- [x] 25. Sifat nazorati
- [ ] 26. AI chek-list yaratadi
- [~] 27. PPR nazorati
- [~] 28. Ishlar ketma-ketligini nazorat qilish
- [~] 29. Loyihani nazorat qilish
- [ ] 30. Versiyalarni nazorat qilish
- [ ] 🔒 31. Texnik nazorat uchun AI Clash
- [ ] 32. Qurilishdagi o'zgarishlarni nazorat qilish
- [~] 33. AI-yordamchi
- [ ] 34. Texnik nazoratning kunlik hisoboti
- [~] 35. Obyekt tayyorligini nazorat qilish
- [ ] 36. Yakuniy qabul
- [x] 37. Boshqa modullar bilan bog'lanish
- [x] 38. Eng muhim funksiya

---

## VIII. Buyurtmachi kabineti

- [x] 1. Asosiy maqsad
- [x] 2. Buyurtmachining bosh paneli
- [x] 3. Obyekt holati
- [~] 4. Obyekt foto va videosi — *jurnal fotolari*
- [~] 5. Obyekt tarixi — *oxirgi ish kunlari*
- [ ] 🔒 6. 3D / BIM
- [~] 7. Ishlar grafigi — *bo'limlar kesimida bajarilish*
- [x] 8. Grafikni AI tahlili
- [x] 9. Moliya
- [~] 10. AI Cost Control
- [ ] 11. Qiymat o'zgarishlari
- [ ] 12. Qo'shimcha ishlarni nazorat qilish
- [~] 13. Smeta
- [ ] 14. Xaridlar
- [ ] 15. Takliflarni solishtirish
- [x] 16. Texnik nazorat kabineti
- [~] 17. Izohlar
- [ ] 18. Buyurtmachi izoh yarata olishi
- [~] 19. Ijro hujjatlari
- [~] 20. Ijro hujjatlarini AI tekshiruvi
- [ ] 21. Ishlarni qabul qilish
- [ ] 🔒 22. Elektron kelishuv
- [~] 23. Loyiha hujjatlari
- [ ] 24. Loyiha versiyalarini AI solishtirishi
- [~] 25. Loyiha xatolarini nazorat qilish
- [x] 26. Xavfsizlik
- [ ] 27. Shartnomalar
- [ ] 28. AI Contract Monitor
- [~] 29. To'lovlar — *pasportda to'langan summa*
- [ ] 30. AI Payment Control
- [~] 31. Risklar — *analitika topilmalari*
- [~] 32. Buyurtmachining AI-yordamchisi
- [ ] 33. Haftalik hisobot
- [x] 34. Kirish darajalari — *rollar*
- [ ] 🔒 35. Mobil kabinet
- [x] 36. Kabinetning eng muhim funksiyasi
- [x] 37. Asosiy tamoyil

---

## IX. Arizalar

- [x] 1. Modul maqsadi
- [x] 2. Kim ariza yarata oladi — *rollar*
- [~] 3. Ariza turlari — *material, texnika, ishchi kuchi, hujjat, boshqa*
- [x] 4. Materialga ariza
- [x] 5. AI o'zi ariza taklif qilishi — *zaxira bo'yicha*
- [x] 6. Ariza muddatni hisobga olishi
- [x] 7. Shoshilinchlik
- [x] 8. Kelishuv marshruti — *bosqichlar tartib bilan, rol bo'yicha*
- [x] 9. Avtomatik limitlar — *summa marshrut uzunligini belgilaydi*
- [x] 10. Byudjetni tekshirish — *bo'lim byudjeti bilan*
- [~] 11. Dublikatni tekshirish — *ochiq ariza bo'lsa takrorlamaydi*
- [ ] 12. Smetani tekshirish
- [ ] 13. Loyihaga muvofiqlikni tekshirish
- [ ] 14. Materialni almashtirish
- [x] 15. Tijorat takliflari
- [x] 16. AI yetkazib beruvchilarni solishtirishi
- [x] 17. Ariza → xarid
- [x] 18. Ariza → buyurtma — *tanlangan taklifdan xarid*
- [x] 19. Ariza → ombor
- [x] 20. Qisman yetkazish
- [x] 21. Muddati o'tgan arizalarni nazorat qilish
- [~] 22. Texnikaga ariza — *tur bor, jarayon yo'q*
- [ ] 23. Transportga ariza
- [ ] 24. Ta'mirga ariza
- [ ] 25. Pulga ariza
- [ ] 26. Xizmatga ariza
- [ ] 27. Xodimga ariza
- [ ] 28. Foto va hujjatlar
- [ ] 🔒 29. Ovozli arizalar
- [x] 30. Ijroni nazorat qilish — *qoplanish*
- [x] 31. Ariza tarixi — *kim, qachon, qanday qaror qildi*
- [x] 32. Rad etish sababi — *sababsiz rad ogohlantiriladi*
- [~] 33. Arizalarni AI tahlili
- [~] 34. Rahbar paneli
- [~] 35. AI-panel
- [ ] 36. GPR bilan bog'lanish
- [x] 37. Ombor bilan bog'lanish
- [ ] 38. Smeta bilan bog'lanish
- [ ] 39. Buxgalteriya bilan bog'lanish
- [~] 40. Yakuniy nazorat
- [x] 41. Modulning bosh ekrani
- [~] 42. Tizim prediktiv bo'lishi

---

## X. Xaridlar

- [x] 1. Modul maqsadi
- [x] 2. Xaridlarning bosh ekrani
- [x] 3. Har xaridning manbai — *arizaga bog'lanadi*
- [ ] 4. AI xaridlarni rejalashtirishi
- [~] 5. Ehtiyojni avtomatik hisoblash — *zaxiradan kam bo'yicha*
- [~] 6. Xariddan oldin tekshirish
- [~] 7. Yetkazib beruvchilarni izlash — *kartochka va ro'yxat*
- [x] 8. Yetkazib beruvchi tarixi — *xaridlardan hisoblanadi*
- [~] 9. Tijorat taklifini so'rash — *taklif yozuvi*
- [x] 10. KP qabul qilish
- [x] 11. AI KP larni solishtirishi — *narx va muddat bo'yicha*
- [~] 12. AI eng yaxshi variantni tanlashi — *eng arzoni va eng tezi belgilanadi, tanlov odamniki*
- [x] 13. Narx anomaliyasi — *katalogdan 20% farq*
- [~] 14. Narx o'zgarishini nazorat qilish — *takliflar taqqoslanadi*
- [x] 15. Muqobil yetkazib beruvchi izlash — *bir arizaga bir necha taklif*
- [~] 16. Yetkazib beruvchini tekshirish — *STIR, taqiq belgisi*
- [~] 17. Materialni tekshirish
- [ ] 18. Texnik kelishuv
- [ ] 19. Materialni almashtirish
- [~] 20. Buyurtma shakllantirish — *xarid yozuvi*
- [ ] 21. Shartnoma
- [ ] 22. Yetkazib beruvchi shartnomasini AI tekshiruvi
- [~] 23. To'lovni nazorat qilish — *holat bor, to'lov grafigi yo'q*
- [x] 24. Yetkazishni nazorat qilish
- [~] 25. Obyektda qabul qilish
- [~] 26. Kirish nazorati — *sifat moduli orqali*
- [ ] 🔒 27. AI sertifikatni tekshiradi
- [ ] 28. Qabulda foto
- [x] 29. Ombor bilan bog'lanish — *bir bosishda kirim*
- [x] 30. Qisman yetkazish — *kelgan miqdor, qoldiq, kirim*
- [x] 31. Kechikishlarni nazorat qilish
- [ ] 32. GPR bilan bog'lanish
- [ ] 33. Obyektlar bo'yicha xaridlarni nazorat qilish
- [x] 34. Bo'limlar bo'yicha xaridlarni nazorat qilish
- [x] 35. Xarid byudjetini nazorat qilish — *reja / buyurtma / qoldiq*
- [~] 36. AI ortiqcha sarfni aniqlashi
- [ ] 37. Markazlashtirilgan xaridlar
- [ ] 38. Turli obyektlar xaridlarini solishtirish
- [~] 39. Sarfni nazorat qilish
- [x] 40. Yetkazib beruvchilar tahlili — *muddatida %, o'rtacha kechikish*
- [ ] 41. Korrupsiya/manfaatlar to'qnashuvi riskini nazorat qilish
- [ ] 42. Xaridlarni avtomatik bo'lish
- [ ] 43. Shoshilinch xaridlar
- [ ] 🔒 44. AI narx prognozi
- [~] 45. Direktorning bosh hisoboti
- [ ] 46. Xaridchi samaradorligini AI tahlili
- [~] 47. Modulning to'liq zanjiri
- [x] 48. Modulning eng kuchli funksiyasi — *xarid → ombor kirimi*

---

## XI. Ombor

- [x] 1. Modul maqsadi
- [x] 2. Omborning bosh ekrani
- [x] 3. **Bir necha ombor** — *tur, mas'ul, ombor kesimida qoldiq*
- [x] 4. Material kartochkasi
- [ ] 🔒 5. QR / shtrix-kod
- [x] 6. Materialni qabul qilish
- [~] 7. Kirish nazorati — *sifat moduli orqali*
- [ ] 8. AI material tekshiruvi
- [x] 9. **Partiyalar** — *raqam, kelgan sana, yetkazib beruvchi, qoldiq*
- [x] 10. Sertifikatlar — *partiyada; muddati o'tgani qizil*
- [x] 11. Material berish
- [x] 12. Aniq ish bo'yicha berish
- [ ] 13. Smeta bilan bog'lanish
- [x] 14. Ortiqcha sarfni nazorat qilish — *«Normativ / fakt», ruxsat foizi*
- [x] 15. Normativ sarf — *bajarilgan hajmga qarab*
- [x] 16. Real vaqtdagi qoldiqlar
- [x] 17. **Rezervlash** — *erkin qoldiq = qoldiq − rezerv*
- [x] 18. Kamomadni avtomatik aniqlash
- [x] 19. Avtomatik ariza yaratish
- [x] 20. Omborlar orasida ko'chirish — *bitta hujjat, ikki yozuv*
- [x] 21. **Qaytarish** — *MoveKind::Return, qoldiqni oshiradi*
- [x] 22. Hisobdan chiqarish
- [~] 23. Nazoratsiz hisobdan chiqarishni taqiqlash — *sababsizlari ogohlantiriladi*
- [x] 24. **Inventarizatsiya** — *hisob/fakt, yopilgach o'zgarmas*
- [x] 25. Farqlar — *tuzatuvchi harakatga aylanadi*
- [ ] 26. AI kamomad sababini izlashi
- [x] 27. Yaroqlilik muddati — *partiyada*
- [x] 28. FIFO / FEFO — *navbatdagi partiya belgilanadi*
- [ ] 29. Harorat nazorati
- [~] 30. **YoMM** — *texnika modulida yoqilg'i*
- [ ] 31. AI YoMM nazorati
- [ ] 32. **Asboblar**
- [ ] 33. Asbob berish
- [ ] 34. Asbobni nazorat qilish
- [ ] 35. **Ish kiyimi va SIZ**
- [x] 36. Xaridlar bilan bog'lanish
- [ ] 37. Buxgalteriya bilan bog'lanish
- [x] 38. Materialning o'rtacha qiymati — *vaznlangan o'rtacha*
- [~] 39. Narxlarni nazorat qilish
- [~] 40. Rahbar paneli
- [x] 41. Uzoq turgan materiallar — *90 kun harakatsiz → sariq*
- [ ] 42. Obyektlar orasida qayta taqsimlash
- [~] 43. Nolikvidlarni nazorat qilish — *uzoq turganlar orqali*
- [ ] 44. Ombor fotosi
- [~] 45. Omborchining AI-yordamchisi
- [~] 46. Direktorning AI-yordamchisi
- [~] 47. Omborning to'liq bog'lanishi
- [x] 48. Modulning eng muhim funksiyasi — *qoldiq harakatlardan hisoblanadi*

---

## XII. Materiallar

- [x] 1. Modul maqsadi
- [x] 2. Yagona material katalogi
- [x] 3. Material kartochkasi
- [x] 4. Texnik xarakteristikalar
- [x] 5. Material ↔ loyiha — *spetsifikatsiya havolasi*
- [x] 6. Material ↔ spetsifikatsiya
- [x] 7. Material ↔ smeta — *rasenka kodi*
- [ ] 8. AI materialning loyihaga mosligini tekshirishi
- [x] 9. Analoglar — *tasdiqlangan almashtiruvchi*
- [x] 10. Materiallarni solishtirish — *narx farqi va qoldiq*
- [~] 11. Almashtiruvchi tanlash — *narx, qoldiq va tasdiq holati ko'rsatiladi*
- [x] 12. Juda muhim qoida — *bir material — bitta kartochka*
- [x] 13. Sertifikatlar
- [ ] 🔒 14. AI OCR sertifikatlar
- [ ] 15. AI sertifikatni tekshiradi
- [x] 16. Hujjat amal qilish muddati
- [x] 17. Material → partiya — *ombor partiyalari*
- [~] 18. Material → yetkazib beruvchi — *xarid orqali*
- [x] 19. Narx tarixi — *kirimlardan, o'zgarish foizi bilan*
- [ ] 🔒 20. Bozor narxi
- [x] 21. Material → sarf normalari
- [~] 22. AI ortiqcha sarfni tahlil qilishi — *farq va summa hisoblanadi*
- [~] 23. Obyektlar bo'yicha materiallar
- [ ] 24. Materiallarni qayta taqsimlash
- [~] 25. Tez orada kerak bo'ladigan materiallar — *analitikada*
- [~] 26. AI oldindan ogohlantirishi — *zaxiradan kam*
- [~] 27. GPR bo'yicha materiallar — *ish tayyorligi ko'rinishi*
- [x] 28. Ish boshlanishidan oldin mavjudlikni nazorat qilish — *ikki hafta oldin*
- [x] 29. Material → ijro hujjati — *ish orqali*
- [x] 30. To'liq kuzatuvchanlik — *yetkazuvchi → partiya → ish → hujjat*
- [x] 31. Maxsus talabli materiallar — *katalogda alohida maydon*
- [ ] 32. AI moslikni tekshirishi
- [ ] 33. Material komplekti
- [ ] 34. Ishlab chiqaruvchilarni solishtirish
- [x] 35. Materiallar reytingi — *yetkazishlar, narx o'zgarishi, kirish nazorati*
- [x] 36. **Brak** — *rad etilgan, hisobdan chiqarilgan, qaytarilgan*
- [x] 37. Yetkazib beruvchiga qaytarish — *alohida harakat turi*
- [x] 38. Taqiqlangan materiallar — *sababsiz taqiq ogohlantiriladi*
- [~] 39. AI-yordamchi
- [~] 40. Direktor paneli
- [x] 41. Modulning eng muhim funksiyasi

---

## XIII. Tabel

- [x] 1. Modul maqsadi
- [x] 2. Bosh ekran — *haftalik jadval*
- [ ] 3. **Obyektlar** — *bir necha obyekt bo'yicha*
- [ ] 🔒 4. Kirish/chiqishni hisobga olish
- [ ] 🔒 5. Obyekt geozonasi
- [ ] 🔒 6. QR-kirish
- [~] 7. Prorabning mobil tabeli — *desktop ekranida*
- [x] 8. **Brigadalar** — *brigadir, ish, ishchilar tarkibi*
- [~] 9. Ishchilarni obyektlar bo'yicha taqsimlash — *ishchi obyektga tegishli*
- [x] 10. Ish vaqtini ishlar bo'yicha taqsimlash — *katakda «Ish» rejimi*
- [x] 11. **Smenalar** — *kunduzgi / kechki / tungi, koeffitsiyent bilan*
- [ ] 12. Ish grafigi
- [x] 13. Ortiqcha ish — *8 soatdan oshgani ×1.5*
- [x] 14. Tungi soatlar — *smena bo'yicha, ×1.5*
- [~] 15. Dam olish / bayram kuni — *sarlavhada ajratiladi*
- [x] 16. **Yo'qliklar** — *kun turi: ta'til, kasallik, safar, sababsiz*
- [x] 17. Ta'tillar
- [x] 18. Kasallik varaqasi
- [x] 19. Xizmat safari — *to'lanadi*
- [ ] 20. AI anomaliyalarni nazorat qilishi
- [ ] 21. Prorabning ish vaqtini nazorat qilish
- [x] 22. **Bo'sh turishlar hisobi** — *to'lanadi, ishlangan soat emas*
- [~] 23. Bo'sh turishlar tahlili — *brigada bo'yicha ulush*
- [~] 24. **Unumdorlik** — *soatning tannarxi*
- [x] 25. Brigadalarni solishtirish
- [ ] 26. Xodimlar sonini rejalashtirish
- [ ] 27. AI xodim ehtiyoji prognozi
- [ ] 28. Xodimlarni ko'chirish
- [x] 29. Tabel → ish haqi
- [x] 30. Tabel → tannarx
- [x] 31. Aniq ishning tannarxi — *ish haqi + material, bir birlikka*
- [~] 32. Ish haqi fondini nazorat qilish
- [~] 33. Buxgalteriya uchun tabel — *soat, yo'qlik, ish haqi jadvali*
- [~] 34. Tabelni tuzatish
- [ ] 35. Oyni yopish
- [ ] 36. Tasdiqlash
- [ ] 🔒 37. Elektron imzo
- [ ] 🔒 38. Mobil ilova
- [ ] 🔒 39. Ovozli kiritish
- [~] 40. AI-yordamchi
- [~] 41. Rahbar nazorati
- [~] 42. Bosh hisobot
- [x] 43. Boshqa modullar bilan bog'lanish
- [x] 44. Modulning eng kuchli funksiyasi

---

## XIV. Sifat

- [x] 1. Modul maqsadi
- [x] 2. Bosh ekran
- [x] 3. **Uch daraja nazorat** — *kirish, operatsion, qabul*
- [x] 4. Materiallarning kirish nazorati
- [~] 5. AI material tekshiruvi — *kirish nazorati chek-listi*
- [ ] 6. Brakka chiqarilgan materialni ishlatishni taqiqlash
- [x] 7. Operatsion nazorat
- [x] 8. Chek-listlar — *namuna, normativ havolasi, tekshiruvga ko'chiriladi*
- [x] 9. Yashirin ishlar nazorati — *chek-list bandi sifatida*
- [x] 10. Keyingi bosqichni bloklash — *ochiq nuqson ishni yopishga qo'ymaydi*
- [~] 11. Sifat fotofiksatsiyasi
- [ ] 🔒 12. Fotolarni AI tahlili
- [ ] 13. Geometriya nazorati
- [ ] 🔒 14. BIM Quality Control
- [~] 15. Bo'limlarni nazorat qilish
- [~] 16. Kolliziyalarni tekshirish — *AI tekshiruvida*
- [x] 17. Izohlar
- [x] 18. Izohlar tasnifi
- [~] 19. AI ustuvorlikni aniqlashi
- [x] 20. Izohni bartaraf etish
- [ ] 21. Oldin / keyin fotosi
- [ ] 22. Laboratoriya sinovlari
- [ ] 23. Beton nazorati
- [ ] 24. Payvand nazorati
- [ ] 25. Muhandislik tizimlari sinovlari
- [~] 26. PPR nazorati
- [~] 27. Texnologik ketma-ketlik nazorati
- [ ] 28. Pudratchilar bo'yicha sifat nazorati
- [ ] 29. Pudratchilar sifat reytingi
- [x] 30. Brak sabablarini tahlil qilish
- [x] 31. Takrorlanuvchi nuqsonlar — *bir xil sabab guruhlanadi*
- [ ] 32. Prediktiv sifat nazorati
- [x] 33. **Quality Score** — *0-100, ochiq va muddati o'tgan nuqsonlar jarimasi*
- [x] 34. Bosqich tayyorligini nazorat qilish
- [x] 35. Bosqichni yopishni taqiqlash — *ogohlantirish, taqiq emas*
- [x] 36. Kunlik sifat nazorati
- [ ] 37. Haftalik hisobot
- [~] 38. AI-yordamchi
- [~] 39. Direktor kabineti
- [x] 40. Boshqa modullar bilan bog'lanish
- [x] 41. Modulning bosh funksiyasi

---

## XV. Xavfsizlik

- [x] 1. Modul maqsadi
- [x] 2. Bosh ekran
- [x] 3. **Safety Dashboard** — *ball, hodisa, chora, ruxsat*
- [x] 4. Ishchining ruxsati — *tur bo'yicha, muddat bilan*
- [x] 5. **Ruxsatlar matritsasi** — *ishchi × ruxsat turi*
- [x] 6. Instruktajlar
- [x] 7. Avtomatik eslatmalar — *30 kun qolganda sariq*
- [x] 8. **SIZ** — *majburiy to'plam, xizmat muddati*
- [x] 9. SIZ nazorati — *berilmagani va muddati o'tgani qizil*
- [x] 10. Yuqori xavfli ishlar — *sakkiz tur*
- [x] 11. **Naryad-dopusk** — *ish, muddat, odamlar, chora-tadbirlar*
- [x] 12. Naryadni tekshirish — *ruxsat, SIZ, muddat, mas'ul*
- [x] 13. Balandlikdagi ishlar — *ruxsat turi va naryad*
- [ ] 🔒 14. AI foto-nazorat
- [ ] 15. Xavfli zonalarni nazorat qilish
- [~] 16. Yuk ko'tarish ishlari — *ruxsat turi bor*
- [~] 17. Texnika — *texnik ko'rik muddati*
- [ ] 18. Texnikaning kunlik ko'rigi
- [~] 19. Elektr xavfsizligi — *ruxsat turi bor*
- [x] 20. O't ishlari — *ruxsat turi va naryad*
- [~] 21. Yer ishlari — *ruxsat turi bor*
- [ ] 22. Yong'in xavfsizligi
- [ ] 23. Evakuatsiya rejasi
- [ ] 24. Favqulodda vaziyatlar
- [x] 25. Hodisa haqida xabar
- [x] 26. **Near Miss**
- [~] 27. AI hodisalarni tahlil qilishi
- [ ] 28. **Root Cause Analysis**
- [x] 29. Korrektiv chora-tadbirlar
- [x] 30. Bartaraf etishni nazorat qilish
- [ ] 🔒 31. Bartaraf etilgandan keyin AI foto tekshiruvi
- [~] 32. Kunlik Safety Report
- [x] 33. **Safety Score** — *hodisa, ruxsat, SIZ va naryad kamchiliklari*
- [ ] 34. Obyektlar reytingi
- [ ] 35. Pudratchilar reytingi
- [ ] 36. **AI Risk Prediction**
- [ ] 🔒 37. Ob-havo bilan bog'lanish
- [~] 38. AI-yordamchi
- [~] 39. Rahbar kabineti
- [x] 40. Boshqa modullar bilan bog'lanish
- [x] 41. Modulning bosh funksiyasi

---

## XVI. Mashinalar

- [x] 1. Modul maqsadi
- [x] 2. Bosh ekran
- [x] 3. Mashina kartochkasi
- [x] 4. Har mashina uchun saqlanadigan ma'lumot
- [x] 5. Texnika toifalari
- [ ] 🔒 6. Texnika qayerda — *GPS*
- [x] 7. Holatlar
- [~] 8. Texnikaga ariza — *ariza turi bor*
- [ ] 9. AI texnikani taqsimlashi
- [ ] 10. GPR bilan bog'lanish
- [ ] 11. Texnikani rejalashtirish
- [ ] 12. To'qnashuvlarning oldini olish
- [x] 13. **Motosoat**
- [x] 14. Yurgan masofa — *spidometr farqidan*
- [x] 15. Motosoat nazorati — *foydalanish koeffitsiyenti*
- [x] 16. **Yoqilg'i**
- [x] 17. Sarf normasi — *litr/motosoat*
- [x] 18. Yoqilg'i tahlili — *fakt / norma, 10% dan oshgani qizil*
- [x] 19. **Yo'l varaqalari** — *raqam, haydovchi, marshrut, spidometr*
- [x] 20. Mashinaga topshiriq — *smena ishga bog'lanadi*
- [~] 21. Bajarilishni nazorat qilish — *soat, masofa, reys*
- [x] 22. Tashishni nazorat qilish — *reys soni va yuk*
- [~] 23. **Ta'mir** — *holat sifatida*
- [ ] 24. Ta'mirga ariza
- [x] 25. Rejali TX — *motosoat oralig'i, qolgani*
- [ ] 26. AI ta'mir prognozi
- [x] 27. Ekspluatatsiyani taqiqlash — *ko'rik, TX, ta'mir*
- [ ] 28. Kunlik ko'rik
- [x] 29. Xavfsizlik bilan bog'lanish — *AN-R1 qoidasi*
- [x] 30. **Operator**
- [ ] 31. AI operatorni tekshirishi
- [x] 32. Ijaraga olingan texnika — *belgisi bor*
- [~] 33. Ijara va o'z texnikasini solishtirish — *soatning qiymati*
- [x] 34. **Mashina-soat qiymati**
- [x] 35. Ishning tannarxi — *tabel tannarxiga kiradi*
- [x] 36. Texnika unumdorligi — *soat, masofa, reys*
- [x] 37. Texnika bo'sh turishi — *bo'sh kunlar soni*
- [x] 38. Foydalanish koeffitsiyenti — *ish kunlariga nisbatan*
- [ ] 39. AI parkni optimallashtirishi
- [ ] 🔒 40. Texnika xaritasi
- [x] 41. Mashina tarixi — *yo'l varaqalari*
- [ ] 42. AI samaradorlik tahlili
- [ ] 43. «Ta'mirlash yoki almashtirish» qarori
- [~] 44. Mashina hujjatlari — *texnik ko'rik*
- [ ] 🔒 45. Bildirishnomalar
- [ ] 46. Mexanik kabineti
- [~] 47. Direktor kabineti
- [~] 48. XVI ning bosh zanjiri
- [x] 49. Boshqa modullar bilan bog'lanish
- [x] 50. Eng kuchli funksiya

---

## XVII. AI analitika

- [x] 1. Asosiy vazifa
- [x] 2. Direktorning bosh ekrani
- [~] 3. **AI 5 ta savolga javob berishi** — *«nima bo'lyapti», «nima qilish kerak» bor; «nega», «qancha turadi», «keyin nima bo'ladi» qisman*
- [ ] 4. Obyektlar bo'yicha analitika — *bir necha obyekt*
- [x] 5. Plan-fakt
- [x] 6. GPR tahlili
- [~] 7. AI kechikish sababini aniqlashi
- [x] 8. Obyekt tugash prognozi
- [x] 9. Kritik yo'l
- [~] 10. Risklar prognozi
- [x] 11. Moliyaviy analitika
- [x] 12. Tannarx tahlili — *ish haqi, material, texnika*
- [~] 13. Yakuniy tannarx prognozi — *pul oqimi rejasi*
- [ ] 14. Foyda prognozi
- [x] 15. Materiallar tahlili
- [~] 16. Xaridlar tahlili
- [~] 17. Tejashni izlash
- [x] 18. Ombor tahlili
- [x] 19. Xodimlar tahlili
- [x] 20. Texnika tahlili
- [x] 21. Sifat tahlili
- [x] 22. Xavfsizlik tahlili
- [ ] 23. Pudratchilar tahlili
- [ ] 24. Yetkazib beruvchilar tahlili
- [ ] 25. Korrelyatsion tahlil
- [ ] 26. Unumdorlik tahlili
- [ ] 27. **Benchmarking**
- [~] 28. Loyihalar tahlili
- [~] 29. Smeta tahlili
- [x] 30. **Pul oqimlari tahlili** — *oylar kesimida fakt va reja*
- [x] 31. **Kassa uzilishini rejalashtirish** — *birinchi manfiy oy va summa*
- [ ] 32. Debitorlik qarzi tahlili
- [ ] 33. AI tushum prognozi
- [ ] 34. Shartnomalar tahlili
- [ ] 35. Loyiha o'zgarishlari tahlili
- [ ] 36. **«Nima bo'ladi, agar?»** — *ssenariy*
- [x] 37. **Kunlik xulosa** — *faqat bugungi muddat, yetkazish, hodisa*
- [ ] 38. **AI Weekly Management Report**
- [~] 39. AI sonlarni tushuntirishi — *fakt/hisob/tavsiya*
- [x] 40. Ogohlantirishlar tizimi
- [ ] 41. AI imkoniyatlarni topishi
- [~] 42. **Qarorlar markazi**
- [ ] 43. Ssenariy modellashtirish
- [ ] 44. AI yashirin yo'qotishlarni izlashi
- [x] 45. Chetlanishlar tahlili
- [x] 46. Yagona AI chat — *o'n uch savol, har biri manba ekraniga bog'langan*
- [~] 47. Darajalar bo'yicha analitika — *ko'rsatkich → bo'lim*
- [x] 48. **Drill-down** — *kartochka, topilma va oqim qatoridan bo'limga*
- [ ] 49. **AI Executive Score** — *sog'lomlik indeksi qisman*
- [~] 50. XVII ning bosh arxitekturasi
- [x] 51. Eng muhim funksiya

---

## XVIII. AI Copilot

> 🔒 TZ ga ko'ra Copilot — **butun platformaning operatsion qatlami**: tabiiy
> tilni tushunadi, ma'lumot topadi, tahlil qiladi, tushuntiradi, yechim taklif
> qiladi va **tasdiqlangandan keyin amalni bajaradi**. Hozir qurilgani —
> qoidalarga asoslangan savol-javob va ixtiyoriy LLM ulanish nuqtasi.

- [x] 1. Asosiy vazifa — *qisman: savol-javob*
- [x] 2. Bosh ekran
- [~] 3. Tabiiy nutqni tushunish — *kalit so'zlar; LLM ixtiyoriy*
- [ ] 🔒 4–10. Ovoz, kontekst, ko'p bosqichli suhbat
- [x] 11. Rol bo'yicha kirish chegarasi
- [~] 12–20. Qoralamalar — *ariza, xarid, ijro hujjati, kelishuv marshruti*
- [x] 21–30. Amalni bajarish — *tasdiqdan keyin, rol huquqi bilan*
- [~] 31–40. Hodisa → tavsiya → nazorat — *taklif bajarilgach ro'yxatdan chiqadi*
- [~] 41. Ma'lumot manbasini ko'rsatish — *«Tekshirish» tugmasi*
- [x] 42. Sonni o'ylab topmaslik
- [~] 43. Modullar bo'ylab kirish — *13 mavzu*
- [~] 44. Copilot — operatsion qatlam — *yetti xil amal; til modeli yo'q*
- [x] 45. TZ uchun asosiy ta'rif — *chegara ochiq yozilgan*

---

## Umumiy (TZ da alohida modul emas, lekin kerak)

- [x] Ikki til (uz/ru), yorug'/qorong'i mavzu, interfeys masshtabi
- [x] Umumiy qidiruv (Ctrl+K)
- [x] Rollar va yozish huquqi
- [x] Zaxira nusxa (`VACUUM INTO`)
- [x] Qurilmalar orasida paket almashish
- [x] Baza migratsiyasi (eski baza ochilaveradi)
- [ ] 🔒 **Bir necha obyekt bo'yicha konsolidatsiya** — *hozir bitta obyekt kesimida*
- [ ] 🔒 **Server va jonli sinxronizatsiya**
- [ ] 🔒 **Mobil klient**
- [x] **Hujjat generatsiyasi** — *KS-2, KS-3, M-29, AOSR — `.xlsx` shaklida*
- [~] **Excel eksporti** — *15 ta ekran jadvali, Ctrl+E; PDF hali yo'q*
- [ ] **Bildirishnomalar tizimi**
- [x] **Amallar tarixi (audit log)** — *har bir qo'shish, o'zgartirish va o'chirish; 50 000 yozuv saqlanadi*
- [ ] **Izoh va muhokama** (yozuvlarga sharh)

---

## Keyingi qadam uchun tavsiya etilgan tartib

Eng ko'p foyda beradigan va tashqi narsa talab qilmaydigan ishlar:

1. ~~Ombor: partiyalar, rezervlash, inventarizatsiya, qaytarish, bir necha ombor~~ (XI.3, 9, 17, 20–21, 24–28) — **bajarildi**
2. ~~Normativ sarf va ortiqcha sarf nazorati~~ (XI.14–15, XII.21–22, III.28) — **bajarildi**
3. ~~Tabel: brigadalar, smenalar, yo'qliklar, bo'sh turish, tannarx~~ (XIII.8–31) — **bajarildi**
4. ~~Xaridlar: KP, yetkazib beruvchilar, qisman yetkazish, byudjet~~ (X.7–16, 30, 34–35, 40) — **bajarildi**
5. ~~Arizalar: kelishuv marshruti, limitlar, tarix, rad sababi~~ (IX.8–10, 31–32) — **bajarildi**
6. ~~Sifat: chek-listlar, bosqich bloklash, Quality Score, brak tahlili~~ (XIV.8–10, 30–35) — **bajarildi**
7. ~~Xavfsizlik: naryad-dopusk, SIZ, ruxsatlar matritsasi, Safety Score~~ (XV.4–13, 20, 33) — **bajarildi**
8. ~~Mashinalar: yo'l varaqalari, TX rejasi, tannarx, foydalanish koeffitsiyenti~~ (XVI.14–27, 32–38) — **bajarildi**
9. ~~Analitika: pul oqimi, kassa uzilishi, kunlik xulosa, drill-down~~ (XVII.30–31, 37, 48) — **bajarildi**
10. **Hujjat generatsiyasi va Excel eksporti** (IV.5–7, umumiy)

### To'qqiz bosqich bitgach — keyingi navbat

Birinchi to'qqiz band bajarildi. Endi eng ko'p foyda beradigan qolgan ishlar:

1. ~~Hujjat generatsiyasi: KS-2, KS-3, M-29, yashirin ishlar dalolatnomasi~~
   (IV.5–6) — **bajarildi**. Qoldi: inventarizatsiya ro'yxati va
   naryad-dopusk blankasi (XI.25, XV.11), ijro sxemasi (IV.7)
2. ~~Excel eksporti: har bir jadval uchun~~ — **bajarildi** (15 ekran, Ctrl+E).
   Qoldi: PDF ga chiqarish
3. **Bir nechta obyekt kesimida analitika** (XVII.4, X.33, XV.34)
4. **Smeta ↔ material ↔ ish bog'lanishi**: pozitsiya darajasida (III.28, XII.5–7)
5. **Debitorlik va to'lov intizomi** (XVII.32–33, XX)
6. **Pudratchilar va yetkazib beruvchilar reytingi** (XIV.28–29, XV.35, XVII.23–24)
7. **Ssenariy modellashtirish «nima bo'ladi, agar?»** (XVII.36, 43)

Server, DWG/RVT va til modeli bo'yicha ishlar `USTA.md` da alohida yozilgan —
ular ilovadan tashqaridagi qarorlarni talab qiladi.
