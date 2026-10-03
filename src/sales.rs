//! Sotuv bo'limining hisob-kitobi (XIX–XX).
//!
//! Ekranlar bu yerdagi funksiyalardan foydalanadi, o'zi hisoblamaydi —
//! shunda shaxmatkadagi rang, ko'rsatkichlar va shartnoma kartochkasi
//! bir-biriga zid chiqmaydi.

use crate::domain::{Deal, DealStatus, PayKind, Payment, Unit, UnitStatus};
use chrono::{Datelike, NaiveDate};

/// Bitta shartnomaning pul holati.
#[derive(Debug, Clone, Default)]
pub struct DealState {
    /// Grafik bo'yicha jami reja.
    pub planned: f64,
    /// Haqiqatda tushgan pul.
    pub paid: f64,
    /// Shartnoma summasidan qolgan qarz.
    pub remaining: f64,
    /// Muddati kelgan, ammo to'lanmagan summa.
    pub overdue: f64,
    /// Keyingi to'lov sanasi.
    pub next_due: Option<NaiveDate>,
    /// Grafikdagi qatorlar soni.
    pub rows: usize,
    /// Grafik summasi shartnoma summasiga to'g'ri kelmaydi.
    pub schedule_mismatch: bool,
}

/// TZ XX: shartnoma bo'yicha reja va faktni solishtiradi.
///
/// Qarz shartnoma summasidan hisoblanadi, grafikdan emas — grafik
/// to'ldirilmagan bo'lsa ham qarz to'g'ri ko'rinadi.
pub fn deal_state(deal: &Deal, payments: &[Payment], today: NaiveDate) -> DealState {
    let mine: Vec<&Payment> = payments.iter().filter(|p| p.deal_id == deal.id).collect();
    let planned: f64 = mine.iter().map(|p| p.planned).sum();
    let paid: f64 = mine.iter().map(|p| p.paid).sum();
    let total = deal.total();
    let overdue: f64 = mine
        .iter()
        .filter(|p| p.due <= today)
        .map(|p| (p.planned - p.paid).max(0.0))
        .sum();
    let next_due = mine
        .iter()
        .filter(|p| p.paid + 0.01 < p.planned)
        .map(|p| p.due)
        .min();

    DealState {
        planned,
        paid,
        remaining: (total - paid).max(0.0),
        overdue,
        next_due,
        rows: mine.len(),
        // Bekor qilingan shartnomada grafik ataylab bo'sh qolishi mumkin.
        schedule_mismatch: deal.status != DealStatus::Cancelled
            && !mine.is_empty()
            && (planned - total).abs() > 1.0,
    }
}

/// Obyekt bo'yicha sotuv xulosasi.
#[derive(Debug, Clone, Default)]
pub struct SalesSummary {
    pub units: usize,
    pub free: usize,
    pub reserved: usize,
    pub sold: usize,
    pub area_total: f64,
    pub area_free: f64,
    /// Barcha birliklarning kataloq narxi bo'yicha qiymati.
    pub value_total: f64,
    /// Bekor qilinmagan shartnomalar summasi.
    pub contracted: f64,
    pub received: f64,
    pub debt: f64,
    pub overdue: f64,
    /// Sotilgan maydonning o'rtacha 1 m² narxi.
    pub avg_price_m2: f64,
}

pub fn sales_summary(
    units: &[Unit],
    deals: &[Deal],
    payments: &[Payment],
    today: NaiveDate,
) -> SalesSummary {
    let mut s = SalesSummary {
        units: units.len(),
        ..Default::default()
    };
    for u in units {
        s.area_total += u.area;
        s.value_total += u.price();
        match unit_status(u, deals) {
            UnitStatus::Free => {
                s.free += 1;
                s.area_free += u.area;
            }
            UnitStatus::Reserved => s.reserved += 1,
            UnitStatus::Contract | UnitStatus::Sold => s.sold += 1,
            UnitStatus::Unavailable => {}
        }
    }

    let mut sold_area = 0.0;
    for d in deals.iter().filter(|d| d.status != DealStatus::Cancelled) {
        s.contracted += d.total();
        let st = deal_state(d, payments, today);
        s.received += st.paid;
        s.debt += st.remaining;
        s.overdue += st.overdue;
        if let Some(u) = units.iter().find(|u| u.id == d.unit_id) {
            sold_area += u.area;
        }
    }
    if sold_area > 0.0 {
        s.avg_price_m2 = s.contracted / sold_area;
    }
    s
}

