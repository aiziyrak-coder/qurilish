//! Taklif etilgan amallar — yordamchining operatsion qatlami (TZ XVIII.12–30, 44).
//!
//! Yordamchi savolga javob berish bilan cheklanmaydi: u bazadagi holatdan
//! **nima qilish kerakligini** ko'radi va tayyor qoralama taklif qiladi.
//! Amal foydalanuvchi tasdiqlagandan keyin bajariladi.
//!
//! Uchta qat'iy qoida:
//!
//! 1. **Taklif o'zi bajarilmaydi.** Ro'yxatdagi har bir qator — taklif;
//!    bajarish uchun alohida tugma bosiladi.
//! 2. **Nima bo'lishi oldindan aytiladi.** Har taklifda qaysi yozuv
//!    yaratilishi yoki o'zgarishi va qaysi sondan chiqqani ko'rsatiladi.
//! 3. **Rol hurmat qilinadi.** Amal qaysi ekranga tegsa, o'sha ekranga yozish
//!    huquqi bo'lmagan rol uni bajara olmaydi.

use crate::app::{App, Screen};
use crate::checks::StockLine;
use crate::domain::{
    ExecDocKind, ExecDocStatus, MoveKind, Priority, PurchaseStatus, QualityCheck, QualityResult,
    Request, RequestKind, RequestStatus, StockMove,
};
use crate::i18n::t;

/// Amalning turi — bajarish paytida nima qilinishini belgilaydi.
#[derive(Debug, Clone, PartialEq)]
pub enum ActionKind {
    /// Zaxiradan tushib ketgan materialga ariza ochish (TZ XVIII.12).
    CreateRequest { material_id: i64, qty: f64 },
    /// Tasdiqlangan ariza bo'yicha xarid ochish.
    CreatePurchase { request_id: i64 },
    /// Kelgan xaridni omborga kirim qilish.
    PostToStock { purchase_id: i64 },
    /// Tugallangan ishga ijro hujjati qoralamasini ochish.
    CreateExecDoc { task_id: i64, kind: ExecDocKind },
    /// Nuqsonga bartaraf etish muddatini qo'yish.
    SetDefectDeadline { check_id: i64, days: i64 },
    /// Tekshiruv nuqsoniga bartaraf etish muddatini qo'yish (TZ VII.19, 21).
    SetInspectionDeadline { inspection_id: i64, days: i64 },
    /// Muddati o'tgan ochiq naryadni yopish.
    ClosePermit { permit_id: i64 },
    /// Arizaga kelishuv marshrutini ochish.
    BuildRoute { request_id: i64 },
}

/// Yordamchining bitta taklifi.
#[derive(Debug, Clone)]
pub struct Action {
    /// Barqaror kod — takliflarni solishtirish va sinash uchun.
    pub code: &'static str,
    pub kind: ActionKind,
    /// Nima qilinadi.
    pub title: String,
    /// Qaysi sondan chiqdi — taklif asossiz ko'rinmasin.
    pub evidence: String,
    /// Bajarilgandan keyin qaysi ekranda ko'rinadi.
    pub screen: Screen,
}

/// TZ XVIII.31–40: bazadagi holatdan taklif chiqaradi.
///
/// Ro'yxat tartibi barqaror: avval ta'minot (ishni to'xtatadi), keyin
/// hujjatlar, oxirida sifat va xavfsizlik. Har bir qoida bir xil savolga
/// javob beradi: «buni hozir kim va nima qilishi kerak?».
pub fn suggest(app: &App, stock: &[StockLine]) -> Vec<Action> {
    let mut out = Vec::new();
    supply_actions(app, stock, &mut out);
    document_actions(app, &mut out);
    quality_actions(app, &mut out);
    inspection_actions(app, &mut out);
    safety_actions(app, &mut out);
    out
}

fn push(
    out: &mut Vec<Action>,
    code: &'static str,
    kind: ActionKind,
    title: String,
    evidence: String,
    screen: Screen,
) {
    out.push(Action {
        code,
        kind,
        title,
        evidence,
        screen,
    });
}

