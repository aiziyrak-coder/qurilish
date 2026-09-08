//! Kirish-chiqish belgilarini kunga aylantirish (TZ XIII.4, XIII.5, XIII.6).
//!
//! Belgi telefondan keladi: ishchi maydonchaga kelganda «kirdim»,
//! ketayotganda «chiqdim» deydi. Bu yerda shu belgilar **kunga**
//! yig'iladi va tabel bilan solishtiriladi.
//!
//! Uchta qoida ishni halol tutadi:
//!
//! 1. **Belgi tabelning o'rnini bosmaydi.** Soatni odam yozadi, belgi esa
//!    mustaqil dalil bo'ladi. Farq chiqsa — savol tug'iladi, avtomatik
//!    tuzatish bo'lmaydi.
//! 2. **Juftlanmagan belgi soat bermaydi.** «Kirdim» bor, «chiqdim» yo'q
//!    bo'lsa, kun *ochiq* deb belgilanadi va soat hisoblanmaydi: qancha
//!    ishlagani noma'lum, taxmin qilinmaydi.
//! 3. **Koordinata bo'lmasa joy bo'yicha hukm ham yo'q.**

use crate::domain::{Attendance, InOut, TimesheetEntry, Worker};
use crate::geo::{self, Fence, Verdict};
use chrono::{NaiveDate, NaiveDateTime};

/// Bir ishchining bir kuni.
#[derive(Debug, Clone)]
pub struct Day {
    pub worker: String,
    pub date: NaiveDate,
    pub first_in: Option<NaiveDateTime>,
    pub last_out: Option<NaiveDateTime>,
    /// Juftlangan oraliqlar yig'indisi, soatda.
    pub hours: f64,
    /// Kun ichidagi belgilar soni.
    pub marks: usize,
    /// «Kirdim» bor, «chiqdim» yo'q.
    pub open: bool,
    /// Joy bo'yicha eng yomon hukm.
    pub verdict: Verdict,
    /// Belgilardan bittasi QR yorliq bilan qo'yilganmi.
    pub by_qr: bool,
}

impl Day {
    /// Ekranda ko'rsatiladigan soat: ochiq kun soat bermaydi.
    pub fn hours_label(&self) -> String {
        if self.open && self.hours <= 0.0 {
            return crate::i18n::t("at_open").to_string();
        }
        format!("{:.1}", self.hours)
    }
}

/// Belgilarni kunlarga yig'adi.
///
/// Kun chegarasi — kalendar kuni. Tungi smena yarim tundan o'tsa ikki
/// kunga bo'linadi va bu ataylab: tabel ham kalendar kuni bo'yicha
/// yuritiladi, ikkisi bir xil o'lchovda bo'lishi kerak.
pub fn days(records: &[Attendance], fence: Option<Fence>) -> Vec<Day> {
    // (ishchi, kun) bo'yicha guruh; ichida vaqt bo'yicha tartib.
    let mut groups: std::collections::BTreeMap<(String, NaiveDate), Vec<&Attendance>> =
        Default::default();
    for a in records {
        groups
            .entry((a.worker.clone(), a.at.date()))
            .or_default()
            .push(a);
    }

    let mut out: Vec<Day> = groups
        .into_iter()
        .map(|((worker, date), mut marks)| {
            marks.sort_by_key(|a| a.at);

            let mut hours = 0.0f64;
            let mut open_in: Option<NaiveDateTime> = None;
            let mut first_in = None;
            let mut last_out = None;
            let mut verdict = Verdict::Unknown;
            let mut by_qr = false;

            for a in &marks {
                if a.source == "qr" {
                    by_qr = true;
                }
                // Joy bo'yicha eng yomon holat saqlanadi: kun ichida bitta
                // belgi tashqarida bo'lsa ham bu ko'rinishi kerak.
                if let Some(fence) = fence {
                    if let Some(p) = geo::parse(&a.gps) {
                        let v = fence.check(p);
                        if v.outside() || matches!(verdict, Verdict::Unknown) {
                            verdict = v;
                        }
                    }
                }
                match a.kind {
                    InOut::In => {
                        if first_in.is_none() {
                            first_in = Some(a.at);
                        }
                        // Ketma-ket ikkita «kirdim» — birinchisi ochiq
                        // qoladi, ikkinchisi hisobni buzmasligi uchun
                        // e'tiborga olinmaydi.
                        if open_in.is_none() {
                            open_in = Some(a.at);
                        }
                    }
                    InOut::Out => {
                        last_out = Some(a.at);
                        if let Some(start) = open_in.take() {
                            let d = (a.at - start).num_minutes() as f64 / 60.0;
                            if d > 0.0 {
                                hours += d;
                            }
                        }
                    }
                }
            }

            Day {
                worker,
                date,
                first_in,
                last_out,
                hours,
                marks: marks.len(),
                open: open_in.is_some(),
                verdict,
                by_qr,
            }
        })
        .collect();

    out.sort_by(|a, b| b.date.cmp(&a.date).then(a.worker.cmp(&b.worker)));
    out
}

