//! Narxlar bazasi (TZ III.14.1–14.4, III.15, XII.20, X.44).
//!
//! TZ da «bozor narxi» va «narxlar bazasi» yozilgan. Bozor narxini
//! **tashqi manbadan olib kelish** ilovaning ishi emas: har mamlakat va
//! har tarmoqda manba boshqacha, ko'pchiligi pullik va ularning hech
//! biri bu yerda ulanmagan. Uydirma son esa xatoning eng yomon turi:
//! u ishonch uyg'otadi.
//!
//! Shu sababli baza **kiritiladigan** qilingan: tashkilot o'z narx
//! ro'yxatini (ta'minotchi prays-listi, resurs normativlari, tender
//! natijalari) fayldan yuklaydi. Undan keyin ilova uni to'liq ishlatadi:
//!
//! * smetadagi narx bazadagi diapazonga tushadimi (III.14.3);
//! * materialning bugungi narxi qanday (XII.20);
//! * o'z xaridlarimiz tarixi qayoqqa ketyapti (X.44).
//!
//! Oxirgisi ham **prognoz emas, tendensiya**: u faqat o'z kirimlarimizga
//! tayanadi va shunday deb ataladi. Bozorni bilmaydi, bilaman ham
//! demaydi.

use chrono::NaiveDate;

/// Narx ro'yxatining bitta qatori.
#[derive(Debug, Clone, PartialEq)]
pub struct PriceRow {
    pub id: i64,
    /// Rasenka yoki artikul kodi — bo'sh bo'lishi mumkin.
    pub code: String,
    pub name: String,
    pub unit: String,
    pub price: f64,
    /// Manba: ta'minotchi, katalog nomi yoki hujjat raqami.
    pub source: String,
    /// Narx qaysi kunga tegishli.
    pub date: Option<NaiveDate>,
    pub region: String,
}

/// Narx diapazoni.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub count: usize,
}

/// Qatorlardan diapazon tuzadi.
///
/// Nol va manfiy narx hisobga olinmaydi: u «narx yo'q» degani va
/// o'rtachani pastga tortib yuborardi.
pub fn range(rows: &[&PriceRow]) -> Option<Range> {
    let values: Vec<f64> = rows.iter().map(|r| r.price).filter(|p| *p > 0.0).collect();
    if values.is_empty() {
        return None;
    }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    Some(Range {
        min,
        max,
        avg: values.iter().sum::<f64>() / values.len() as f64,
        count: values.len(),
    })
}

/// Narx diapazonga nisbatan qayerda.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verdict {
    /// Bazada bu material yo'q — hukm chiqarilmaydi.
    NoData,
    Inside,
    /// Diapazondan past: `pct` — eng past narxdan necha foiz past.
    Below {
        pct: f64,
    },
    /// Diapazondan yuqori: `pct` — eng yuqori narxdan necha foiz yuqori.
    Above {
        pct: f64,
    },
}

impl Verdict {
    pub fn outside(&self) -> bool {
        matches!(self, Verdict::Below { .. } | Verdict::Above { .. })
    }

    pub fn text(&self) -> String {
        use crate::i18n::t;
        match self {
            Verdict::NoData => t("pb_no_data").to_string(),
            Verdict::Inside => t("pb_inside").to_string(),
            Verdict::Below { pct } => format!("{} {pct:.0}%", t("pb_below")),
            Verdict::Above { pct } => format!("{} {pct:.0}%", t("pb_above")),
        }
    }
}

/// Diapazondan chetlanish shu foizdan oshsagina e'tiroz bo'ladi.
///
/// Narx ro'yxati hech qachon to'liq bo'lmaydi: yetkazish, hajm va
/// muddat narxni o'nlab foizga surib yuboradi. Shuning uchun kichik
/// chetlanish xato emas.
pub const TOLERANCE: f64 = 15.0;

/// Narxni diapazon bilan solishtiradi.
pub fn compare(price: f64, range: Option<Range>) -> Verdict {
    let Some(r) = range else {
        return Verdict::NoData;
    };
    if price <= 0.0 {
        return Verdict::NoData;
    }
    if price < r.min {
        let pct = (r.min - price) / r.min * 100.0;
        return if pct > TOLERANCE {
            Verdict::Below { pct }
        } else {
            Verdict::Inside
        };
    }
    if price > r.max {
        let pct = (price - r.max) / r.max * 100.0;
        return if pct > TOLERANCE {
            Verdict::Above { pct }
        } else {
            Verdict::Inside
        };
    }
    Verdict::Inside
}