// ================================================================ Ta'minot

fn supply_actions(app: &App, stock: &[StockLine], out: &mut Vec<Action>) {
    // 1. Zaxiradan tushgan material — ariza yo'q bo'lsa taklif qilamiz.
    for l in stock.iter().filter(|l| l.below_min) {
        let Some(m) = app.materials.iter().find(|m| m.id == l.material_id) else {
            continue;
        };
        // Ochiq ariza bo'lsa takrorlamaymiz.
        let open = app.requests.iter().any(|q| {
            q.material_id == Some(m.id)
                && !matches!(q.status, RequestStatus::Closed | RequestStatus::Rejected)
        });
        if open {
            continue;
        }
        // Minimal zaxiraning ikki barobarigacha to'ldiramiz: bir barobar
        // yetarli emas, chiqim davom etadi.
        let qty = (m.min_stock * 2.0 - l.available).max(m.min_stock);
        push(
            out,
            "AC-S1",
            ActionKind::CreateRequest {
                material_id: m.id,
                qty,
            },
            format!("{} {} {}", t("ac_create_request"), m.code, m.name),
            format!(
                "{} {} · {} {}",
                t("ac_free"),
                crate::ui::materials::trim_num(l.available),
                t("ac_min"),
                crate::ui::materials::trim_num(m.min_stock)
            ),
            Screen::Requests,
        );
    }

    // 2. Tasdiqlangan ariza — xaridi yo'q.
    for q in app
        .requests
        .iter()
        .filter(|q| q.status == RequestStatus::Approved)
    {
        if app.purchases.iter().any(|p| p.request_id == Some(q.id)) {
            continue;
        }
        push(
            out,
            "AC-S2",
            ActionKind::CreatePurchase { request_id: q.id },
            format!("{} {}", t("ac_create_purchase"), q.number),
            format!(
                "{} {} {}",
                t("ac_needed"),
                crate::ui::materials::trim_num(q.qty),
                q.unit
            ),
            Screen::Purchases,
        );
    }

    // 3. Kelgan xarid — omborga kirim qilinmagan.
    for p in &app.purchases {
        let arrived = matches!(p.status, PurchaseStatus::Delivered | PurchaseStatus::Closed)
            || p.delivered_qty > 0.0;
        if !arrived {
            continue;
        }
        if app.stock_moves.iter().any(|m| m.document == p.number) {
            continue;
        }
        // Material aniqlanmasa kirim qilib bo'lmaydi.
        if material_of(app, p.request_id).is_none() {
            continue;
        }
        push(
            out,
            "AC-S3",
            ActionKind::PostToStock { purchase_id: p.id },
            format!("{} {}", t("ac_post_stock"), p.number),
            format!(
                "{} {} {}",
                t("ac_arrived"),
                crate::ui::materials::trim_num(if p.delivered_qty > 0.0 {
                    p.delivered_qty
                } else {
                    p.qty
                }),
                p.unit
            ),
            Screen::Warehouse,
        );
    }

    // 4. Marshruti ochilmagan yangi ariza.
    for q in app
        .requests
        .iter()
        .filter(|q| q.status == RequestStatus::New)
    {
        if app.approvals.iter().any(|a| a.request_id == q.id) {
            continue;
        }
        let amount = crate::checks::request_amount(q, &app.materials);
        let route = crate::checks::approval_route(amount);
        push(
            out,
            "AC-S4",
            ActionKind::BuildRoute { request_id: q.id },
            format!("{} {}", t("ac_build_route"), q.number),
            format!(
                "{} {} · {}",
                t("route_amount"),
                crate::ui::money(amount),
                route
                    .iter()
                    .map(|r| r.label())
                    .collect::<Vec<_>>()
                    .join(" → ")
            ),
            Screen::Requests,
        );
    }
}

/// Ariza orqali materialni topadi.
fn material_of(app: &App, request_id: Option<i64>) -> Option<i64> {
    app.requests
        .iter()
        .find(|q| Some(q.id) == request_id)?
        .material_id
}