/// Birlikning holati — amaldagi shartnomadan kelib chiqadi.
///
/// Qoida **bitta joyda** turadi: shaxmatka, qavatlar kesimi, yig'indi,
/// hisobot, eksport va portfel shu funksiyadan o'qiydi. Avval ba'zi joy
/// yozuvdagi holatni, ba'zi joy shartnomadan hisoblangan holatni olardi va
/// bitta ekranda ikki xil son chiqib turardi.
pub fn unit_status(unit: &Unit, deals: &[Deal]) -> UnitStatus {
    // «Sotuvda emas» — odamning qarori; shartnoma uni o'zgartirmaydi.
    if unit.status == UnitStatus::Unavailable {
        return UnitStatus::Unavailable;
    }
    match active_deal(deals, unit.id).map(|d| d.status) {
        Some(DealStatus::Reserved) => UnitStatus::Reserved,
        Some(DealStatus::Signed) => UnitStatus::Contract,
        Some(DealStatus::Completed) => UnitStatus::Sold,
        // Amaldagi shartnoma yo'q — bekor qilingan yoki umuman bo'lmagan.
        // Avval bekor qilingan shartnoma birlikni bo'shatmasdi: `active_deal`
        // uni chetlab o'tar, holat esa «Shartnoma» bo'lib qolaverardi.
        Some(DealStatus::Cancelled) | None => UnitStatus::Free,
    }
}

/// Birlikning amaldagi shartnomasi — bekor qilinganlari hisobga olinmaydi.
pub fn active_deal(deals: &[Deal], unit_id: i64) -> Option<&Deal> {
    deals
        .iter()
        .filter(|d| d.unit_id == unit_id && d.status != DealStatus::Cancelled)
        .max_by_key(|d| d.date)
}

/// Shartnoma shartlaridan to'lov grafigini quradi.
///
/// Birinchi qator — boshlang'ich to'lov (shartnoma sanasida), qolgani teng
/// oylik ulushlar. Yaxlitlash qoldig'i oxirgi oyga qo'shiladi, shunda grafik
/// summasi shartnoma summasiga tiyin-tiyin to'g'ri keladi.
pub fn build_schedule(deal: &Deal) -> Vec<Payment> {
    let total = deal.total();
    let mut rows = Vec::new();
    let pre = deal.prepayment.clamp(0.0, total);
    if pre > 0.0 || deal.months <= 0 {
        rows.push(row(
            deal,
            deal.date,
            if deal.months <= 0 { total } else { pre },
        ));
    }
    if deal.months <= 0 {
        return rows;
    }
    let rest = total - pre;
    if rest <= 0.0 {
        return rows;
    }
    // Oylik ulush 1000 gacha yaxlitlanadi — kassa uchun qulay son.
    let raw = rest / deal.months as f64;
    let monthly = (raw / 1000.0).floor() * 1000.0;
    let mut left = rest;
    for i in 1..=deal.months {
        let last = i == deal.months;
        let amount = if last { left } else { monthly.min(left) };
        left -= amount;
        rows.push(row(deal, add_months(deal.date, i), amount));
    }
    rows
}

/// Grafikni qayta qurish rejasi.
#[derive(Debug, Default, Clone)]
pub struct Rebuild {
    /// O'chiriladigan qatorlar — faqat to'lov tushmaganlari.
    pub remove: Vec<i64>,
    /// Qo'shiladigan yangi qatorlar.
    pub add: Vec<Payment>,
    /// Saqlanib qolgan, allaqachon tushgan summa.
    pub kept: f64,
    /// Saqlanib qolgan qatorlar soni.
    pub kept_rows: usize,
}

