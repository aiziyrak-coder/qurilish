//! Foto-nazorat: rasm dalil bo'la oladimi (TZ V.9, X.9, XI.9/10, XIII.12,
//! XIV.14/31, XV.12).
//!
//! TZ da bu band «fotolarni AI tahlili» deb yozilgan va odatda undan
//! rasmning mazmunini tanish tushuniladi. Mazmunni tanish tashqi model
//! ishi — u bu yerda yo'q va bor deb ko'rsatilmaydi.
//!
//! Lekin qurilishda fotoning asosiy vazifasi — **dalil**. Dalil sifatida
//! rasmdan so'raladigan savollar mazmundan oldin keladi:
//!
//! * rasm shu kuni olinganmi yoki eski papkadan olib qo'yilganmi;
//! * obyektda olinganmi yoki boshqa joyda;
//! * tahrirlangan dasturdan o'tganmi;
//! * xuddi shu rasm boshqa yozuvga ham qo'yilmaganmi.
//!
//! Bularning hammasi rasmning o'zida yozilgan (EXIF) va tashqi xizmatsiz
//! tekshiriladi. Shuning uchun shu yerda aynan shu qilinadi.
//!
//! Qat'iy qoida avvalgidek: **ma'lumot bo'lmasa hukm ham yo'q.** EXIF
//! o'chirilgan bo'lsa (masalan, rasm messenjerdan o'tgan) — bu «yolg'on»
//! degani emas, «tekshirib bo'lmadi» degani, va shunday ham aytiladi.

use crate::exif;
use crate::geo::{Fence, Point};
use chrono::NaiveDate;

/// Tekshiriladigan bitta rasm.
#[derive(Debug, Clone)]
pub struct Shot {
    /// Fayl yo'li — yozuvda shu saqlanadi.
    pub file: String,
    /// Rasm qaysi yozuvga biriktirilgan (jurnal sanasi, dalolatnoma raqami).
    pub owner: String,
    /// Yozuv qaysi kunga tegishli.
    pub date: NaiveDate,
}

/// Foto haqidagi e'tiroz.
#[derive(Debug, Clone, PartialEq)]
pub enum PhotoIssue {
    /// EXIF yo'q — rasm dalil sifatida zaif.
    NoData { file: String, owner: String },
    /// Rasm boshqa kuni olingan.
    WrongDay {
        file: String,
        owner: String,
        taken: NaiveDate,
        expected: NaiveDate,
        days: i64,
    },
    /// Rasm obyekt doirasidan tashqarida olingan.
    OutsideFence {
        file: String,
        owner: String,
        distance: f64,
    },
    /// Rasm tahrirlovchi dasturdan o'tgan.
    Edited {
        file: String,
        owner: String,
        software: String,
    },
    /// Bitta rasm bir necha yozuvga qo'yilgan.
    Reused { file: String, owners: Vec<String> },
}

impl PhotoIssue {
    /// Jiddiy e'tiroz — hujjatga qo'yishdan oldin hal qilinishi kerak.
    ///
    /// «Ma'lumot yo'q» jiddiy emas: u ayblov emas, eslatma.
    pub fn severe(&self) -> bool {
        matches!(
            self,
            PhotoIssue::WrongDay { .. }
                | PhotoIssue::OutsideFence { .. }
                | PhotoIssue::Reused { .. }
        )
    }

    /// Qaysi faylga tegishli.
    pub fn file(&self) -> &str {
        match self {
            PhotoIssue::NoData { file, .. }
            | PhotoIssue::WrongDay { file, .. }
            | PhotoIssue::OutsideFence { file, .. }
            | PhotoIssue::Edited { file, .. }
            | PhotoIssue::Reused { file, .. } => file,
        }
    }

