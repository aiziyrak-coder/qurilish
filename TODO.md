# QURAi — TZ bo'yicha to'liq talablar ro'yxati

TZ hujjatidagi **har bir raqamlangan talab** shu yerda. Belgilar:

- `[x]` — **bajarilgan**, ishlaydi va sinovdan o'tgan
- `[~]` — **qisman**: asosi bor, TZ dagi to'liq talab emas
- `[ ]` — **yo'q**
- 🔒 — tashqi narsa kerak (server, mobil klient, LLM, OCR, BIM geometriya, tashqi ma'lumot bazasi)

Har bir qatordagi *kursiv* izoh — nima bor va nima yetishmayotgani.

---

## Umumiy hisob

> Jadval qo'lda yuritilmaydi: `python tools/todo_count.py` uni shu
> fayldagi belgilardan qayta hisoblaydi.

| Modul | Talab | ✅ | 🟡 | ⬜ | shundan 🔒 |
|---|---:|---:|---:|---:|---:|
| I. Loyihani boshqarish | 3 | 3 | 0 | 0 | 0 |
| II. AI loyiha tekshiruvi | 22 | 11 | 7 | 4 | 2 |
| III. AI smeta tekshiruvi | 34 | 22 | 6 | 6 | 2 |
| IV. Ijro hujjatlari | 30 | 16 | 8 | 6 | 3 |
| V. Kunlik ishlar jurnali | 34 | 22 | 4 | 8 | 6 |
| VI. Prorab ilovasi | 37 | 21 | 9 | 7 | 7 |
| VII. Texnik nazorat kabineti | 38 | 26 | 6 | 6 | 3 |
| VIII. Buyurtmachi kabineti | 37 | 23 | 11 | 3 | 2 |
| IX. Arizalar | 42 | 32 | 6 | 4 | 1 |
| X. Xaridlar | 48 | 33 | 13 | 2 | 2 |
| XI. Ombor | 48 | 32 | 8 | 8 | 1 |
| XII. Materiallar | 41 | 31 | 8 | 2 | 2 |
| XIII. Tabel | 44 | 26 | 9 | 9 | 6 |
| XIV. Sifat | 41 | 30 | 8 | 3 | 2 |
| XV. Xavfsizlik | 41 | 33 | 4 | 4 | 3 |
| XVI. Mashinalar | 50 | 35 | 9 | 6 | 2 |
| XVII. AI analitika | 51 | 34 | 11 | 6 | 0 |
| XVIII. AI Copilot | 45 | 17 | 28 | 0 | 0 |
| Umumiy (TZ dan tashqari) | 14 | 11 | 1 | 2 | 2 |
| **Jami** | **700** | **458** | **156** | **86** | **46** |

Ya'ni **~65 % to'liq**, **~22 % qisman**, **~12 % hali yo'q**.

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
- [x] 9. **SS — kuchsiz tok** — *kabel va o'rnatish joyi tekshiriladi*
- [x] 10. **PB — yong'in xavfsizligi** — *himoya, chiqish va suv ta'minoti*
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
- [x] 17. Koeffitsiyentlarni tekshirish — *ko'rsatilmagani va haddan tashqarisi*
- [x] 18. Ustama xarajatlarni tekshirish — *to'g'ridan-to'g'ri xarajatdan foizda*
- [x] 19. Foydani tekshirish — *ustama bilan birga summadan*
- [x] 20. QQS ni tekshirish — *yakuniy summadan*
- [x] 21. Bir necha smeta variantini solishtirish — *bo'lim kesimida*
- [x] 22. Smetani shartnoma bilan solishtirish — *analitika qoidasi va prognoz tabidagi amaldagi summa*
- [x] 23. Smetani byudjet bilan solishtirish — *bo'lim bo'yicha*
- [x] 24. Smeta → Xaridlar — *zanjirda ariza va xarid summasi ish kesimida*
- [x] 25. Smeta → Ombor — *kirim va ishga berilgan material zanjirda*
- [x] 26. Smeta → GPR — *pozitsiya ishga bog'lanadi, qoplanish foizi*
- [x] 27. Smeta → fakt — *bajarilish foizi va haqiqiy tannarx zanjirda*
- [x] 28. Ortiqcha sarfni nazorat qilish — *sarf normalari bilan*
- [x] 29. Yakuniy qiymat prognozi — *analitikadagi «Prognoz» tabi*
- [x] 30. Tejashni izlash — *eng past narx hisobi va analitikadagi «Yo'qotishlar» tabi*
- [~] 31. AI-smetachining bosh ekrani — *KPI bor, to'liq COST CONTROL paneli yo'q*
- [x] 32. Eng muhim talab — *hukm chiqarmaslik (III.32)*
- [x] 33. Yakuniy arxitektura — *«Zanjir» tabi: reja → ariza → xarid → ombor → fakt → farq; uzilish belgilanadi*

---

## IV. Ijro hujjatlari

- [x] 1. Modul maqsadi
- [x] 2. Ijro hujjati kartochkasi — *tur, raqam, nom, sana, ish, holat, mas'ul*
- [x] 3. Kerakli hujjatlar avtomatik ro'yxati — *bo'lim bo'yicha reyestr*
- [~] 4. **AI Document Matrix** — *talablar jadvali bor; to'liq matritsa emas*
- [x] 5. Hujjatlarni avtomatik yaratish — *KS-2, KS-3, M-29, AOSR bazadan*
- [x] 6. **AOSR** — *ish, ishlatilgan material, sertifikat, imzo joylari*
- [ ] 7. Ijro sxemalari
- [x] 8. Fotolar — *hujjatga biriktiriladi*
- [ ] 🔒 9. Geolokatsiya va vaqt
- [~] 10. Material sertifikatlari — *katalogda sertifikat va muddati*
- [ ] 🔒 11. AI sertifikat tekshiruvi (OCR)
- [x] 12. Beton pasporti
- [x] 13. Laboratoriya sinovlari
- [~] 14. Jurnallar — *umumiy ishlar jurnali bor; maxsus jurnallar yo'q*
- [x] 15. Prorabning kunlik hisoboti
- [x] 16. Yashirin ishlar nazorati
- [~] 17. Kelishuv workflow — *holatlar bor; marshrut yo'q*
- [ ] 🔒 18. Elektron imzo
- [x] 19. Versiyalilik
- [x] 20. Imzolashdan oldin AI tekshiruvi
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
- [x] 10. Materiallar — *kunlik hajm bo'yicha sarf*
- [x] 11. Material sarfini nazorat — *kun ichida norma bilan solishtiriladi*
- [x] 12. Texnika
- [ ] 🔒 13. GPS bilan avtomatik bog'lanish
- [x] 14. GPR bajarilishi — *jurnal hajmi ishga o'tadi*
- [x] 15. Kechikish tahlili — *AN-S2, AN-S3, AN-S4*
- [x] 16. Ertangi kunga reja
- [ ] 17. Avtomatik arizalar — *zaxira bo'yicha ariza bor, jurnaldan emas*
- [x] 18. Muammolar — *xavfsizlik, sifat va yozuvlarga izoh*
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
- [x] 32. Soxta hisobotlardan himoya — *ichki ziddiyatlar*
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
- [x] 21. Texnika buzilishi
- [x] 22. Izohlar — *har qanday yozuvga izoh va javob*
- [x] 23. Texnik nazorat izohlari
- [x] 24. Ijro hujjatlari
- [x] 25. Ishni yopishdan oldin ogohlantirish
- [x] 26. Texnika xavfsizligi
- [x] 27. Instruktaj
- [ ] 🔒 28. Internetsiz ishlash — *lokal baza; sinxronizatsiya paket orqali*
- [~] 29. Bildirishnomalar — *ilova ichida bildirishnomalar markazi; push tashqi omil*
- [~] 30. AI-yordamchi — *qoidalarga asoslangan yordamchi*
- [x] 31. AI aniq obyektni bilishi
- [ ] 🔒 32. Ofis bilan chat
- [x] 33. Ish kunining avtomatik yakunlanishi — *yakuniy tekshiruv ro'yxati*
- [x] 34. Hisobotni yuborishdan oldin AI tekshiruvi
- [x] 35. Prorabning bosh paneli
- [~] 36. Platforma bilan bog'lanish — *fayl orqali paket*
- [x] 37. Ishlab chiqishning bosh tamoyili

---

## VII. Texnik nazorat kabineti

- [x] 1. Modul maqsadi
- [x] 2. Bosh ekran — *ko'rib chiqish navbati*
- [x] 3. Tekshiruvlar kalendari — *o'tkazilmagan tekshiruvlar kunlar bo'yicha, muddati o'tgani qizil*
- [x] 4. Tekshiruvga arizalar — *kim chaqirdi, qachonga rejalashtirilgan*
- [x] 5. Yashirin ishlar tekshiruvi — *tekshiruv turi, natija, bartaraf etish muddati*
- [x] 6. Jismoniy tekshiruv — *tekshiruv turlaridan biri*
- [ ] 🔒 7. AR / vizual tekshiruv
- [~] 8. Loyihaga muvofiqlikni tekshirish — *AI tekshiruvi orqali*
- [x] 9. Materiallarni tekshirish
- [x] 10. Kirish nazorati
- [ ] 🔒 11. AI sertifikat tekshiruvi
- [x] 12. Betonni tekshirish — *7 va 28 kunlik namunalar, laboratoriya natijasi talab bilan solishtiriladi*
- [x] 13. Hajmlarni nazorat qilish — *hajm tekshiruvi va qabul hujjati*
- [x] 14. Geodeziyani tekshirish — *loyiha/fakt/dopusk, chetlanish hisoblanadi*
- [x] 15. Izohlar — *nomuvofiqliklar*
- [x] 16. Izoh toifalari
- [~] 17. AI izohni tasniflashi — *muhimlik darajasi*
- [ ] 18. Chizmadagi izoh
- [x] 19. Bartaraf etishni nazorat qilish
- [x] 20. «Oldin/keyin» fotosi
- [x] 21. Muddatlarni nazorat qilish
- [x] 22. Izohlar jurnali — *tekshiruv bo'yicha muhokama*
- [x] 23. Ijro hujjatlari
- [x] 24. AOSR ni tekshirish — *imzolash/rad etish*
- [x] 25. Sifat nazorati
- [x] 26. AI chek-list yaratadi
- [~] 27. PPR nazorati
- [~] 28. Ishlar ketma-ketligini nazorat qilish
- [~] 29. Loyihani nazorat qilish
- [ ] 30. Versiyalarni nazorat qilish
- [ ] 🔒 31. Texnik nazorat uchun AI Clash
- [ ] 32. Qurilishdagi o'zgarishlarni nazorat qilish
- [~] 33. AI-yordamchi
- [x] 34. Texnik nazoratning kunlik hisoboti — *oxirgi 14 kun, yozuvlardan yig'iladi*
- [x] 35. Obyekt tayyorligini nazorat qilish — *yetti shart bo'yicha*
- [x] 36. Yakuniy qabul — *tayyorlik va to'siqlar ro'yxati*
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
- [x] 11. Qiymat o'zgarishlari — *faqat kelishilgani shartnoma summasiga qo'shiladi*
- [x] 12. Qo'shimcha ishlarni nazorat qilish — *qo'shimcha, chiqarish, narx va muddat o'zgarishi*
- [~] 13. Smeta
- [x] 14. Xaridlar — *kabinetda yirik xaridlar: kimdan, qancha va taklif solishtirilganmi*
- [x] 15. Takliflarni solishtirish — *har xarid yonida nechta taklif borligi ko'rinadi*
- [x] 16. Texnik nazorat kabineti
- [x] 17. Izohlar — *buyurtmachi izohlari alohida modul sifatida, holati bilan*
- [x] 18. Buyurtmachi izoh yarata olishi — *kabinetdagi yagona yozish huquqi*
- [~] 19. Ijro hujjatlari
- [~] 20. Ijro hujjatlarini AI tekshiruvi
- [x] 21. Ishlarni qabul qilish — *topshirildi → qabul/rad etildi, sabab bilan*
- [~] 22. Elektron kelishuv — *ilova ichidagi qaror: kim va qachon; ERI tashqi omil*
- [~] 23. Loyiha hujjatlari
- [ ] 24. Loyiha versiyalarini AI solishtirishi
- [~] 25. Loyiha xatolarini nazorat qilish
- [x] 26. Xavfsizlik
- [x] 27. Shartnomalar — *bosh pudrat, subpudrat, yetkazib berish; avans va kafolat ushlanmasi*
- [x] 28. Contract Monitor — *olti qoida: chetlanish, kutilayotgan qaror, kechikkan to'lov, muddati o'tgan shartnoma, jadval farqi, javobsiz qabul*
- [x] 29. To'lovlar — *to'lov jadvali: bosqich, muddat, to'langan, qoldiq*
- [x] 30. AI Payment Control — *qarz va kechikish ajratiladi, o'rtacha kechikish, 30 kunlik prognoz*
- [x] 31. Risklar — *shartnoma ogohlantirishlari va analitika topilmalari*
- [~] 32. Buyurtmachining AI-yordamchisi
- [x] 33. Haftalik hisobot — *kabinetdagi blok: bajarilish, ishlar, tekshiruv, pul*
- [x] 34. Kirish darajalari — *rollar*
- [ ] 🔒 35. Mobil kabinet
- [x] 36. Kabinetning eng muhim funksiyasi
- [x] 37. Asosiy tamoyil

---

## IX. Arizalar

- [x] 1. Modul maqsadi
- [x] 2. Kim ariza yarata oladi — *rollar*
- [x] 3. Ariza turlari — *material, texnika, transport, ta'mir, ishchi kuchi, xizmat, pul, hujjat, boshqa*
- [x] 4. Materialga ariza
- [x] 5. AI o'zi ariza taklif qilishi — *zaxira bo'yicha*
- [x] 6. Ariza muddatni hisobga olishi
- [x] 7. Shoshilinchlik
- [x] 8. Kelishuv marshruti — *bosqichlar tartib bilan, rol bo'yicha*
- [x] 9. Avtomatik limitlar — *summa marshrut uzunligini belgilaydi*
- [x] 10. Byudjetni tekshirish — *bo'lim byudjeti bilan*
- [x] 11. Dublikatni tekshirish — *shu material bo'yicha ochiq ariza raqami bilan ko'rsatiladi*
- [x] 12. Smetani tekshirish — *material smetada bormi, kod yoki nom bo'yicha*
- [ ] 13. Loyihaga muvofiqlikni tekshirish
- [x] 14. Materialni almashtirish — *arzonroq analog tejash summasi bilan taklif qilinadi*
- [x] 15. Tijorat takliflari
- [x] 16. AI yetkazib beruvchilarni solishtirishi
- [x] 17. Ariza → xarid
- [x] 18. Ariza → buyurtma — *tanlangan taklifdan xarid*
- [x] 19. Ariza → ombor
- [x] 20. Qisman yetkazish
- [x] 21. Muddati o'tgan arizalarni nazorat qilish
- [~] 22. Texnikaga ariza — *tur bor, jarayon yo'q*
- [x] 23. Transportga ariza — *ariza turi*
- [x] 24. Ta'mirga ariza — *ariza turi*
- [x] 25. Pulga ariza — *ariza turi*
- [x] 26. Xizmatga ariza — *ariza turi*
- [ ] 27. Xodimga ariza
- [x] 28. Foto va hujjatlar
- [ ] 🔒 29. Ovozli arizalar
- [x] 30. Ijroni nazorat qilish — *qoplanish*
- [x] 31. Ariza tarixi — *kim, qachon, qanday qaror qildi*
- [x] 32. Rad etish sababi — *sababsiz rad ogohlantiriladi*
- [~] 33. Arizalarni AI tahlili
- [~] 34. Rahbar paneli
- [~] 35. AI-panel
- [x] 36. GPR bilan bog'lanish — *ariza ishga bog'lanadi; bog'lanmagani tekshiruvda aytiladi*
- [x] 37. Ombor bilan bog'lanish
- [x] 38. Smeta bilan bog'lanish — *smetada yo'q material belgilanadi*
- [ ] 39. Buxgalteriya bilan bog'lanish
- [~] 40. Yakuniy nazorat
- [x] 41. Modulning bosh ekrani
- [~] 42. Tizim prediktiv bo'lishi

---

## X. Xaridlar

- [x] 1. Modul maqsadi
- [x] 2. Xaridlarning bosh ekrani
- [x] 3. Har xaridning manbai — *arizaga bog'lanadi*
- [x] 4. AI xaridlarni rejalashtirishi — *«Xarid rejasi» tabi: qoldiq, yo'ldagi buyurtma va normativ ehtiyoj*
- [x] 5. Ehtiyojni avtomatik hisoblash — *minimal zaxira va yaqin ishlar normasi bo'yicha, 45 kunlik ufq*
- [x] 6. Xariddan oldin tekshirish — *reja arizasi bor-yo'qligini ko'rsatadi, takrorlashni oldini oladi*
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
- [x] 18. Texnik kelishuv
- [x] 19. Materialni almashtirish — *tasdiqlangan analog talab qilinadi*
- [~] 20. Buyurtma shakllantirish — *xarid yozuvi*
- [x] 21. Shartnoma — *ta'minot shartnomasi va xaridga bog'lanish*
- [x] 22. Yetkazib beruvchi shartnomasini AI tekshiruvi — *summa va muddat*
- [~] 23. To'lovni nazorat qilish — *holat bor, to'lov grafigi yo'q*
- [x] 24. Yetkazishni nazorat qilish
- [~] 25. Obyektda qabul qilish
- [~] 26. Kirish nazorati — *sifat moduli orqali*
- [ ] 🔒 27. AI sertifikatni tekshiradi
- [x] 28. Qabulda foto
- [x] 29. Ombor bilan bog'lanish — *bir bosishda kirim*
- [x] 30. Qisman yetkazish — *kelgan miqdor, qoldiq, kirim*
- [x] 31. Kechikishlarni nazorat qilish
- [x] 32. GPR bilan bog'lanish — *xarid ishga bog'lanadi; ariza orqali ham*
- [x] 33. Obyektlar bo'yicha xaridlarni nazorat qilish
- [x] 34. Bo'limlar bo'yicha xaridlarni nazorat qilish
- [x] 35. Xarid byudjetini nazorat qilish — *reja / buyurtma / qoldiq*
- [~] 36. AI ortiqcha sarfni aniqlashi
- [x] 37. Markazlashtirilgan xaridlar
- [x] 38. Turli obyektlar xaridlarini solishtirish
- [x] 39. Sarfni nazorat qilish — *byudjet, xaridchi va risk kesimlari*
- [x] 40. Yetkazib beruvchilar tahlili — *muddatida %, o'rtacha kechikish*
- [x] 41. Korrupsiya riskini nazorat qilish — *yetkazib beruvchi ulushi, taklifsiz xarid, narx oshishi, shoshilinch ulushi*
- [x] 42. Xaridlarni avtomatik bo'lish
- [x] 43. Shoshilinch xaridlar — *belgi va ulush nazorati*
- [ ] 🔒 44. AI narx prognozi
- [~] 45. Direktorning bosh hisoboti
- [x] 46. Xaridchi samaradorligi — *xarid soni, summa, muddatida %, taklif bilan %*
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
- [x] 26. Kamomad sababini izlash — *takrorlangan farq tizimli sabab degani; puldagi zarar bo'yicha tartib*
- [x] 27. Yaroqlilik muddati — *partiyada*
- [x] 28. FIFO / FEFO — *navbatdagi partiya belgilanadi*
- [ ] 29. Harorat nazorati
- [~] 30. **YoMM** — *texnika modulida yoqilg'i*
- [ ] 31. AI YoMM nazorati
- [x] 32. Asboblar — *katalog: tur, inventar raqami, holat, tekshiruv muddati*
- [x] 33. Asbob berish — *kimga, qachon, qaysi muddatgacha*
- [x] 34. Asbobni nazorat qilish — *kimda, necha kundan beri, muddati o'tganmi*
- [ ] 35. **Ish kiyimi va SIZ**
- [x] 36. Xaridlar bilan bog'lanish
- [ ] 37. Buxgalteriya bilan bog'lanish
- [x] 38. Materialning o'rtacha qiymati — *vaznlangan o'rtacha*
- [~] 39. Narxlarni nazorat qilish
- [~] 40. Rahbar paneli
- [x] 41. Uzoq turgan materiallar — *90 kun harakatsiz → sariq*
- [x] 42. Obyektlar orasida qayta taqsimlash — *ortiqcha va yetishmayotgan solishtiriladi; minimal zaxiraga tegilmaydi*
- [x] 43. Nolikvidlarni nazorat qilish — *ortiqcha zaxira analitikada pulda o'lchanadi*
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
- [x] 8. AI materialning loyihaga mosligini tekshirishi
- [x] 9. Analoglar — *tasdiqlangan almashtiruvchi*
- [x] 10. Materiallarni solishtirish — *narx farqi va qoldiq*
- [~] 11. Almashtiruvchi tanlash — *narx, qoldiq va tasdiq holati ko'rsatiladi*
- [x] 12. Juda muhim qoida — *bir material — bitta kartochka*
- [x] 13. Sertifikatlar
- [ ] 🔒 14. AI OCR sertifikatlar
- [x] 15. AI sertifikatni tekshiradi — *raqam, muddat va ishlatilgani*
- [x] 16. Hujjat amal qilish muddati
- [x] 17. Material → partiya — *ombor partiyalari*
- [~] 18. Material → yetkazib beruvchi — *xarid orqali*
- [x] 19. Narx tarixi — *kirimlardan, o'zgarish foizi bilan*
- [ ] 🔒 20. Bozor narxi
- [x] 21. Material → sarf normalari
- [~] 22. AI ortiqcha sarfni tahlil qilishi — *farq va summa hisoblanadi*
- [x] 23. Obyektlar bo'yicha materiallar
- [x] 24. Materiallarni qayta taqsimlash
- [~] 25. Tez orada kerak bo'ladigan materiallar — *analitikada*
- [~] 26. AI oldindan ogohlantirishi — *zaxiradan kam*
- [~] 27. GPR bo'yicha materiallar — *ish tayyorligi ko'rinishi*
- [x] 28. Ish boshlanishidan oldin mavjudlikni nazorat qilish — *ikki hafta oldin*
- [x] 29. Material → ijro hujjati — *ish orqali*
- [x] 30. To'liq kuzatuvchanlik — *yetkazuvchi → partiya → ish → hujjat*
- [x] 31. Maxsus talabli materiallar — *katalogda alohida maydon*
- [x] 32. AI moslikni tekshirishi
- [x] 33. Material komplekti
- [x] 34. Ishlab chiqaruvchilarni solishtirish
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
- [x] 20. Anomaliyalarni nazorat qilish — *kunlik chegara, dam olish kuni, dam olishsiz ketma-ketlik, bir xil yozuv*
- [x] 21. Ish vaqtini nazorat qilish — *anomaliyalar ro'yxati ishchi kesimida*
- [x] 22. **Bo'sh turishlar hisobi** — *to'lanadi, ishlangan soat emas*
- [~] 23. Bo'sh turishlar tahlili — *brigada bo'yicha ulush*
- [x] 24. Unumdorlik — *bir birlik ish: soat va pul, o'rtacha bilan solishtirish*
- [x] 25. Brigadalarni solishtirish
- [x] 26. Xodimlar sonini rejalashtirish — *yaqin 30 kun uchun kerakli soat va ishchi soni*
- [x] 27. Xodim ehtiyoji prognozi — *bugungi unumdorlikdan; unumdorligi noma'lum ish hisobga kirmaydi*
- [ ] 28. Xodimlarni ko'chirish
- [x] 29. Tabel → ish haqi
- [x] 30. Tabel → tannarx
- [x] 31. Aniq ishning tannarxi — *ish haqi + material, bir birlikka*
- [~] 32. Ish haqi fondini nazorat qilish
- [~] 33. Buxgalteriya uchun tabel — *soat, yo'qlik, ish haqi jadvali*
- [x] 34. Tabelni tuzatish — *davrni qayta ochish orqali, sabab bilan*
- [x] 35. Oyni yopish — *yopilgan oy tasodifan o'zgarmaydi*
- [x] 36. Tasdiqlash — *kim va qachon yopgani yoziladi; qayta ochish iz qoldiradi*
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
- [x] 6. Brakka chiqarilgan material — *taqiqdan keyin chiqarilgani ko'rsatiladi: qancha va necha marta*
- [x] 7. Operatsion nazorat
- [x] 8. Chek-listlar — *namuna, normativ havolasi, tekshiruvga ko'chiriladi*
- [x] 9. Yashirin ishlar nazorati — *chek-list bandi sifatida*
- [x] 10. Keyingi bosqichni bloklash — *ochiq nuqson ishni yopishga qo'ymaydi*
- [x] 11. Sifat fotofiksatsiyasi
- [ ] 🔒 12. Fotolarni AI tahlili
- [x] 13. Geometriya nazorati — *geodeziya: loyiha/fakt/dopusk (VII)*
- [ ] 🔒 14. BIM Quality Control
- [~] 15. Bo'limlarni nazorat qilish
- [~] 16. Kolliziyalarni tekshirish — *AI tekshiruvida*
- [x] 17. Izohlar
- [x] 18. Izohlar tasnifi
- [~] 19. AI ustuvorlikni aniqlashi
- [x] 20. Izohni bartaraf etish
- [x] 21. Oldin / keyin fotosi
- [x] 22. Laboratoriya sinovlari — *«Sinovlar» tabi: payvand, bosim, izolyatsiya, grunt, ishga tushirish*
- [x] 23. Beton nazorati — *7 va 28 kunlik namunalar (VII)*
- [x] 24. Payvand nazorati — *sinov turi, talab bilan solishtiriladi*
- [x] 25. Muhandislik tizimlari sinovlari — *bosim, izolyatsiya, ishga tushirish*
- [~] 26. PPR nazorati
- [~] 27. Texnologik ketma-ketlik nazorati
- [x] 28. Mas'ullar bo'yicha sifat nazorati — *tekshiruv, salbiy, ochiq nuqson, muddat*
- [x] 29. Sifat reytingi — *ball umumiy ball bilan bir xil qoidada*
- [x] 30. Brak sabablarini tahlil qilish
- [x] 31. Takrorlanuvchi nuqsonlar — *bir xil sabab guruhlanadi*
- [x] 32. Prediktiv sifat nazorati — *ikki va undan ortiq sabab bo'lsa ish e'tibor ro'yxatiga tushadi*
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
- [x] 15. Xavfli zonalarni nazorat qilish — *zona, chora, mas'ul va tekshiruv muddati*
- [x] 16. Yuk ko'tarish ishlari — *ruxsat turi va xavfli zona*
- [~] 17. Texnika — *texnik ko'rik muddati*
- [ ] 18. Texnikaning kunlik ko'rigi
- [x] 19. Elektr xavfsizligi — *ruxsat turi va xavfli zona*
- [x] 20. O't ishlari — *ruxsat turi va naryad*
- [x] 21. Yer ishlari — *ruxsat turi va xavfli zona*
- [x] 22. Yong'in xavfsizligi — *yong'in inventari zonalar ro'yxatida, tekshiruv muddati bilan*
- [x] 23. Evakuatsiya rejasi — *rejalar va yo'llar, ularning holati*
- [x] 24. Favqulodda vaziyatlar — *aloqa va inventar zonalar ro'yxatida*
- [x] 25. Hodisa haqida xabar
- [x] 26. **Near Miss**
- [x] 27. Hodisalarni tahlil qilish — *sabab kesimida, ulushi va jiddiylari bilan*
- [x] 28. Root Cause Analysis — *sakkizta ildiz sabab; faqat haqiqiy hodisalar sanaladi*
- [x] 29. Korrektiv chora-tadbirlar
- [x] 30. Bartaraf etishni nazorat qilish
- [ ] 🔒 31. Bartaraf etilgandan keyin AI foto tekshiruvi
- [~] 32. Kunlik Safety Report
- [x] 33. **Safety Score** — *hodisa, ruxsat, SIZ va naryad kamchiliklari*
- [x] 34. Obyektlar reytingi — *«Obyektlar» ekranida xavfsizlik balli*
- [x] 35. Mas'ullar reytingi — *hodisa, buzilish, baxtsiz hodisa, ochiq va muddati o'tgan*
- [x] 36. Risk Prediction — *zona chorasi, takrorlangan sabab, ishga qo'yib bo'lmaydigan ishchi, kamchilikli naryad*
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
- [x] 9. Texnikani taqsimlash — *bandlik rejasi: qaysi texnika qaysi ishda*
- [x] 10. GPR bilan bog'lanish — *bandlik ishga bog'lanadi*
- [x] 11. Texnikani rejalashtirish — *muddat va kunlik smena soni*
- [x] 12. To'qnashuvlarning oldini olish — *bir texnika ikki ishda: kesishgan kunlar ko'rsatiladi*
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
- [x] 23. Ta'mir — *rejali, nosozlik, TX, texnik ko'rik; xarajat va bo'sh turgan kun*
- [x] 24. Ta'mirga ariza — *ariza turi (IX) va ta'mir yozuvi*
- [x] 25. Rejali TX — *motosoat oralig'i, qolgani*
- [~] 26. Ta'mir prognozi — *nosozliklar soni va xarajati ko'rinadi; prognoz formulasi yo'q*
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
- [~] 42. Samaradorlik tahlili — *bo'sh turish, ta'mir xarajati va foydalanish koeffitsiyenti; umumiy ball yo'q*
- [x] 43. «Ta'mirlash yoki almashtirish» — *ta'mir qiymati balansning 40 % idan oshsa belgilanadi*
- [~] 44. Mashina hujjatlari — *texnik ko'rik*
- [~] 45. Bildirishnomalar — *TX muddati markazga chiqadi; push tashqi omil*
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
- [x] 4. Obyektlar bo'yicha analitika — *«Obyektlar» ekrani: barcha obyektlar yonma-yon, e'tibor talab qiladiganlari oldinda*
- [x] 5. Plan-fakt
- [x] 6. GPR tahlili
- [~] 7. AI kechikish sababini aniqlashi
- [x] 8. Obyekt tugash prognozi
- [x] 9. Kritik yo'l
- [~] 10. Risklar prognozi
- [x] 11. Moliyaviy analitika
- [x] 12. Tannarx tahlili — *ish haqi, material, texnika*
- [x] 13. Yakuniy tannarx prognozi — *bugungi tannarx ÷ bajarilish ulushi; hisob ochiq yozilgan*
- [x] 14. Foyda prognozi — *«Prognoz» tabi: yakuniy tannarx, foyda va marja*
- [x] 15. Materiallar tahlili
- [~] 16. Xaridlar tahlili
- [x] 17. Tejashni izlash — *«Yo'qotishlar» tabi*
- [x] 18. Ombor tahlili
- [x] 19. Xodimlar tahlili
- [x] 20. Texnika tahlili
- [x] 21. Sifat tahlili
- [x] 22. Xavfsizlik tahlili
- [ ] 23. Pudratchilar tahlili
- [ ] 24. Yetkazib beruvchilar tahlili
- [ ] 25. Korrelyatsion tahlil
- [x] 26. Unumdorlik tahlili — *bir birlik ish: soat va pul, o'rtacha bilan solishtirish*
- [~] 27. Benchmarking — *ishlar o'rtacha bilan solishtiriladi; obyektlar orasida hali yo'q*
- [~] 28. Loyihalar tahlili
- [~] 29. Smeta tahlili
- [x] 30. **Pul oqimlari tahlili** — *oylar kesimida fakt va reja*
- [x] 31. **Kassa uzilishini rejalashtirish** — *birinchi manfiy oy va summa*
- [x] 32. Debitorlik qarzi tahlili — *to'lov jadvalidan: qoldiq va muddati o'tgani*
- [x] 33. Tushum prognozi — *yaqin 90 kun, to'lov jadvali bo'yicha*
- [x] 34. Shartnomalar tahlili — *amaldagi summa kelishilgan o'zgarishlar bilan*
- [ ] 35. Loyiha o'zgarishlari tahlili
- [x] 36. «Nima bo'ladi, agar?» — *muddat, material narxi va ish haqi bo'yicha ssenariy*
- [x] 37. **Kunlik xulosa** — *faqat bugungi muddat, yetkazish, hodisa*
- [ ] 38. **AI Weekly Management Report**
- [~] 39. AI sonlarni tushuntirishi — *fakt/hisob/tavsiya*
- [x] 40. Ogohlantirishlar tizimi
- [x] 41. Imkoniyatlarni topish — *har biri pulda o'lchanadi va manba ekraniga bog'langan*
- [~] 42. **Qarorlar markazi**
- [x] 43. Ssenariy modellashtirish — *natija: tugash sanasi, tannarx va foyda*
- [x] 44. Yashirin yo'qotishlar — *ortiqcha zaxira, bo'sh texnika, normadan sarf, tanlanmagan tejash*
- [x] 45. Chetlanishlar tahlili
- [x] 46. Yagona AI chat — *o'n uch savol, har biri manba ekraniga bog'langan*
- [~] 47. Darajalar bo'yicha analitika — *ko'rsatkich → bo'lim*
- [x] 48. **Drill-down** — *kartochka, topilma va oqim qatoridan bo'limga*
- [ ] 49. **AI Executive Score** — *sog'lomlik indeksi qisman*
- [~] 50. XVII ning bosh arxitekturasi
- [x] 51. Eng muhim funksiya

---

## XVIII. AI Copilot

> TZ ga ko'ra Copilot — **butun platformaning operatsion qatlami**: tabiiy
> tilni tushunadi, ma'lumot topadi, tahlil qiladi, tushuntiradi, yechim taklif
> qiladi va **tasdiqlangandan keyin amalni bajaradi**.
>
> Qurilgani: qoidalarga asoslangan savol-javob (son har doim baza hisobidan)
> va **OpenAI bilan to'liq integratsiya** — ko'p bosqichli suhbat, fon
> so'rovi, xatolarni aniq ko'rsatish, token hisobi. Model **son hisoblamaydi**:
> har so'rovga ilova hisoblab bergan sonlar biriktiriladi.
>
> Integratsiya yig'ilishga kiradi, lekin **sukut bo'yicha o'chiq**: sozlamada
> yoqilib API kalit kiritilmaguncha ilova hech qayerga ulanmaydi. Tarmoq kodi
> umuman kerak bo'lmasa — `cargo build --no-default-features`.

- [x] 1. Asosiy vazifa — *qisman: savol-javob*
- [x] 2. Bosh ekran
- [x] 3. Tabiiy nutqni tushunish — *OpenAI modeli; kalit so'zlar zaxira sifatida*
- [~] 4–10. Ovoz, kontekst, ko'p bosqichli suhbat — *suhbat va kontekst bor; ovoz 🔒*
- [x] 11. Rol bo'yicha kirish chegarasi
- [~] 12–20. Qoralamalar — *ariza, xarid, ijro hujjati, kelishuv marshruti*
- [x] 21–30. Amalni bajarish — *tasdiqdan keyin, rol huquqi bilan*
- [~] 31–40. Hodisa → tavsiya → nazorat — *taklif bajarilgach ro'yxatdan chiqadi*
- [~] 41. Ma'lumot manbasini ko'rsatish — *«Tekshirish» tugmasi*
- [x] 42. Sonni o'ylab topmaslik
- [~] 43. Modullar bo'ylab kirish — *13 mavzu*
- [x] 44. Copilot — operatsion qatlam — *yetti xil amal va til modeli*
- [x] 45. TZ uchun asosiy ta'rif — *chegara ochiq yozilgan*

---

## Umumiy (TZ da alohida modul emas, lekin kerak)

- [x] Ikki til (uz/ru), yorug'/qorong'i mavzu, interfeys masshtabi
- [x] Umumiy qidiruv (Ctrl+K)
- [x] Rollar va yozish huquqi
- [x] Zaxira nusxa (`VACUUM INTO`)
- [x] Qurilmalar orasida paket almashish
- [x] Baza migratsiyasi (eski baza ochilaveradi)
- [x] **Bir necha obyekt bo'yicha konsolidatsiya** — *«Obyektlar» ekrani, vaznlangan bajarilish, Excel eksporti*
- [ ] 🔒 **Server va jonli sinxronizatsiya**
- [ ] 🔒 **Mobil klient**
- [x] **Hujjat generatsiyasi** — *KS-2, KS-3, M-29, AOSR — `.xlsx` shaklida*
- [~] **Excel eksporti** — *20 ta ekran jadvali, Ctrl+E; PDF hali yo'q*
- [x] **Bildirishnomalar tizimi** — *ilova ichida: 9 modul signali, yon panelda son; SMS/Telegram tashqi omil*
- [x] **Amallar tarixi (audit log)** — *har bir qo'shish, o'zgartirish va o'chirish; 50 000 yozuv saqlanadi*
- [x] **Izoh va muhokama** — *har qanday yozuvga izoh, javob va «hal qilindi»; fayl va «oldin/keyin» fotosi*

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
3. ~~Bir nechta obyekt kesimida analitika~~ — **bajarildi** (XVII.4: «Obyektlar» ekrani)
4. **Smeta ↔ material ↔ ish bog'lanishi**: pozitsiya darajasida (III.28, XII.5–7)
5. **Debitorlik va to'lov intizomi** (XVII.32–33, XX)
6. **Pudratchilar va yetkazib beruvchilar reytingi** (XIV.28–29, XV.35, XVII.23–24)
7. **Ssenariy modellashtirish «nima bo'ladi, agar?»** (XVII.36, 43)

Server, DWG/RVT va til modeli bo'yicha ishlar `USTA.md` da alohida yozilgan —
ular ilovadan tashqaridagi qarorlarni talab qiladi.