/// Nomni solishtirish uchun bir ko'rinishga keltiradi.
fn normal(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Nomdagi ma'noli so'zlar.
fn words(s: &str) -> Vec<String> {
    normal(s)
        .split(' ')
        .filter(|w| w.chars().count() > 2)
        .map(|w| w.to_string())
        .collect()
}

/// Tayyorlangan baza: nomlar bir marta oddiy ko'rinishga keltiriladi.
///
/// Kerakligi sababi oddiy hisobda: katalogda yuzlab material, bazada
/// minglab qator bo'lishi mumkin, va har materialni har qator bilan
/// solishtirish har yozuvdan keyin qayta bajarilardi. Indeks bilan esa
/// nomlar bir marta tayyorlanadi va har material uchun faqat **umumiy
/// so'zi bor** qatorlar ko'rib chiqiladi.
///
/// Natija o'zgarmaydi: so'zlarning yarmidan ko'pi mos kelishi sharti
/// kamida bitta umumiy so'zni talab qiladi, ya'ni indeks hech qanday
/// mos qatorni tashlab ketmaydi.
#[derive(Debug, Default)]
pub struct Index {
    rows: Vec<Prepared>,
    /// Aniq nom → qator o'rni.
    by_name: std::collections::BTreeMap<String, Vec<usize>>,
    /// Kod → qator o'rni.
    by_code: std::collections::BTreeMap<String, Vec<usize>>,
    /// Ma'noli so'z → qator o'rni.
    by_word: std::collections::BTreeMap<String, Vec<usize>>,
}

#[derive(Debug)]
struct Prepared {
    unit: String,
    words: Vec<String>,
}

impl Index {
    pub fn build(rows: &[PriceRow]) -> Index {
        let mut idx = Index::default();
        for (i, r) in rows.iter().enumerate() {
            let name = normal(&r.name);
            let code = normal(&r.code);
            let words = words(&r.name);
            if !name.is_empty() {
                idx.by_name.entry(name).or_default().push(i);
            }
            if !code.is_empty() {
                idx.by_code.entry(code).or_default().push(i);
            }
            for w in &words {
                idx.by_word.entry(w.clone()).or_default().push(i);
            }
            idx.rows.push(Prepared {
                unit: normal(&r.unit),
                words,
            });
        }
        idx
    }

    /// Indeksdagi qatorlar soni — ro'yxat bilan mosligini tekshirish
    /// uchun.
    pub fn len(&self) -> usize {
        self.rows.len()
    }
}

/// Tayyorlangan indeks bilan izlaydi.
pub fn find_with<'a>(
    rows: &'a [PriceRow],
    index: &Index,
    name: &str,
    code: &str,
    unit: &str,
) -> Vec<&'a PriceRow> {
    if rows.len() != index.len() {
        // Indeks boshqa ro'yxatdan tuzilgan — bunda taxmin qilmaymiz.
        return Vec::new();
    }
    let unit_key = normal(unit);
    let unit_ok = |i: usize| {
        unit_key.is_empty() || index.rows[i].unit.is_empty() || index.rows[i].unit == unit_key
    };

    // 1. Kod bo'yicha aniq moslik.
    let code_key = normal(code);
    if !code_key.is_empty() {
        if let Some(hits) = index.by_code.get(&code_key) {
            if !hits.is_empty() {
                return hits.iter().map(|i| &rows[*i]).collect();
            }
        }
    }

    // 2. Nom bo'yicha aniq moslik.
    let name_key = normal(name);
    if name_key.is_empty() {
        return Vec::new();
    }
    if let Some(hits) = index.by_name.get(&name_key) {
        let exact: Vec<&PriceRow> = hits
            .iter()
            .filter(|i| unit_ok(**i))
            .map(|i| &rows[*i])
            .collect();
        if !exact.is_empty() {
            return exact;
        }
    }

    // 3. So'zlarning yarmidan ko'pi mos kelsa.
    let want = words(name);
    if want.is_empty() {
        return Vec::new();
    }
    let mut seen: std::collections::BTreeSet<usize> = Default::default();
    for w in &want {
        if let Some(hits) = index.by_word.get(w) {
            seen.extend(hits.iter().copied());
        }
    }
    seen.into_iter()
        .filter(|i| {
            if !unit_ok(*i) {
                return false;
            }
            let have = &index.rows[*i].words;
            if have.is_empty() {
                return false;
            }
            let hits = want.iter().filter(|w| have.contains(w)).count();
            hits * 2 > want.len()
        })
        .map(|i| &rows[i])
        .collect()
}

// ================================================================ Tendensiya

/// O'z xaridlarimiz tarixidagi yo'nalish (TZ X.44).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trend {
    /// Oyiga o'rtacha o'zgarish, foizda.
    pub per_month: f64,
    /// Nechta kirimga tayanadi.
    pub points: usize,
    /// Birinchi va oxirgi narx.
    pub first: f64,
    pub last: f64,
}