// ================================================================ Hujjatlar

fn document_actions(app: &App, out: &mut Vec<Action>) {
    for task in app.tasks.iter().filter(|t| t.progress >= 100.0) {
        // Yashirin ishlar dalolatnomasi faqat konstruktiv bo'limlarda talab
        // qilinadi — barcha ishga hujjat taklif qilish shovqin bo'lardi.
        let needs = matches!(
            task.section,
            crate::model::Section::Kj | crate::model::Section::Ar | crate::model::Section::Km
        );
        if !needs {
            continue;
        }
        if app.exec_docs.iter().any(|d| d.task_id == Some(task.id)) {
            continue;
        }
        push(
            out,
            "AC-D1",
            ActionKind::CreateExecDoc {
                task_id: task.id,
                kind: ExecDocKind::Hidden,
            },
            format!("{} {} {}", t("ac_create_doc"), task.wbs, task.name),
            format!("{} {}", t("ac_done"), task.section.code()),
            Screen::ExecDocs,
        );
    }
}

// ================================================================ Sifat

fn quality_actions(app: &App, out: &mut Vec<Action>) {
    for q in app.quality.iter().filter(|q| q.open_defect()) {
        if q.deadline.is_some() {
            continue;
        }
        // Muddatsiz nuqson nazoratdan chiqib ketadi: kim, nima va qachongacha
        // qilishi kerakligi yozilmagan bo'lsa, u shunchaki qoladi.
        push(
            out,
            "AC-Q1",
            ActionKind::SetDefectDeadline {
                check_id: q.id,
                days: DEFECT_DAYS,
            },
            format!("{} {}", t("ac_set_deadline"), q.subject),
            format!(
                "{} · {}",
                q.result.label(),
                if q.defect.trim().is_empty() {
                    t("dash")
                } else {
                    q.defect.trim()
                }
            ),
            Screen::Quality,
        );
    }
}

/// Nuqsonga sukut bo'yicha shuncha kun beriladi.
const DEFECT_DAYS: i64 = 7;

// ================================================================ Texnik nazorat

/// Salbiy tekshiruv natijasi muddatsiz qolmasin (TZ VII.19, 21).
///
/// Ro'yxat `notify` modulidan olinadi: bir xil savolga ikki joyda ikki xil
/// javob bo'lmasligi kerak.
fn inspection_actions(app: &App, out: &mut Vec<Action>) {
    for id in crate::notify::defects_without_deadline(app) {
        let Some(x) = app.inspections.iter().find(|x| x.id == id) else {
            continue;
        };
        push(
            out,
            "AC-V1",
            ActionKind::SetInspectionDeadline {
                inspection_id: id,
                days: DEFECT_DAYS,
            },
            format!("{} {}", t("ac_set_deadline"), x.number),
            format!("{} · {}", x.result.label(), x.place),
            Screen::Inspections,
        );
    }
}

// ================================================================ Xavfsizlik

fn safety_actions(app: &App, out: &mut Vec<Action>) {
    let safety = app.worker_safety();
    for p in &app.work_permits {
        if p.status != crate::domain::PermitStatus::Open {
            continue;
        }
        if p.date_to >= app.today {
            continue;
        }
        push(
            out,
            "AC-X1",
            ActionKind::ClosePermit { permit_id: p.id },
            format!("{} {}", t("ac_close_permit"), p.number),
            format!(
                "{} {}",
                t("ac_permit_expired"),
                p.date_to.format("%d.%m.%Y")
            ),
            Screen::Safety,
        );
    }
    let _ = safety;
}

// ================================================================ Bajarish

