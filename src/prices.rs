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

/// Bazadan materialga mos qatorlarni topadi.
///
/// Uch bosqich: kod bo'yicha aniq moslik, keyin nom bo'yicha aniq
/// moslik, keyin so'zlarning yarmidan ko'pi mos kelishi. Shundan
/// pastga tushilmaydi: «sement» bilan «sementli qorishma» bir narsa
/// emas va ularni chalkashtirish narxni buzardi.
pub fn find<'a>(rows: &'a [PriceRow], name: &str, code: &str, unit: &str) -> Vec<&'a PriceRow> {
    let unit_ok = |r: &PriceRow| {
        // Birlik ko'rsatilmagan bo'lsa cheklamaymiz; ko'rsatilgan bo'lsa
        // u mos kelishi kerak — «tonna» va «kg» narxi solishtirilmaydi.
        unit.trim().is_empty() || r.unit.trim().is_empty() || normal(&r.unit) == normal(unit)
    };

    if !code.trim().is_empty() {
        let by_code: Vec<&PriceRow> = rows
            .iter()
            .filter(|r| !r.code.trim().is_empty() && normal(&r.code) == normal(code))
            .collect();
        if !by_code.is_empty() {
            return by_code;
        }
    }

    let target = normal(name);
    if target.is_empty() {
        return Vec::new();
    }
    let exact: Vec<&PriceRow> = rows
        .iter()
        .filter(|r| normal(&r.name) == target && unit_ok(r))
        .collect();
    if !exact.is_empty() {
        return exact;
    }

    let want = words(name);
    if want.is_empty() {
        return Vec::new();
    }
    rows.iter()
        .filter(|r| {
            if !unit_ok(r) {
                return false;
            }
            let have = words(&r.name);
            if have.is_empty() {
                return false;
            }
            let hits = want.iter().filter(|w| have.contains(w)).count();
            hits * 2 > want.len()
        })
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
        let found = find(&rows, "Sement M400", "", "kg");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Sement M400");

        // Birlik boshqa — solishtirilmaydi.
        assert!(find(&rows, "Sement M400", "", "tonna").is_empty());

        // Umuman yo'q material.
        assert!(find(&rows, "Armatura A500C", "", "kg").is_empty());

        // Kod aniq moslik beradi va nomga qaramaydi.
        let mut coded = rows.clone();
        coded[2].code = "GOST-530".into();
        let found = find(&coded, "boshqa nom", "gost 530", "");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "G'isht qizil M150");
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
