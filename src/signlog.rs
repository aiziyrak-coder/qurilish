//! Imzo daftari va uning zanjiri (TZ IV.18, V.28, V.32, XV.22).
//!
//! TZ da «elektron imzo» deb yozilgan va odatda undan **davlat ERI si**
//! tushuniladi: kalit markazda beriladi, ariza va litsenziya bilan.
//! Bunday imzo bu yerda yo'q va bor deb ko'rsatilmaydi.
//!
//! Lekin imzodan kutiladigan asosiy narsa — hujjat imzolangandan **keyin
//! o'zgarmaganini isbotlash** — kalitsiz ham qilinadi va bu yerda aynan
//! shu qilinadi:
//!
//! 1. Imzolanayotgan matnning xeshi olinadi (SHA-256). Hujjat keyin
//!    o'zgarsa, xesh mos kelmaydi.
//! 2. Har imzo **oldingisiga bog'lanadi**: zanjir bo'g'ini oldingi
//!    bo'g'in va shu yozuvdan hisoblanadi. Shuning uchun daftardan bitta
//!    yozuvni o'chirish yoki tuzatish undan keyingi hamma bo'g'inni
//!    buzadi — «bilinmasdan tuzatish» ishlamaydi.
//! 3. Zanjirning uchi vaqti-vaqti bilan **serverga** yoziladi. Bazaning
//!    o'zi ochiq (rol ish taqsimoti, chegara emas) — shuning uchun
//!    dalilning bir uchi boshqa joyda turishi kerak. Server esa o'sha
//!    uzunlikda boshqa uch kelsa, buni aytadi.
//!
//! Nimani isbotlamaydi: kim imzolaganini. Ism yozuvda turadi, lekin uni
//! kalitsiz tasdiqlab bo'lmaydi. Shuning uchun bu **hujjat butunligi**
//! daftari, shaxsni tasdiqlovchi imzo emas — va ekranda ham shunday
//! yoziladi.

use crate::domain::SignEntry;

/// Bitta yozuvning zanjirdagi bo'g'ini.
///
/// Ichiga yozuvning **hamma** ma'noli maydoni kiradi: birortasi
/// o'zgarsa bo'g'in ham o'zgaradi.
pub fn link(prev: &str, e: &SignEntry) -> String {
    qurai_hash::text(&format!(
        "{prev}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        e.document,
        e.subject,
        e.signer,
        e.role,
        e.at.format("%Y-%m-%dT%H:%M:%S"),
        e.digest,
        e.rejected
    ))
}

/// Zanjirning uzunligi va uchi.
pub fn head(entries: &[SignEntry]) -> (usize, String) {
    (
        entries.len(),
        entries.last().map(|e| e.chain.clone()).unwrap_or_default(),
    )
}

/// Zanjirni boshidan qayta hisoblab, `count` uzunlikdagi uchini beradi.
///
/// Serverdagi eski belgi bilan solishtirish uchun kerak: o'sha payt
/// daftar shu uzunlikda edi.
pub fn head_at(entries: &[SignEntry], count: usize) -> Option<String> {
    if count == 0 {
        return Some(String::new());
    }
    entries.get(count - 1).map(|e| e.chain.clone())
}

/// Daftardagi buzilish.
#[derive(Debug, Clone, PartialEq)]
pub enum Break {
    /// Bo'g'in yozuvga mos kelmaydi: yozuv tuzatilgan yoki o'chirilgan.
    Chain { index: usize, document: String },
    /// Imzolangan matn o'zgargan.
    Text { index: usize, document: String },
}

impl Break {
    pub fn document(&self) -> &str {
        match self {
            Break::Chain { document, .. } | Break::Text { document, .. } => document,
        }
    }

    pub fn text(&self) -> String {
        use crate::i18n::t;
        match self {
            Break::Chain { .. } => t("sl_break_chain").to_string(),
            Break::Text { .. } => t("sl_break_text").to_string(),
        }
    }
}

/// Daftarni tekshiradi: har bo'g'in o'z yozuvidan chiqadimi.
pub fn verify(entries: &[SignEntry]) -> Vec<Break> {
    let mut out = Vec::new();
    let mut prev = String::new();
    for (i, e) in entries.iter().enumerate() {
        let expected = link(&prev, e);
        if expected != e.chain {
            out.push(Break::Chain {
                index: i,
                document: e.document.clone(),
            });
            // Zanjir uzilgach, keyingi bo'g'inlar baribir mos kelmaydi —
            // ularni qayta e'lon qilish shovqin bo'lardi. Shuning uchun
            // hisob **yozuvdagi** bo'g'indan davom etadi va faqat yangi
            // uzilish ko'rsatiladi.
            prev = e.chain.clone();
            continue;
        }
        prev = expected;
    }
    out
}

/// Imzolangan matn hali ham o'sha-o'shami.
///
/// Matnni bu modul bilmaydi — uni hujjatdan qayta yig'ish chaqiruvchining
/// ishi. Shu sababli tekshiruv alohida funksiya: daftar ham, hujjat ham
/// o'z joyida qoladi.
pub fn text_matches(entry: &SignEntry, current_text: &str) -> bool {
    entry.digest == qurai_hash::text(current_text)
}