/// Tabel bilan belgi orasidagi farq shu chegaradan oshsa e'tiroz bo'ladi.
///
/// Bir soat — kelib-ketish, tushlik va telefonni kech ochish uchun
/// yetarli zaxira. Undan katta farq esa allaqachon savol tug'diradi.
pub const HOUR_TOLERANCE: f64 = 1.0;

/// Tabel va belgi orasidagi farq.
#[derive(Debug, Clone, PartialEq)]
pub enum Mismatch {
    /// Tabelda soat bor, belgi umuman yo'q.
    NoMarks {
        worker: String,
        date: NaiveDate,
        timesheet: f64,
    },
    /// Belgi bor, tabelda kun bo'sh.
    NoTimesheet {
        worker: String,
        date: NaiveDate,
        marked: f64,
    },
    /// Ikkalasi bor, lekin soat farq qiladi.
    Hours {
        worker: String,
        date: NaiveDate,
        marked: f64,
        timesheet: f64,
    },
    /// Belgi obyekt doirasidan tashqarida qo'yilgan.
    Outside {
        worker: String,
        date: NaiveDate,
        distance: f64,
    },
    /// Kun ochiq qolgan: chiqish belgilanmagan.
    Open { worker: String, date: NaiveDate },
}

impl Mismatch {
    pub fn worker(&self) -> &str {
        match self {
            Mismatch::NoMarks { worker, .. }
            | Mismatch::NoTimesheet { worker, .. }
            | Mismatch::Hours { worker, .. }
            | Mismatch::Outside { worker, .. }
            | Mismatch::Open { worker, .. } => worker,
        }
    }

    pub fn date(&self) -> NaiveDate {
        match self {
            Mismatch::NoMarks { date, .. }
            | Mismatch::NoTimesheet { date, .. }
            | Mismatch::Hours { date, .. }
            | Mismatch::Outside { date, .. }
            | Mismatch::Open { date, .. } => *date,
        }
    }

    /// Jiddiy farq — hisob-kitobga ta'sir qiladi.
    pub fn severe(&self) -> bool {
        matches!(
            self,
            Mismatch::Hours { .. } | Mismatch::NoTimesheet { .. } | Mismatch::Outside { .. }
        )
    }

    pub fn text(&self) -> String {
        use crate::i18n::t;
        match self {
            Mismatch::NoMarks { timesheet, .. } => {
                format!("{} ({timesheet:.1} {})", t("at_no_marks"), t("hours_short"))
            }
            Mismatch::NoTimesheet { marked, .. } => {
                format!(
                    "{} ({marked:.1} {})",
                    t("at_no_timesheet"),
                    t("hours_short")
                )
            }
            Mismatch::Hours {
                marked, timesheet, ..
            } => format!(
                "{}: {} {marked:.1} · {} {timesheet:.1}",
                t("at_hours_differ"),
                t("at_marked"),
                t("at_in_timesheet")
            ),
            Mismatch::Outside { distance, .. } => {
                format!("{} — {}", t("at_outside"), geo::meters(*distance))
            }
            Mismatch::Open { .. } => t("at_open_day").to_string(),
        }
    }
}