/// Grafikni qayta quradi va **tushgan pulni saqlaydi**.
///
/// To'lov tushgan qator — fakt: shartnoma sharti o'zgargani uni o'chirish
/// sababi emas. Avval tugma butun grafikni o'chirib, rejani noldan
/// yozardi — kvitansiya raqamlari ham, tushgan summalar ham yo'qolardi va
/// qarz yana to'liq summaga chiqib ketardi.
///
/// Endi: to'lov tushgan qatorlar joyida qoladi, qolgan summaga
/// (shartnoma summasi minus tushgani) yangi grafik tuziladi.
pub fn rebuild_schedule(deal: &Deal, payments: &[Payment]) -> Rebuild {
    let mut out = Rebuild::default();
    for p in payments.iter().filter(|p| p.deal_id == deal.id) {
        if p.paid > 0.0 {
            out.kept += p.paid;
            out.kept_rows += 1;
        } else {
            out.remove.push(p.id);
        }
    }
    let rest = (deal.total() - out.kept).max(0.0);
    if rest <= 0.0 {
        return out;
    }
    let mut plan = deal.clone();
    // Qolgan summa uchun grafik: `total()` = narx - chegirma.
    plan.price = rest + deal.discount;
    // Tushgan pul boshlang'ich to'lov o'rnini bosadi — uni ikkinchi marta
    // rejalashtirmaymiz.
    plan.prepayment = (deal.prepayment - out.kept).max(0.0);
    out.add = build_schedule(&plan);
    out
}

fn row(deal: &Deal, due: NaiveDate, planned: f64) -> Payment {
    Payment {
        id: 0,
        project_id: deal.project_id,
        deal_id: deal.id,
        due,
        planned,
        paid: 0.0,
        paid_date: None,
        kind: deal.pay_kind,
        document: String::new(),
        note: String::new(),
    }
}

/// Sanaga oy qo'shadi. Oy oxiri qisqa bo'lsa — o'sha oyning oxirgi kuni.
pub fn add_months(d: NaiveDate, months: i64) -> NaiveDate {
    let total = d.year() as i64 * 12 + d.month0() as i64 + months;
    let year = total.div_euclid(12) as i32;
    let month = total.rem_euclid(12) as u32 + 1;
    let mut day = d.day();
    loop {
        if let Some(x) = NaiveDate::from_ymd_opt(year, month, day) {
            return x;
        }
        day -= 1;
        if day == 0 {
            return d;
        }
    }
}

/// To'lov turlari kesimidagi summa — sotuv tahlili uchun.
pub fn by_pay_kind(deals: &[Deal]) -> Vec<(PayKind, f64, usize)> {
    PayKind::ALL
        .iter()
        .map(|k| {
            let mine: Vec<&Deal> = deals
                .iter()
                .filter(|d| d.pay_kind == *k && d.status != DealStatus::Cancelled)
                .collect();
            (*k, mine.iter().map(|d| d.total()).sum(), mine.len())
        })
        .collect()
}

// ================================================================ Qarz muddati

/// Qarzning kechikish guruhi (TZ XX).
///
/// Qarz «bor yoki yo'q» degan savol emas: bugun muddati kelgan to'lov
/// bilan uch oylik qarz bir xil emas va ular bilan ishlash ham har xil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bucket {
    /// Muddati hali kelmagan.
    Future,
    /// 1-30 kun.
    Days30,
    /// 31-60 kun.
    Days60,
    /// 61-90 kun.
    Days90,
    /// 90 kundan ortiq.
    Over90,
}

impl Bucket {
    pub const ALL: [Bucket; 5] = [
        Bucket::Future,
        Bucket::Days30,
        Bucket::Days60,
        Bucket::Days90,
        Bucket::Over90,
    ];

    /// Kechikish kunidan guruh.
    pub fn of(days: i64) -> Bucket {
        match days {
            d if d <= 0 => Bucket::Future,
            d if d <= 30 => Bucket::Days30,
            d if d <= 60 => Bucket::Days60,
            d if d <= 90 => Bucket::Days90,
            _ => Bucket::Over90,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Bucket::Future => crate::i18n::t("bk_future"),
            Bucket::Days30 => crate::i18n::t("bk_30"),
            Bucket::Days60 => crate::i18n::t("bk_60"),
            Bucket::Days90 => crate::i18n::t("bk_90"),
            Bucket::Over90 => crate::i18n::t("bk_over"),
        }
    }
}