    /// Odam o'qiydigan izoh.
    pub fn text(&self) -> String {
        use crate::i18n::t;
        match self {
            PhotoIssue::NoData { .. } => t("ph_no_data").to_string(),
            PhotoIssue::WrongDay {
                taken,
                expected,
                days,
                ..
            } => format!(
                "{} — {taken} ({} {expected}, {days} {})",
                t("ph_wrong_day"),
                t("ph_expected"),
                t("days_short")
            ),
            PhotoIssue::OutsideFence { distance, .. } => {
                format!("{} — {}", t("ph_outside"), crate::geo::meters(*distance))
            }
            PhotoIssue::Edited { software, .. } => format!("{}: {software}", t("ph_edited")),
            PhotoIssue::Reused { owners, .. } => {
                format!("{} — {}", t("ph_reused"), owners.join(", "))
            }
        }
    }
}

/// Kun farqi shu chegaradan oshsa e'tiroz bo'ladi.
///
/// Bir kun — chegaraviy holat: kechqurun olingan rasm ertasiga
/// yozilishi mumkin, va telefon vaqti bir necha soatga surilgan bo'lishi
/// ham mumkin. Shuning uchun bir kun kechikish e'tiroz emas.
pub const DAY_TOLERANCE: i64 = 1;

/// Tahrirlovchi dastur nomlari — EXIF `Software` maydonida uchraydigan.
///
/// Ro'yxat qisqa va ochiq: kamera va telefonning o'z dasturi (versiya
/// raqami, «Camera», model nomi) tahrir emas, shuning uchun bu yerda
/// faqat rasm ustida ishlaydigan tanilgan dasturlar turadi.
const EDITORS: [&str; 6] = [
    "photoshop",
    "lightroom",
    "gimp",
    "snapseed",
    "picsart",
    "paint.net",
];

/// Rasmlarni tekshiradi.
///
/// `read` — faylni o'qiydigan funksiya. U alohida berilgani sinov uchun:
/// haqiqiy diskka bog'lanmasdan turli holatlarni tekshirish mumkin.
pub fn check_with(
    shots: &[Shot],
    fence: Option<Fence>,
    read: impl Fn(&str) -> Option<exif::Meta>,
) -> Vec<PhotoIssue> {
    let mut out = Vec::new();

    // --- Bitta rasm bir necha yozuvda.
    let mut seen: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for s in shots {
        let key = normalize(&s.file);
        let list = seen.entry(key).or_default();
        if !list.contains(&s.owner) {
            list.push(s.owner.clone());
        }
    }
    for (file, owners) in &seen {
        if owners.len() > 1 {
            out.push(PhotoIssue::Reused {
                file: file.clone(),
                owners: owners.clone(),
            });
        }
    }

    for s in shots {
        let Some(meta) = read(&s.file) else {
            out.push(PhotoIssue::NoData {
                file: s.file.clone(),
                owner: s.owner.clone(),
            });
            continue;
        };
        if !meta.any() {
            out.push(PhotoIssue::NoData {
                file: s.file.clone(),
                owner: s.owner.clone(),
            });
            continue;
        }

        if let Some(taken) = exif::day(&meta) {
            let days = (s.date - taken).num_days();
            if days.abs() > DAY_TOLERANCE {
                out.push(PhotoIssue::WrongDay {
                    file: s.file.clone(),
                    owner: s.owner.clone(),
                    taken,
                    expected: s.date,
                    days,
                });
            }
        }

        if let (Some(fence), Some(point)) = (fence, meta.point) {
            if let crate::geo::Verdict::Outside { distance, .. } = fence.check(point) {
                out.push(PhotoIssue::OutsideFence {
                    file: s.file.clone(),
                    owner: s.owner.clone(),
                    distance,
                });
            }
        }

        let soft = meta.software.to_lowercase();
        if let Some(name) = EDITORS.iter().find(|e| soft.contains(*e)) {
            out.push(PhotoIssue::Edited {
                file: s.file.clone(),
                owner: s.owner.clone(),
                software: meta.software.clone(),
            });
            let _ = name;
        }
    }

    // Jiddiylari oldinda: ekranda birinchi ko'rinadigan narsa muhimi
    // bo'lsin.
    out.sort_by_key(|i| !i.severe());
    out
}