/// Belgilangan kunlarni tabel bilan solishtiradi.
///
/// Solishtirish **faqat belgi kelgan kunlarda** o'tkaziladi. Belgi
/// yo'qligi hali qoidabuzarlik emas: telefon bo'lmasligi, batareya
/// o'tirishi mumkin. Shuning uchun «belgi yo'q» faqat shu ishchining
/// boshqa kunlarida belgi bor bo'lsa aytiladi — ya'ni u odatda
/// belgilaydi, bu kuni esa belgilamagan.
pub fn compare(days: &[Day], timesheet: &[TimesheetEntry], workers: &[Worker]) -> Vec<Mismatch> {
    let name = |id: i64| {
        workers
            .iter()
            .find(|w| w.id == id)
            .map(|w| w.name.clone())
            .unwrap_or_default()
    };
    let hours_of = |worker: &str, date: NaiveDate| -> Option<f64> {
        timesheet
            .iter()
            .find(|e| e.date == date && name(e.worker_id) == worker)
            .map(|e| e.hours)
    };

    let mut out = Vec::new();
    let marked_days: std::collections::BTreeSet<(String, NaiveDate)> =
        days.iter().map(|d| (d.worker.clone(), d.date)).collect();
    let markers: std::collections::BTreeSet<String> =
        days.iter().map(|d| d.worker.clone()).collect();

    for d in days {
        if let Verdict::Outside { distance, .. } = d.verdict {
            out.push(Mismatch::Outside {
                worker: d.worker.clone(),
                date: d.date,
                distance,
            });
        }
        if d.open {
            out.push(Mismatch::Open {
                worker: d.worker.clone(),
                date: d.date,
            });
            // Ochiq kunda soat noma'lum — solishtirilmaydi.
            continue;
        }
        match hours_of(&d.worker, d.date) {
            None if d.hours > 0.0 => out.push(Mismatch::NoTimesheet {
                worker: d.worker.clone(),
                date: d.date,
                marked: d.hours,
            }),
            Some(ts) if (ts - d.hours).abs() > HOUR_TOLERANCE => out.push(Mismatch::Hours {
                worker: d.worker.clone(),
                date: d.date,
                marked: d.hours,
                timesheet: ts,
            }),
            _ => {}
        }
    }

    // Tabelda bor, belgi yo'q — faqat belgilaydigan ishchilar uchun.
    for e in timesheet {
        if e.hours <= 0.0 {
            continue;
        }
        let who = name(e.worker_id);
        if who.is_empty() || !markers.contains(&who) {
            continue;
        }
        if !marked_days.contains(&(who.clone(), e.date)) {
            out.push(Mismatch::NoMarks {
                worker: who,
                date: e.date,
                timesheet: e.hours,
            });
        }
    }

    out.sort_by(|a, b| {
        b.severe()
            .cmp(&a.severe())
            .then(b.date().cmp(&a.date()))
            .then(a.worker().cmp(b.worker()))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DayKind, Shift};

    fn at(y: i32, m: u32, d: u32, h: u32, mi: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, mi, 0)
            .unwrap()
    }

    fn mark(worker: &str, when: NaiveDateTime, kind: InOut, gps: &str) -> Attendance {
        Attendance {
            id: 0,
            project_id: 1,
            worker: worker.into(),
            at: when,
            kind,
            gps: gps.into(),
            source: "list".into(),
        }
    }

    fn worker(id: i64, name: &str) -> Worker {
        Worker {
            id,
            project_id: 1,
            name: name.into(),
            position: String::new(),
            org: String::new(),
            hourly_rate: 0.0,
            active: true,
            brigade_id: None,
        }
    }

    fn ts(worker_id: i64, date: NaiveDate, hours: f64) -> TimesheetEntry {
        TimesheetEntry {
            id: 0,
            project_id: 1,
            worker_id,
            date,
            hours,
            task_id: None,
            kind: DayKind::Work,
            shift: Shift::Day,
            note: String::new(),
        }
    }

    /// Kirish va chiqish juftlanadi, soat shundan chiqadi.
    #[test]
    fn a_pair_of_marks_becomes_hours() {
        let d = NaiveDate::from_ymd_opt(2026, 9, 8).unwrap();
        let records = vec![
            mark("Alisher", at(2026, 9, 8, 8, 0), InOut::In, ""),
            mark("Alisher", at(2026, 9, 8, 12, 0), InOut::Out, ""),
            mark("Alisher", at(2026, 9, 8, 13, 0), InOut::In, ""),
            mark("Alisher", at(2026, 9, 8, 17, 30), InOut::Out, ""),
        ];
        let days = days(&records, None);
        assert_eq!(days.len(), 1);
        let day = &days[0];
        assert_eq!(day.date, d);
        // 4 soat + 4,5 soat — tushlik oralig'i hisoblanmaydi.
        assert!((day.hours - 8.5).abs() < 1e-9, "{}", day.hours);
        assert!(!day.open);
        assert_eq!(day.marks, 4);
        assert_eq!(day.first_in, Some(at(2026, 9, 8, 8, 0)));
        assert_eq!(day.last_out, Some(at(2026, 9, 8, 17, 30)));
    }

    /// Chiqish belgilanmasa soat taxmin qilinmaydi — kun ochiq qoladi.
    #[test]
    fn an_open_day_gives_no_hours() {
        let records = vec![mark("Bek", at(2026, 9, 8, 8, 0), InOut::In, "")];
        let days = days(&records, None);
        assert!(days[0].open);
        assert_eq!(days[0].hours, 0.0);
        assert_eq!(days[0].hours_label(), crate::i18n::t("at_open"));

        // Ochiq kun tabel bilan solishtirilmaydi, lekin ochiq deb aytiladi.
        let d = NaiveDate::from_ymd_opt(2026, 9, 8).unwrap();
        let m = compare(&days, &[ts(1, d, 8.0)], &[worker(1, "Bek")]);
        assert!(m.iter().any(|x| matches!(x, Mismatch::Open { .. })));
        assert!(!m.iter().any(|x| matches!(x, Mismatch::Hours { .. })));
    }

    /// Tabeldagi soat belgidan ancha farq qilsa — savol tug'iladi.
    #[test]
    fn timesheet_that_disagrees_with_the_marks_is_flagged() {
        let d = NaiveDate::from_ymd_opt(2026, 9, 8).unwrap();
        let records = vec![
            mark("Alisher", at(2026, 9, 8, 8, 0), InOut::In, ""),
            mark("Alisher", at(2026, 9, 8, 12, 0), InOut::Out, ""),
        ];
        let days = days(&records, None);
        let workers = vec![worker(1, "Alisher")];

        // 4 soat belgilangan, tabelda 10 — farq katta.
        let m = compare(&days, &[ts(1, d, 10.0)], &workers);
        assert!(
            m.iter().any(|x| matches!(x, Mismatch::Hours { .. })),
            "{m:?}"
        );

        // Yarim soatlik farq — e'tiroz emas.
        let m = compare(&days, &[ts(1, d, 4.5)], &workers);
        assert!(m.is_empty(), "{m:?}");

        // Tabelda umuman yo'q.
        let m = compare(&days, &[], &workers);
        assert!(m.iter().any(|x| matches!(x, Mismatch::NoTimesheet { .. })));
    }

    /// «Belgi yo'q» faqat odatda belgilaydigan ishchi uchun aytiladi.
    #[test]
    fn a_worker_who_never_marks_is_not_accused() {
        let d1 = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 9, 8).unwrap();
        let records = vec![
            mark("Alisher", at(2026, 9, 7, 8, 0), InOut::In, ""),
            mark("Alisher", at(2026, 9, 7, 16, 0), InOut::Out, ""),
        ];
        let days = days(&records, None);
        let workers = vec![worker(1, "Alisher"), worker(2, "Bek")];
        let sheet = vec![ts(1, d1, 8.0), ts(1, d2, 8.0), ts(2, d2, 8.0)];
        let m = compare(&days, &sheet, &workers);

        // Alisher 8-sentabrda belgilamagan — bu ko'rsatiladi.
        assert!(m
            .iter()
            .any(|x| matches!(x, Mismatch::NoMarks { worker, date, .. }
                if worker == "Alisher" && *date == d2)));
        // Bek hech qachon belgilamaydi — unga e'tiroz yo'q.
        assert!(!m.iter().any(|x| x.worker() == "Bek"), "{m:?}");
    }

    /// Obyektdan tashqarida qo'yilgan belgi ko'rinadi; geozonasiz — yo'q.
    #[test]
    fn a_mark_from_outside_is_visible_only_with_a_fence() {
        let fence = Fence {
            center: geo::Point::new(41.2995, 69.2401),
            radius: 100.0,
        };
        let records = vec![
            mark(
                "Alisher",
                at(2026, 9, 8, 8, 0),
                InOut::In,
                "41.2995,69.2401,10",
            ),
            // ~3,4 km narida.
            mark(
                "Alisher",
                at(2026, 9, 8, 16, 0),
                InOut::Out,
                "41.3300,69.2401,10",
            ),
        ];
        let with = days(&records, Some(fence));
        assert!(with[0].verdict.outside());

        let without = days(&records, None);
        assert_eq!(without[0].verdict, Verdict::Unknown);

        let m = compare(&with, &[], &[worker(1, "Alisher")]);
        assert!(m.iter().any(|x| matches!(x, Mismatch::Outside { .. })));
    }
}