/// Bitta shartnoma bo'yicha qarz holati.
#[derive(Debug, Clone, PartialEq)]
pub struct Aging {
    pub deal_id: i64,
    pub unit_id: i64,
    pub number: String,
    pub client: String,
    pub manager: String,
    pub total: f64,
    pub paid: f64,
    /// Muddati o'tgan qarz.
    pub overdue: f64,
    /// Eng eski to'lanmagan to'lov necha kun kechikkan.
    pub days: i64,
    pub bucket: Bucket,
}

/// TZ XX: qarzlarni kechikish muddati bo'yicha guruhlaydi.
///
/// Faqat **qarzi borlari** qaytadi: to'liq to'langan shartnoma ro'yxatda
/// turishi kerak emas. Bekor qilingan shartnoma ham chiqmaydi — undan
/// pul kutilmaydi.
pub fn aging(deals: &[Deal], payments: &[Payment], today: NaiveDate) -> Vec<Aging> {
    let mut out: Vec<Aging> = deals
        .iter()
        .filter(|d| d.status != DealStatus::Cancelled)
        .filter_map(|d| {
            let state = deal_state(d, payments, today);
            if state.remaining <= 0.01 {
                return None;
            }
            // Eng eski to'lanmagan to'lov kechikishni belgilaydi.
            let days = payments
                .iter()
                .filter(|p| p.deal_id == d.id && p.paid + 0.01 < p.planned && p.due <= today)
                .map(|p| (today - p.due).num_days())
                .max()
                .unwrap_or(0);
            Some(Aging {
                deal_id: d.id,
                unit_id: d.unit_id,
                number: d.number.clone(),
                client: d.client.clone(),
                manager: d.manager.clone(),
                total: d.total(),
                paid: state.paid,
                overdue: state.overdue,
                days,
                bucket: Bucket::of(days),
            })
        })
        .collect();
    // Eng uzoq kechikkanlari oldinda.
    out.sort_by(|a, b| b.days.cmp(&a.days).then(b.overdue.total_cmp(&a.overdue)));
    out
}

/// Guruh bo'yicha yig'indi: nechta shartnoma va qancha summa.
pub fn aging_totals(rows: &[Aging]) -> Vec<(Bucket, usize, f64)> {
    Bucket::ALL
        .iter()
        .map(|b| {
            let mine: Vec<&Aging> = rows.iter().filter(|r| r.bucket == *b).collect();
            let sum: f64 = mine
                .iter()
                .map(|r| {
                    if *b == Bucket::Future {
                        r.total - r.paid
                    } else {
                        r.overdue
                    }
                })
                .sum();
            (*b, mine.len(), sum)
        })
        .collect()
}

// ================================================================ Voronka

/// Sotuv voronkasi (TZ XIX).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Funnel {
    pub total: usize,
    pub free: usize,
    pub reserved: usize,
    pub contracted: usize,
    pub sold: usize,
    /// Sotuvdan chiqarilganlar.
    pub off: usize,
    /// Sotilgan maydon va jami maydon.
    pub area_sold: f64,
    pub area_total: f64,
}

impl Funnel {
    /// Sotilgan ulush, foizda. Kvartira bo'lmasa — nol.
    pub fn sold_pct(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.sold + self.contracted) as f64 * 100.0 / self.total as f64
        }
    }
}

/// TZ XIX: kvartiralar holati bo'yicha voronka.
///
/// Holat `unit_status` dan olinadi — butun dastur bilan bitta qoidadan.
pub fn funnel(units: &[Unit], deals: &[Deal]) -> Funnel {
    let mut f = Funnel {
        total: units.len(),
        area_total: units.iter().map(|u| u.area).sum(),
        ..Default::default()
    };
    for u in units {
        match unit_status(u, deals) {
            UnitStatus::Free => f.free += 1,
            UnitStatus::Reserved => f.reserved += 1,
            UnitStatus::Contract => {
                f.contracted += 1;
                f.area_sold += u.area;
            }
            UnitStatus::Sold => {
                f.sold += 1;
                f.area_sold += u.area;
            }
            UnitStatus::Unavailable => f.off += 1,
        }
    }
    f
}

// ================================================================ Bron va narx

/// Bron muddati: shuncha kundan keyin band qilingan kvartira bo'shatiladi.
///
/// Bron muddatsiz bo'lsa, kvartira oylab band turib qolishi mumkin — bu
/// sotuvni to'xtatadi. Bir oy — odatdagi muddat.
pub const RESERVE_DAYS: i64 = 30;