/// TZ XVIII.21–30: tasdiqlangan amalni bajaradi.
///
/// Natija — foydalanuvchiga ko'rsatiladigan xabar yoki xato. Amal bajarilgandan
/// keyin ma'lumot qayta o'qiladi, shuning uchun taklif ro'yxati ham yangilanadi
/// va bajarilgan taklif o'zi yo'qoladi.
pub fn perform(app: &mut App, action: &Action) -> Result<String, String> {
    // Rol tekshiruvi: amal tegadigan ekranga yozish huquqi bo'lishi kerak.
    if !app.can_edit(action.screen) {
        return Err(format!("{} — {}", t("role_readonly"), app.role().label()));
    }
    let Some(pid) = app.current else {
        return Err(t("no_object_selected").to_string());
    };

    let msg = match &action.kind {
        ActionKind::CreateRequest { material_id, qty } => {
            let m = app
                .materials
                .iter()
                .find(|m| m.id == *material_id)
                .ok_or_else(|| t("ac_no_material").to_string())?;
            let n = app.requests.len() + 1;
            let number = format!("Z-{n:03}");
            app.db.insert_request(&Request {
                id: 0,
                project_id: pid,
                number: number.clone(),
                date: app.today,
                kind: RequestKind::Material,
                title: format!("{} {}", m.code, m.name),
                material_id: Some(m.id),
                qty: *qty,
                unit: m.unit.clone(),
                requester: app.user_name(),
                need_date: app.today + chrono::Duration::days(14),
                priority: Priority::Normal,
                status: RequestStatus::New,
                task_id: None,
                reject_reason: String::new(),
                note: t("ac_from_copilot").to_string(),
            });
            format!("{} {number}", t("ac_done_request"))
        }

        ActionKind::CreatePurchase { request_id } => {
            let q = app
                .requests
                .iter()
                .find(|q| q.id == *request_id)
                .cloned()
                .ok_or_else(|| t("ac_no_request").to_string())?;
            let n = app.purchases.len() + 1;
            let number = format!("X-{n:03}");
            let section = q
                .material_id
                .and_then(|id| app.materials.iter().find(|m| m.id == id))
                .map(|m| m.section)
                .unwrap_or(crate::model::Section::None);
            app.db.insert_purchase(&crate::domain::Purchase {
                id: 0,
                project_id: pid,
                request_id: Some(q.id),
                number: number.clone(),
                date: app.today,
                supplier: String::new(),
                title: q.title.clone(),
                qty: q.qty,
                unit: q.unit.clone(),
                price: q
                    .material_id
                    .and_then(|id| app.materials.iter().find(|m| m.id == id))
                    .map_or(0.0, |m| m.price),
                currency: app.settings.default_currency.clone(),
                delivery_date: q.need_date,
                status: PurchaseStatus::Draft,
                delivered_qty: 0.0,
                section,
                note: t("ac_from_copilot").to_string(),
            });
            format!("{} {number}", t("ac_done_purchase"))
        }

        ActionKind::PostToStock { purchase_id } => {
            let p = app
                .purchases
                .iter()
                .find(|p| p.id == *purchase_id)
                .cloned()
                .ok_or_else(|| t("ac_no_purchase").to_string())?;
            let material_id =
                material_of(app, p.request_id).ok_or_else(|| t("ac_no_material").to_string())?;
            let qty = if p.delivered_qty > 0.0 {
                p.delivered_qty
            } else {
                p.qty
            };
            app.db.insert_stock_move(&StockMove {
                id: 0,
                project_id: pid,
                material_id,
                date: p.delivery_date,
                kind: MoveKind::In,
                qty,
                price: p.price,
                document: p.number.clone(),
                counterparty: p.supplier.clone(),
                task_id: None,
                note: t("ac_from_copilot").to_string(),
                warehouse_id: app.warehouses.first().map(|w| w.id),
                batch_id: None,
            });
            format!(
                "{} {} {}",
                t("ac_done_stock"),
                crate::ui::materials::trim_num(qty),
                p.unit
            )
        }

        ActionKind::CreateExecDoc { task_id, kind } => {
            let task = app
                .task(*task_id)
                .cloned()
                .ok_or_else(|| t("ac_no_task").to_string())?;
            let n = app.exec_docs.len() + 1;
            let number = format!("AOSR-{n:03}");
            app.db.insert_exec_doc(&crate::domain::ExecDoc {
                id: 0,
                project_id: pid,
                kind: *kind,
                number: number.clone(),
                name: task.name.clone(),
                date: app.today,
                task_id: Some(task.id),
                status: ExecDocStatus::Draft,
                responsible: app.user_name(),
                note: t("ac_from_copilot").to_string(),
            });
            format!("{} {number}", t("ac_done_doc"))
        }

        ActionKind::SetDefectDeadline { check_id, days } => {
            let mut q: QualityCheck = app
                .quality
                .iter()
                .find(|q| q.id == *check_id)
                .cloned()
                .ok_or_else(|| t("ac_no_check").to_string())?;
            q.deadline = Some(app.today + chrono::Duration::days(*days));
            if q.result == QualityResult::Pass {
                return Err(t("ac_check_passed").to_string());
            }
            app.db.update_quality(&q);
            format!(
                "{} {}",
                t("ac_done_deadline"),
                q.deadline
                    .map(|d| d.format("%d.%m.%Y").to_string())
                    .unwrap_or_default()
            )
        }

        ActionKind::SetInspectionDeadline {
            inspection_id,
            days,
        } => {
            let mut x = app
                .inspections
                .iter()
                .find(|x| x.id == *inspection_id)
                .cloned()
                .ok_or_else(|| t("ac_no_inspection").to_string())?;
            // Ijobiy natijaga muddat qo'yish mantiqsiz — bartaraf etadigan
            // narsa yo'q.
            if x.result == crate::domain::InspectionResult::Pass {
                return Err(t("ac_check_passed").to_string());
            }
            x.deadline = Some(app.today + chrono::Duration::days(*days));
            app.db.update_inspection(&x);
            format!(
                "{} {}",
                t("ac_done_deadline"),
                x.deadline
                    .map(|d| d.format("%d.%m.%Y").to_string())
                    .unwrap_or_default()
            )
        }

        ActionKind::ClosePermit { permit_id } => {
            let mut p = app
                .work_permits
                .iter()
                .find(|p| p.id == *permit_id)
                .cloned()
                .ok_or_else(|| t("ac_no_permit").to_string())?;
            p.status = crate::domain::PermitStatus::Closed;
            app.db.update_work_permit(&p);
            format!("{} {}", t("ac_done_permit"), p.number)
        }

        ActionKind::BuildRoute { request_id } => {
            let q = app
                .requests
                .iter()
                .find(|q| q.id == *request_id)
                .cloned()
                .ok_or_else(|| t("ac_no_request").to_string())?;
            let amount = crate::checks::request_amount(&q, &app.materials);
            let route = crate::checks::approval_route(amount);
            for (i, role) in route.iter().enumerate() {
                app.db.insert_approval(&crate::domain::Approval {
                    id: 0,
                    project_id: pid,
                    request_id: q.id,
                    step: i as i64 + 1,
                    role: role.code().to_string(),
                    approver: String::new(),
                    decision: crate::domain::ApprovalDecision::Pending,
                    decided_at: None,
                    comment: String::new(),
                });
            }
            format!("{} {}", t("ac_done_route"), route.len())
        }
    };

    app.reload_modules();
    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    /// Sinov uchun namuna bazasi bilan ilova.
    fn app() -> (tempdir::Temp, App) {
        let t = tempdir::Temp::new();
        let db = Db::open(&t.path).expect("baza");
        let pid = db.seed_demo().expect("namuna");
        let mut app = App::new(Db::open(&t.path).expect("baza"));
        app.select_project(pid);
        (t, app)
    }

    /// Vaqtinchalik baza — sinovlar bir-biriga xalaqit bermasligi uchun.
    mod tempdir {
        use std::path::PathBuf;
        use std::sync::atomic::{AtomicU32, Ordering};

        static N: AtomicU32 = AtomicU32::new(0);

        pub struct Temp {
            pub path: PathBuf,
        }

        impl Temp {
            pub fn new() -> Self {
                let n = N.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir()
                    .join(format!("qurai_actions_{}_{n}.db", std::process::id()));
                let _ = std::fs::remove_file(&path);
                Temp { path }
            }
        }

        impl Drop for Temp {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.path);
            }
        }
    }

    /// TZ XVIII.12: zaxiradan tushgan materialga ariza taklif qilinadi.
    #[test]
    fn low_stock_suggests_a_request() {
        let (_t, app) = app();
        let stock = app.stock();
        let list = suggest(&app, &stock);

        let a = list
            .iter()
            .find(|a| a.code == "AC-S1")
            .expect("ariza taklifi yo'q");
        assert!(!a.evidence.is_empty(), "asos ko'rsatilmagan");
        assert_eq!(a.screen, Screen::Requests);
        match a.kind {
            ActionKind::CreateRequest { qty, .. } => assert!(qty > 0.0),
            _ => panic!("noto'g'ri tur"),
        }
    }

    /// Taklif bajarilgach yo'qoladi — takrorlanmaydi.
    #[test]
    fn performed_action_disappears_from_the_list() {
        let (_t, mut app) = app();
        let stock = app.stock();
        let a = suggest(&app, &stock)
            .into_iter()
            .find(|a| a.code == "AC-S1")
            .expect("taklif");
        let ActionKind::CreateRequest { material_id, .. } = a.kind else {
            panic!("noto'g'ri tur");
        };

        let before = app.requests.len();
        perform(&mut app, &a).expect("bajarildi");
        assert_eq!(app.requests.len(), before + 1, "ariza yaratilmadi");

        // Endi shu material uchun ochiq ariza bor — taklif takrorlanmaydi.
        let stock = app.stock();
        assert!(
            !suggest(&app, &stock).iter().any(|x| matches!(
                x.kind,
                ActionKind::CreateRequest { material_id: m, .. } if m == material_id
            )),
            "taklif takrorlandi"
        );
    }

    /// TZ XVIII.21–30: rol huquqi bo'lmasa amal bajarilmaydi.
    #[test]
    fn role_without_write_access_cannot_perform() {
        use crate::roles::{Role, User};

        let (_t, mut app) = app();
        // Buyurtmachi faqat ko'radi.
        let uid = app.db.insert_user(&User {
            id: 0,
            name: "Buyurtmachi".into(),
            role: Role::Client,
            note: String::new(),
        });
        app.reload_users();
        app.set_user(Some(uid));

        let stock = app.stock();
        let a = suggest(&app, &stock)
            .into_iter()
            .find(|a| a.code == "AC-S1")
            .expect("taklif");
        let before = app.requests.len();
        let err = perform(&mut app, &a).expect_err("huquqsiz bajarildi");
        assert!(!err.is_empty());
        assert_eq!(app.requests.len(), before, "yozuv baribir qo'shildi");
    }

    /// Har bir taklifda kod, sarlavha va asos bo'ladi.
    #[test]
    fn every_suggestion_explains_itself() {
        let (_t, app) = app();
        let stock = app.stock();
        let list = suggest(&app, &stock);
        assert!(!list.is_empty(), "namunada taklif yo'q");
        for a in &list {
            assert!(!a.code.is_empty());
            assert!(!a.title.is_empty(), "{}: sarlavha yo'q", a.code);
            assert!(!a.evidence.is_empty(), "{}: asos yo'q", a.code);
        }
    }

    /// Marshrut taklifi bajarilgach ariza kelishuvga tushadi.
    #[test]
    fn route_action_opens_the_approval_steps() {
        let (_t, mut app) = app();
        let stock = app.stock();
        let Some(a) = suggest(&app, &stock)
            .into_iter()
            .find(|a| a.code == "AC-S4")
        else {
            // Namunada barcha yangi arizada marshrut bo'lsa — sinash shart emas.
            return;
        };
        let ActionKind::BuildRoute { request_id } = a.kind else {
            panic!("noto'g'ri tur");
        };
        perform(&mut app, &a).expect("bajarildi");
        assert!(
            app.approvals.iter().any(|x| x.request_id == request_id),
            "marshrut ochilmadi"
        );
    }
}
