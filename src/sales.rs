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
        match u.status {
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

/// Shartnoma holatidan kelib chiqadigan birlik holati.
///
/// Shaxmatkadagi rang shartnomaga ergashishi kerak: qo'lda qo'yilgan holat
/// bilan shartnoma bir-biriga zid bo'lib qolmasin.
pub fn status_for(deal: Option<&Deal>) -> Option<UnitStatus> {
    match deal?.status {
        DealStatus::Reserved => Some(UnitStatus::Reserved),
        DealStatus::Signed => Some(UnitStatus::Contract),
        DealStatus::Completed => Some(UnitStatus::Sold),
        // Bekor qilingan shartnoma birlikni bo'shatadi.
        DealStatus::Cancelled => Some(UnitStatus::Free),
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

#[cfg(test)]
mod tests {
    use super::*;

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