/// Muddati o'tgan bron (TZ XIX).
#[derive(Debug, Clone, PartialEq)]
pub struct StaleReserve {
    pub unit_id: i64,
    pub number: String,
    pub client: String,
    pub days: i64,
}

/// Uzoq turib qolgan bronlarni topadi.
pub fn stale_reserves(units: &[Unit], deals: &[Deal], today: NaiveDate) -> Vec<StaleReserve> {
    let mut out: Vec<StaleReserve> = units
        .iter()
        .filter_map(|u| {
            let deal = active_deal(deals, u.id)?;
            if deal.status != DealStatus::Reserved {
                return None;
            }
            let days = (today - deal.date).num_days();
            (days > RESERVE_DAYS).then(|| StaleReserve {
                unit_id: u.id,
                number: u.number.clone(),
                client: deal.client.clone(),
                days,
            })
        })
        .collect();
    out.sort_by_key(|r| std::cmp::Reverse(r.days));
    out
}

/// Menejer kesimi (TZ XIX).
#[derive(Debug, Clone, PartialEq)]
pub struct ManagerStat {
    pub manager: String,
    pub deals: usize,
    pub amount: f64,
    pub paid: f64,
    pub debt: f64,
}

/// TZ XIX: menejerlar bo'yicha sotuv.
///
/// Ism bo'sh bo'lsa alohida qator sifatida qoladi — «kim sotgani noma'lum»
/// ham javob, uni boshqa menejerga qo'shib yuborish noto'g'ri bo'lardi.
pub fn by_manager(deals: &[Deal], payments: &[Payment], today: NaiveDate) -> Vec<ManagerStat> {
    let mut out: Vec<ManagerStat> = Vec::new();
    for d in deals.iter().filter(|d| d.status != DealStatus::Cancelled) {
        let state = deal_state(d, payments, today);
        let name = d.manager.trim().to_string();
        match out.iter_mut().find(|m| m.manager == name) {
            Some(m) => {
                m.deals += 1;
                m.amount += d.total();
                m.paid += state.paid;
                m.debt += state.remaining;
            }
            None => out.push(ManagerStat {
                manager: name,
                deals: 1,
                amount: d.total(),
                paid: state.paid,
                debt: state.remaining,
            }),
        }
    }
    out.sort_by(|a, b| b.amount.total_cmp(&a.amount));
    out
}

/// Qavat bo'yicha narx va qoldiq (TZ XIX).
#[derive(Debug, Clone, PartialEq)]
pub struct FloorStat {
    pub floor: i64,
    pub units: usize,
    pub free: usize,
    /// O'rtacha kvadrat metr narxi.
    pub avg_price_m2: f64,
    pub area_free: f64,
}