impl Trend {
    /// O'sish yoki tushishmi.
    pub fn rising(&self) -> bool {
        self.per_month > 0.0
    }

    pub fn text(&self) -> String {
        use crate::i18n::t;
        format!(
            "{} {:+.1}% / {} · {} {}",
            t("pb_trend"),
            self.per_month,
            t("pb_month"),
            self.points,
            t("pb_points")
        )
    }
}

/// Tendensiya hisoblanishi uchun kerak bo'lgan eng kam nuqta.
///
/// Ikkita narxdan yo'nalish chiqarish — tasodifni qonun deb ko'rsatish.
pub const MIN_POINTS: usize = 3;

/// Narx tarixidan tendensiya chiqaradi.
///
/// Bu **prognoz emas**: kelajakdagi narx aytilmaydi. Faqat o'tgan
/// kirimlar qaysi tomonga ketgani hisoblanadi va nechta kirimga
/// tayangani ochiq ko'rsatiladi.
pub fn trend(history: &[(NaiveDate, f64)]) -> Option<Trend> {
    let mut points: Vec<(NaiveDate, f64)> =
        history.iter().filter(|(_, p)| *p > 0.0).cloned().collect();
    if points.len() < MIN_POINTS {
        return None;
    }
    points.sort_by_key(|(d, _)| *d);

    let (first_date, first) = points[0];
    let (last_date, last) = points[points.len() - 1];
    let days = (last_date - first_date).num_days();
    if days <= 0 || first <= 0.0 {
        return None;
    }
    let months = days as f64 / 30.44;
    let change = (last - first) / first * 100.0;
    Some(Trend {
        per_month: change / months,
        points: points.len(),
        first,
        last,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, unit: &str, price: f64) -> PriceRow {
        PriceRow {
            id: 0,
            code: String::new(),
            name: name.into(),
            unit: unit.into(),
            price,
            source: "Prays".into(),
            date: None,
            region: String::new(),
        }
    }

    /// Bazada material bo'lmasa hukm chiqarilmaydi.
    #[test]
    fn no_entry_means_no_verdict() {
        assert_eq!(compare(1000.0, None), Verdict::NoData);
        assert!(range(&[]).is_none());
        // Nol narxli qatorlar diapazon bermaydi.
        let zero = row("Sement", "kg", 0.0);
        assert!(range(&[&zero]).is_none());
        // Narx nol bo'lsa ham hukm yo'q.
        let r = row("Sement", "kg", 1000.0);
        assert_eq!(compare(0.0, range(&[&r])), Verdict::NoData);
    }

    /// Kichik chetlanish xato emas, kattasi esa ko'rsatiladi.
    #[test]
    fn only_a_real_gap_is_reported() {
        let rows = [row("Sement", "kg", 1000.0), row("Sement", "kg", 1200.0)];
        let refs: Vec<&PriceRow> = rows.iter().collect();
        let r = range(&refs).expect("diapazon");
        assert_eq!(r.min, 1000.0);
        assert_eq!(r.max, 1200.0);
        assert_eq!(r.count, 2);

        // Diapazon ichida.
        assert_eq!(compare(1100.0, Some(r)), Verdict::Inside);
        // 10% yuqori — chegaradan past, e'tiroz yo'q.
        assert_eq!(compare(1320.0, Some(r)), Verdict::Inside);
        // 50% yuqori.
        assert!(matches!(compare(1800.0, Some(r)), Verdict::Above { .. }));
        // Yarim narx — shubhali va bu ham aytiladi.
        assert!(matches!(compare(500.0, Some(r)), Verdict::Below { .. }));
    }

    /// Nom bo'yicha izlash yaqin, lekin boshqa materialni tortmaydi.
    #[test]
    fn a_similar_name_is_not_the_same_material() {
        let rows = vec![
            row("Sement M400", "kg", 1000.0),
            row("Sementli qorishma", "m3", 400_000.0),
            row("G'isht qizil M150", "dona", 900.0),
        ];
        let index = Index::build(&rows);
        let found = find_with(&rows, &index, "Sement M400", "", "kg");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Sement M400");

        // Birlik boshqa — solishtirilmaydi.
        assert!(find_with(&rows, &index, "Sement M400", "", "tonna").is_empty());

        // Umuman yo'q material.
        assert!(find_with(&rows, &index, "Armatura A500C", "", "kg").is_empty());

        // Kod aniq moslik beradi va nomga qaramaydi.
        let mut coded = rows.clone();
        coded[2].code = "GOST-530".into();
        let coded_index = Index::build(&coded);
        let found = find_with(&coded, &coded_index, "boshqa nom", "gost 530", "");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "G'isht qizil M150");
    }

    /// Indeks bilan izlash indekssiz izlash bilan bir xil natija beradi.
    ///
    /// Tezlik uchun qilingan indeks natijani o'zgartirmasligi kerak —
    /// aks holda ekrandagi son bilan hisobotdagi son ajralib qolardi.
    #[test]
    fn the_index_finds_exactly_what_a_full_scan_would() {
        let names = [
            "Sement M400",
            "Sement M500",
            "Sementli qorishma",
            "G'isht qizil M150",
            "G'isht silikat",
            "Armatura A500C d12",
            "Armatura A500C d16",
            "Qum karyer",
        ];
        let rows: Vec<PriceRow> = names
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let mut r = row(
                    n,
                    if i % 2 == 0 { "kg" } else { "" },
                    1000.0 + i as f64 * 10.0,
                );
                if i == 5 {
                    r.code = "A500-12".into();
                }
                r
            })
            .collect();
        let index = Index::build(&rows);
        assert_eq!(index.len(), rows.len());

        // Indekssiz hisob: shu yerda mustaqil yozilgan sodda variant.
        // U har qatorni ko'rib chiqadi va indeksning kodiga tegmaydi —
        // shundagina solishtirish ma'noli bo'ladi.
        let brute = |name: &str, code: &str, unit: &str| -> Vec<String> {
            let unit_key = normal(unit);
            let fits = |r: &PriceRow| {
                unit_key.is_empty() || r.unit.trim().is_empty() || normal(&r.unit) == unit_key
            };
            let code_key = normal(code);
            let mut out: Vec<&PriceRow> = if !code_key.is_empty() {
                rows.iter()
                    .filter(|r| !r.code.trim().is_empty() && normal(&r.code) == code_key)
                    .collect()
            } else {
                Vec::new()
            };
            if out.is_empty() {
                let name_key = normal(name);
                if !name_key.is_empty() {
                    out = rows
                        .iter()
                        .filter(|r| normal(&r.name) == name_key && fits(r))
                        .collect();
                    if out.is_empty() {
                        let want = words(name);
                        if !want.is_empty() {
                            out = rows
                                .iter()
                                .filter(|r| {
                                    let have = words(&r.name);
                                    fits(r)
                                        && !have.is_empty()
                                        && want.iter().filter(|w| have.contains(w)).count() * 2
                                            > want.len()
                                })
                                .collect();
                        }
                    }
                }
            }
            let mut names: Vec<String> = out.iter().map(|r| r.name.clone()).collect();
            names.sort();
            names
        };

        for (name, code, unit) in [
            ("Sement M400", "", "kg"),
            ("Sement M400", "", ""),
            ("Armatura A500C d12", "", ""),
            ("Armatura A500C d12", "A500-12", ""),
            ("G'isht qizil M150", "", "kg"),
            ("Yo'q material", "", ""),
            ("", "", ""),
        ] {
            let mut fast: Vec<String> = find_with(&rows, &index, name, code, unit)
                .iter()
                .map(|r| r.name.clone())
                .collect();
            fast.sort();
            assert_eq!(fast, brute(name, code, unit), "{name} / {code} / {unit}");
        }

        // Boshqa ro'yxatdan tuzilgan indeks bilan taxmin qilinmaydi.
        let other = Index::build(&rows[..3]);
        assert!(find_with(&rows, &other, "Sement M400", "", "").is_empty());
    }

    /// Ikkita nuqtadan yo'nalish chiqarilmaydi.
    #[test]
    fn a_trend_needs_enough_points() {
        let d = |m: u32, day: u32| NaiveDate::from_ymd_opt(2026, m, day).unwrap();
        assert!(trend(&[(d(1, 1), 100.0), (d(6, 1), 130.0)]).is_none());

        // Uch nuqta: besh oyda 30% o'sish ≈ oyiga 6%.
        let t = trend(&[(d(1, 1), 100.0), (d(3, 1), 115.0), (d(6, 1), 130.0)]).expect("tendensiya");
        assert_eq!(t.points, 3);
        assert!(t.rising());
        assert!((t.per_month - 6.0).abs() < 0.6, "{}", t.per_month);
        assert_eq!(t.first, 100.0);
        assert_eq!(t.last, 130.0);

        // Bir kunda bo'lgan kirimlardan yo'nalish chiqmaydi.
        assert!(trend(&[(d(1, 1), 100.0), (d(1, 1), 110.0), (d(1, 1), 120.0)]).is_none());
        // Narxi tushayotgan material.
        let t = trend(&[(d(1, 1), 130.0), (d(3, 1), 120.0), (d(6, 1), 100.0)]).expect("tendensiya");
        assert!(!t.rising());
    }
}