/// Diskdagi fayllarni o'qib tekshiradi.
pub fn check(shots: &[Shot], fence: Option<Fence>) -> Vec<PhotoIssue> {
    check_with(shots, fence, |file| exif::read(std::path::Path::new(file)))
}

/// Fayl yo'lini solishtirish uchun bir ko'rinishga keltiradi.
///
/// Windows da katta-kichik harf farq qilmaydi va ajratgich ikki xil
/// bo'ladi; shu sababli solishtirishdan oldin tenglashtiriladi.
fn normalize(path: &str) -> String {
    path.trim().replace('\\', "/").to_lowercase()
}

/// Jurnal yozuvlaridan tekshiriladigan ro'yxat tuzadi.
pub fn from_journal(entries: &[crate::domain::JournalEntry]) -> Vec<Shot> {
    entries
        .iter()
        .flat_map(|e| {
            e.photos
                .split(';')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(|p| Shot {
                    file: p.to_string(),
                    owner: e.date.to_string(),
                    date: e.date,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Nuqtalar ro'yxatidan o'rtacha markaz — geozonani taklif qilish uchun.
///
/// Obyekt koordinatasi qo'lda kiritilmagan bo'lsa, uni fotolardan taklif
/// qilish mumkin: bu **taklif**, avtomatik qabul qilinmaydi.
pub fn suggest_center(points: &[Point]) -> Option<Point> {
    let valid: Vec<&Point> = points.iter().filter(|p| p.valid()).collect();
    if valid.is_empty() {
        return None;
    }
    let n = valid.len() as f64;
    Some(Point::new(
        valid.iter().map(|p| p.lat).sum::<f64>() / n,
        valid.iter().map(|p| p.lon).sum::<f64>() / n,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn shot(file: &str, owner: &str, date: NaiveDate) -> Shot {
        Shot {
            file: file.into(),
            owner: owner.into(),
            date,
        }
    }

    fn meta(taken: Option<NaiveDate>, point: Option<Point>, software: &str) -> exif::Meta {
        exif::Meta {
            taken: taken.map(|d| d.and_hms_opt(9, 0, 0).unwrap()),
            point,
            camera: "Samsung SM-A536".into(),
            software: software.into(),
        }
    }

    /// Boshqa kuni olingan rasm ushlanadi, bir kunlik farq esa e'tiroz emas.
    #[test]
    fn photo_from_another_day_is_caught() {
        let shots = vec![
            shot("a.jpg", "2026-09-08", day(2026, 9, 8)),
            shot("b.jpg", "2026-09-08", day(2026, 9, 8)),
            shot("c.jpg", "2026-09-08", day(2026, 9, 8)),
        ];
        let issues = check_with(&shots, None, |f| match f {
            // O'sha kuni — e'tiroz yo'q.
            "a.jpg" => Some(meta(Some(day(2026, 9, 8)), None, "")),
            // Bir kun oldin — kechqurungi rasm, e'tiroz yo'q.
            "b.jpg" => Some(meta(Some(day(2026, 9, 7)), None, "")),
            // Ikki hafta oldin — eski papkadan olingan.
            _ => Some(meta(Some(day(2026, 8, 25)), None, "")),
        });
        let wrong: Vec<&PhotoIssue> = issues
            .iter()
            .filter(|i| matches!(i, PhotoIssue::WrongDay { .. }))
            .collect();
        assert_eq!(wrong.len(), 1, "{issues:?}");
        assert_eq!(wrong[0].file(), "c.jpg");
        assert!(wrong[0].severe());
    }

    /// EXIF yo'q bo'lsa ayblanmaydi — «tekshirib bo'lmadi» deyiladi.
    #[test]
    fn missing_exif_is_a_note_not_an_accusation() {
        let shots = vec![shot("x.jpg", "2026-09-08", day(2026, 9, 8))];
        let issues = check_with(&shots, None, |_| None);
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0], PhotoIssue::NoData { .. }));
        assert!(!issues[0].severe(), "bu ayblov emas");

        // Bo'sh EXIF ham xuddi shunday.
        let issues = check_with(&shots, None, |_| Some(exif::Meta::default()));
        assert!(matches!(issues[0], PhotoIssue::NoData { .. }));
    }

    /// Obyektdan uzoqda olingan rasm ushlanadi; geozona yo'q bo'lsa —
    /// hech qanday hukm chiqarilmaydi.
    #[test]
    fn photo_taken_elsewhere_is_caught_only_with_a_fence() {
        let fence = Fence {
            center: Point::new(41.2995, 69.2401),
            radius: 100.0,
        };
        let far = Point::new(41.3300, 69.2401); // ~3,4 km
        let shots = vec![shot("far.jpg", "2026-09-08", day(2026, 9, 8))];
        let read = |_: &str| Some(meta(Some(day(2026, 9, 8)), Some(far), ""));

        let issues = check_with(&shots, Some(fence), read);
        assert!(issues
            .iter()
            .any(|i| matches!(i, PhotoIssue::OutsideFence { .. })));

        // Geozona kiritilmagan — e'tiroz ham yo'q.
        let issues = check_with(&shots, None, read);
        assert!(issues.is_empty(), "{issues:?}");
    }

    /// Bitta rasm ikki yozuvga qo'yilsa — bir marta e'tiroz bo'ladi.
    #[test]
    fn the_same_photo_cannot_prove_two_things() {
        let shots = vec![
            shot("C:\\foto\\1.JPG", "2026-09-08", day(2026, 9, 8)),
            // Xuddi shu fayl, boshqa yozilishi bilan.
            shot("c:/foto/1.jpg", "2026-09-09", day(2026, 9, 9)),
            shot("2.jpg", "2026-09-08", day(2026, 9, 8)),
        ];
        let issues = check_with(&shots, None, |_| Some(meta(None, None, "")));
        let reused: Vec<&PhotoIssue> = issues
            .iter()
            .filter(|i| matches!(i, PhotoIssue::Reused { .. }))
            .collect();
        assert_eq!(reused.len(), 1, "{issues:?}");
        if let PhotoIssue::Reused { owners, .. } = reused[0] {
            assert_eq!(owners.len(), 2);
        }
    }

    /// Tahrirlovchi dastur nomi belgilanadi, kameraning o'z dasturi — yo'q.
    #[test]
    fn editor_software_is_flagged_camera_firmware_is_not() {
        let shots = vec![shot("e.jpg", "2026-09-08", day(2026, 9, 8))];
        let issues = check_with(&shots, None, |_| {
            Some(meta(Some(day(2026, 9, 8)), None, "Adobe Photoshop 25.0"))
        });
        assert!(issues
            .iter()
            .any(|i| matches!(i, PhotoIssue::Edited { .. })));

        let issues = check_with(&shots, None, |_| {
            Some(meta(Some(day(2026, 9, 8)), None, "A536BXXU9DXH2"))
        });
        assert!(issues.is_empty(), "{issues:?}");
    }

    /// Markaz taklifi faqat haqiqiy nuqtalardan chiqadi.
    #[test]
    fn center_is_suggested_from_real_points_only() {
        assert!(suggest_center(&[]).is_none());
        assert!(suggest_center(&[Point::new(0.0, 0.0)]).is_none());
        let c = suggest_center(&[
            Point::new(41.0, 69.0),
            Point::new(41.2, 69.2),
            Point::new(0.0, 0.0),
        ])
        .expect("markaz");
        assert!((c.lat - 41.1).abs() < 1e-9);
        assert!((c.lon - 69.1).abs() < 1e-9);
    }
}