/// Matn tekshiruvini daftarga qo'shadi.
pub fn verify_with_text(
    entries: &[SignEntry],
    text_of: impl Fn(&SignEntry) -> Option<String>,
) -> Vec<Break> {
    let mut out = verify(entries);
    for (i, e) in entries.iter().enumerate() {
        let Some(text) = text_of(e) else {
            // Hujjat topilmadi — bu boshqa masala va bu yerda hukm
            // chiqarilmaydi.
            continue;
        };
        if !text_matches(e, &text) {
            out.push(Break::Text {
                index: i,
                document: e.document.clone(),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(day: u32) -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 9, day)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap()
    }

    fn entry(n: u32, text: &str) -> SignEntry {
        SignEntry {
            id: n as i64,
            project_id: 1,
            document: format!("AOSR-{n}"),
            subject: "Beton quyish".into(),
            signer: "Dilnoza".into(),
            role: "supervisor".into(),
            at: at(n),
            digest: qurai_hash::text(text),
            chain: String::new(),
            rejected: String::new(),
        }
    }

    /// Daftarni tuzadi: har bo'g'in oldingisidan hisoblanadi.
    fn book(texts: &[&str]) -> Vec<SignEntry> {
        let mut out: Vec<SignEntry> = Vec::new();
        let mut prev = String::new();
        for (i, t) in texts.iter().enumerate() {
            let mut e = entry(i as u32 + 1, t);
            e.chain = link(&prev, &e);
            prev = e.chain.clone();
            out.push(e);
        }
        out
    }

    /// To'g'ri daftarda e'tiroz bo'lmaydi.
    #[test]
    fn a_clean_book_has_no_breaks() {
        let b = book(&["birinchi", "ikkinchi", "uchinchi"]);
        assert!(verify(&b).is_empty());
        let (count, head) = head(&b);
        assert_eq!(count, 3);
        assert_eq!(head.len(), 64);
        assert_eq!(head_at(&b, 3), Some(head));
        assert_eq!(head_at(&b, 0), Some(String::new()));
        assert_eq!(head_at(&b, 9), None);
    }

    /// O'rtadagi yozuvni tuzatish yashirin qolmaydi.
    #[test]
    fn editing_a_record_breaks_the_chain() {
        let mut b = book(&["birinchi", "ikkinchi", "uchinchi"]);
        // Kimdir ikkinchi imzoning sanasini «tuzatdi».
        b[1].at = at(9);
        let breaks = verify(&b);
        assert_eq!(breaks.len(), 1, "{breaks:?}");
        assert_eq!(breaks[0].document(), "AOSR-2");
        assert!(matches!(breaks[0], Break::Chain { index: 1, .. }));
    }

    /// Yozuvni o'chirish ham ko'rinadi.
    #[test]
    fn removing_a_record_breaks_the_chain() {
        let mut b = book(&["birinchi", "ikkinchi", "uchinchi"]);
        b.remove(1);
        let breaks = verify(&b);
        assert!(!breaks.is_empty(), "o'chirish sezilmadi");

        // Muhim nozik joy: **uchning o'zi yetarli emas.** O'chirilgan
        // yozuvdan keyingi bo'g'in daftarda o'zgarmay qoladi, shuning
        // uchun uch avvalgidek ko'rinadi.
        let full = book(&["birinchi", "ikkinchi", "uchinchi"]);
        assert_eq!(head(&b).1, head(&full).1, "uch o'zgarmaydi — kutilgani");

        // Ushlaydigan narsa — **uzunlik bilan birga** olingan uch:
        // avval serverga «uchta yozuv, uchi X» deb yozilgan edi, endi esa
        // daftarda uchta yozuv yo'q.
        assert_eq!(head(&b).0, 2);
        assert_eq!(head_at(&b, 3), None, "eski belgini takrorlab bo'lmaydi");
    }

    /// Imzolangan hujjat keyin o'zgarsa — bu alohida aytiladi.
    #[test]
    fn changing_the_document_after_signing_is_seen() {
        let b = book(&["beton 10 m3"]);
        assert!(text_matches(&b[0], "beton 10 m3"));
        assert!(!text_matches(&b[0], "beton 12 m3"));

        // Zanjir butun, lekin matn o'zgargan.
        let breaks = verify_with_text(&b, |_| Some("beton 12 m3".to_string()));
        assert_eq!(breaks.len(), 1);
        assert!(matches!(breaks[0], Break::Text { .. }));

        // Hujjat topilmasa hukm chiqarilmaydi.
        assert!(verify_with_text(&b, |_| None).is_empty());
    }

    /// Ikki uzilish bo'lsa ikkalasi ham ko'rinadi, oradagilar esa emas.
    #[test]
    fn only_real_breaks_are_reported() {
        let mut b = book(&["a", "b", "c", "d", "e"]);
        b[1].signer = "Boshqa".into();
        b[3].rejected = "rad".into();
        let breaks = verify(&b);
        assert_eq!(breaks.len(), 2, "{breaks:?}");
        assert_eq!(breaks[0].document(), "AOSR-2");
        assert_eq!(breaks[1].document(), "AOSR-4");
    }
}