/// TZ XIX: qavatlar kesimida qoldiq va narx.
pub fn by_floor(units: &[Unit], deals: &[Deal]) -> Vec<FloorStat> {
    let mut out: Vec<FloorStat> = Vec::new();
    for u in units {
        let free = unit_status(u, deals) == UnitStatus::Free;
        match out.iter_mut().find(|f| f.floor == u.floor) {
            Some(f) => {
                f.units += 1;
                if free {
                    f.free += 1;
                    f.area_free += u.area;
                }
                // O'rtacha narx qatorlar soniga qarab qayta hisoblanadi.
                f.avg_price_m2 =
                    (f.avg_price_m2 * (f.units - 1) as f64 + u.price_per_m2) / f.units as f64;
            }
            None => out.push(FloorStat {
                floor: u.floor,
                units: 1,
                free: usize::from(free),
                avg_price_m2: u.price_per_m2,
                area_free: if free { u.area } else { 0.0 },
            }),
        }
    }
    out.sort_by_key(|f| f.floor);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TZ XX: qarz kechikish muddati bo'yicha guruhlanadi va to'liq
    /// to'langan shartnoma ro'yxatga tushmaydi.
    #[test]
    fn debts_are_grouped_by_how_late_they_are() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
        let deal = |id: i64, price: f64| Deal {
            id,
            project_id: 1,
            unit_id: id,
            number: format!("D-{id}"),
            date: today - chrono::Duration::days(200),
            client: format!("Mijoz {id}"),
            phone: String::new(),
            client_doc: String::new(),
            pay_kind: PayKind::Installment,
            price,
            discount: 0.0,
            prepayment: 0.0,
            months: 6,
            status: DealStatus::Signed,
            manager: "Menejer".into(),
            note: String::new(),
        };
        let pay = |deal_id: i64, days_ago: i64, planned: f64, paid: f64| Payment {
            id: 0,
            project_id: 1,
            deal_id,
            due: today - chrono::Duration::days(days_ago),
            planned,
            paid,
            paid_date: None,
            kind: PayKind::Installment,
            document: String::new(),
            note: String::new(),
        };

        let deals = vec![deal(1, 100.0), deal(2, 100.0), deal(3, 100.0)];
        let payments = vec![
            // 1 — 100 kun kechikkan.
            pay(1, 100, 100.0, 0.0),
            // 2 — 10 kun kechikkan.
            pay(2, 10, 100.0, 0.0),
            // 3 — to'liq to'langan.
            pay(3, 40, 100.0, 100.0),
        ];

        let rows = aging(&deals, &payments, today);
        assert_eq!(rows.len(), 2, "to'langan shartnoma ro'yxatga tushdi");
        // Eng uzoq kechikkani oldinda.
        assert_eq!(rows[0].deal_id, 1);
        assert_eq!(rows[0].bucket, Bucket::Over90);
        assert_eq!(rows[1].bucket, Bucket::Days30);

        let totals = aging_totals(&rows);
        let over = totals
            .iter()
            .find(|(b, _, _)| *b == Bucket::Over90)
            .unwrap();
        assert_eq!(over.1, 1);
        assert_eq!(over.2, 100.0);
        // Guruhlar chegarasi.
        assert_eq!(Bucket::of(0), Bucket::Future);
        assert_eq!(Bucket::of(30), Bucket::Days30);
        assert_eq!(Bucket::of(31), Bucket::Days60);
        assert_eq!(Bucket::of(91), Bucket::Over90);
    }

    /// TZ XIX: voronka holatni shartnomadan oladi, kvartira yozuvidan emas.
    #[test]
    fn funnel_counts_status_from_the_deal() {
        let unit = |id: i64, status: UnitStatus| Unit {
            id,
            project_id: 1,
            block_id: 1,
            number: format!("{id}"),
            floor: 1,
            position: id,
            kind: crate::domain::UnitKind::Flat,
            rooms: 2,
            area: 50.0,
            area_living: 30.0,
            price_per_m2: 10.0,
            status,
            layout: String::new(),
            note: String::new(),
        };
        let today = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
        let mut deal = Deal {
            id: 1,
            project_id: 1,
            unit_id: 1,
            number: "D-1".into(),
            date: today,
            client: "Mijoz".into(),
            phone: String::new(),
            client_doc: String::new(),
            pay_kind: PayKind::Cash,
            price: 500.0,
            discount: 0.0,
            prepayment: 0.0,
            months: 0,
            status: DealStatus::Signed,
            manager: String::new(),
            note: String::new(),
        };

        // Kvartira «bo'sh» deb yozilgan, lekin shartnoma bor.
        let units = vec![unit(1, UnitStatus::Free), unit(2, UnitStatus::Free)];
        let f = funnel(&units, std::slice::from_ref(&deal));
        assert_eq!(f.total, 2);
        assert_eq!(f.free, 1, "shartnomali kvartira bo'sh deb sanaldi");
        assert_eq!(f.contracted, 1);
        assert_eq!(f.area_sold, 50.0);
        assert_eq!(f.sold_pct(), 50.0);

        // Bekor qilingan shartnoma kvartirani bo'shatadi.
        deal.status = DealStatus::Cancelled;
        let f = funnel(&units, std::slice::from_ref(&deal));
        assert_eq!(f.free, 2);
        assert_eq!(f.sold_pct(), 0.0);

        // Kvartira yo'q bo'lsa foiz nolga bo'linmaydi.
        assert_eq!(funnel(&[], &[]).sold_pct(), 0.0);
    }

    /// TZ XIX: uzoq turib qolgan bron ko'rinadi.
    #[test]
    fn stale_reservations_are_found() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
        let unit = Unit {
            id: 1,
            project_id: 1,
            block_id: 1,
            number: "12".into(),
            floor: 3,
            position: 1,
            kind: crate::domain::UnitKind::Flat,
            rooms: 2,
            area: 60.0,
            area_living: 40.0,
            price_per_m2: 10.0,
            status: UnitStatus::Reserved,
            layout: String::new(),
            note: String::new(),
        };
        let mut deal = Deal {
            id: 1,
            project_id: 1,
            unit_id: 1,
            number: "D-1".into(),
            date: today - chrono::Duration::days(RESERVE_DAYS + 5),
            client: "Mijoz".into(),
            phone: String::new(),
            client_doc: String::new(),
            pay_kind: PayKind::Cash,
            price: 600.0,
            discount: 0.0,
            prepayment: 0.0,
            months: 0,
            status: DealStatus::Reserved,
            manager: String::new(),
            note: String::new(),
        };

        let rows = stale_reserves(
            std::slice::from_ref(&unit),
            std::slice::from_ref(&deal),
            today,
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].days, RESERVE_DAYS + 5);

        // Yangi bron hali muddatida.
        deal.date = today - chrono::Duration::days(3);
        assert!(stale_reserves(
            std::slice::from_ref(&unit),
            std::slice::from_ref(&deal),
            today
        )
        .is_empty());

        // Imzolangan shartnoma bron emas.
        deal.date = today - chrono::Duration::days(100);
        deal.status = DealStatus::Signed;
        assert!(stale_reserves(&[unit], &[deal], today).is_empty());
    }

    fn deal(price: f64, prepayment: f64, months: i64) -> Deal {
        Deal {
            id: 1,
            project_id: 1,
            unit_id: 1,
            number: "S-1".into(),
            date: NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
            client: String::new(),
            phone: String::new(),
            client_doc: String::new(),
            pay_kind: PayKind::Installment,
            price,
            discount: 0.0,
            prepayment,
            months,
            status: DealStatus::Signed,
            manager: String::new(),
            note: String::new(),
        }
    }

    #[test]
    fn schedule_sums_to_contract_total() {
        let d = deal(120_000_000.0, 30_000_000.0, 12);
        let rows = build_schedule(&d);
        assert_eq!(rows.len(), 13, "boshlang'ich + 12 oy");
        let sum: f64 = rows.iter().map(|r| r.planned).sum();
        assert!((sum - d.total()).abs() < 0.01, "grafik summasi: {sum}");
        assert_eq!(rows[0].planned, 30_000_000.0);
    }

    #[test]
    fn cash_deal_has_single_row() {
        let mut d = deal(90_000_000.0, 0.0, 0);
        d.pay_kind = PayKind::Cash;
        let rows = build_schedule(&d);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].planned, 90_000_000.0);
    }

    /// 31-yanvarga oy qo'shilganda 31-fevral bo'lmaydi — oxirgi kunga tushadi.
    #[test]
    fn month_end_does_not_overflow() {
        let jan31 = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        assert_eq!(
            add_months(jan31, 1),
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
        );
        assert_eq!(
            add_months(jan31, 12),
            NaiveDate::from_ymd_opt(2027, 1, 31).unwrap()
        );
    }

    #[test]
    fn overdue_counts_only_due_rows() {
        let d = deal(120_000_000.0, 30_000_000.0, 3);
        let mut rows = build_schedule(&d);
        for (i, r) in rows.iter_mut().enumerate() {
            r.id = i as i64 + 1;
        }
        // Bugun — ikkinchi to'lovdan keyin, uchinchisidan oldin.
        let today = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let st = deal_state(&d, &rows, today);
        assert_eq!(st.rows, 4);
        assert!((st.planned - d.total()).abs() < 0.01);
        assert_eq!(st.paid, 0.0);
        // Muddati kelgan ikki qator: 31.01 va 28.02.
        let due_sum = rows[0].planned + rows[1].planned;
        assert!(
            (st.overdue - due_sum).abs() < 0.01,
            "overdue: {}",
            st.overdue
        );
        assert_eq!(st.next_due, Some(d.date));
        assert!(!st.schedule_mismatch);
    }
}
