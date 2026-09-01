//! Tekshiruv dvigateli: loyiha (TZ II) va smeta (TZ III) qoidalari.
//!
//! TZ II.17 majburiy talabi: **AI normativlarni o'ylab topishga haqqi yo'q.**
//! Shuning uchun qoidalar normativ raqamlarini o'zida saqlamaydi — ular
//! `norm` reyestridan olinadi. Reyestr to'ldirilmagan bo'lsa, xato
//! «muhandis tekshiruvi talab qilinadi» belgisi bilan chiqadi va
//! me'yoriy asos maydoni bo'sh qoladi.

use crate::db::Db;
use crate::domain::*;
use crate::model::{Section, Task};
use crate::roles::Role;
use chrono::NaiveDate;
use rusqlite::params;
use std::collections::HashMap;

/// Bitta qoida uchun normativ asos.
#[derive(Debug, Clone, Default)]
pub struct Norm {
    pub doc: String,
    pub edition: String,
    pub clause: String,
    pub text: String,
    /// Sonli chegara (masalan, minimal uklon). `param_set` false bo'lsa ishlatilmaydi.
    pub param: f64,
    pub param_set: bool,
    pub source: String,
}

impl Norm {
    pub fn filled(&self) -> bool {
        !self.doc.trim().is_empty() && !self.clause.trim().is_empty()
    }
}

/// Dastur biladigan qoidalar ro'yxati. Kalit — reyestrdagi identifikator.
pub const RULES: &[(&str, &str)] = &[
    ("EST_ARITH", "rule_est_arith"),
    ("EST_TOTAL", "rule_est_total"),
    ("EST_DUP", "rule_est_dup"),
    ("EST_UNIT", "rule_est_unit"),
    ("EST_VOLUME", "rule_est_volume"),
    ("EST_PRICE", "rule_est_price"),
    ("EST_MISSING", "rule_est_missing"),
    ("EST_ZERO", "rule_est_zero"),
    ("PRJ_DUP_MARK", "rule_prj_dup_mark"),
    ("PRJ_NO_SHEET", "rule_prj_no_sheet"),
    ("PRJ_ZERO_SIZE", "rule_prj_zero_size"),
    ("PRJ_AR_KJ", "rule_prj_ar_kj"),
    ("PRJ_AR_VK", "rule_prj_ar_vk"),
    ("PRJ_AR_OV", "rule_prj_ar_ov"),
    ("PRJ_AR_EOM", "rule_prj_ar_eom"),
    ("PRJ_CROSS", "rule_prj_cross"),
    ("PRJ_SLOPE", "rule_prj_slope"),
    ("PPR_MISSING", "rule_ppr_missing"),
    ("PPR_NOT_APPROVED", "rule_ppr_not_approved"),
    ("PPR_ORPHAN", "rule_ppr_orphan"),
    ("PPR_RESOURCE", "rule_ppr_resource"),
    ("PPR_SEQ", "rule_ppr_seq"),
    ("PPR_RISK", "rule_ppr_risk"),
    ("PRJ_POWER", "rule_prj_power"),
    ("PRJ_KM_KJ", "rule_prj_km_kj"),
    ("PRJ_ORPHAN", "rule_prj_orphan"),
];

impl Db {
    pub fn migrate_norms(&self) -> rusqlite::Result<()> {
        self.conn().execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS norm (
                rule_key  TEXT PRIMARY KEY,
                doc       TEXT NOT NULL DEFAULT '',
                edition   TEXT NOT NULL DEFAULT '',
                clause    TEXT NOT NULL DEFAULT '',
                text      TEXT NOT NULL DEFAULT '',
                param     REAL NOT NULL DEFAULT 0,
                param_set INTEGER NOT NULL DEFAULT 0,
                source    TEXT NOT NULL DEFAULT ''
            );
            "#,
        )
    }

    pub fn norms(&self) -> HashMap<String, Norm> {
        let mut out = HashMap::new();
        let Ok(mut st) = self
            .conn()
            .prepare("SELECT rule_key,doc,edition,clause,text,param,param_set,source FROM norm")
        else {
            return out;
        };
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                Norm {
                    doc: r.get(1)?,
                    edition: r.get(2)?,
                    clause: r.get(3)?,
                    text: r.get(4)?,
                    param: r.get(5)?,
                    param_set: r.get::<_, i64>(6)? != 0,
                    source: r.get(7)?,
                },
            ))
        });
        if let Ok(rows) = rows {
            for x in rows.flatten() {
                out.insert(x.0, x.1);
            }
        }
        out
    }

    pub fn save_norm(&self, key: &str, n: &Norm) -> bool {
        self.conn()
            .execute(
                "INSERT INTO norm (rule_key,doc,edition,clause,text,param,param_set,source)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
                 ON CONFLICT(rule_key) DO UPDATE SET doc=excluded.doc,edition=excluded.edition,
                     clause=excluded.clause,text=excluded.text,param=excluded.param,
                     param_set=excluded.param_set,source=excluded.source",
                params![
                    key,
                    n.doc,
                    n.edition,
                    n.clause,
                    n.text,
                    n.param,
                    n.param_set as i64,
                    n.source
                ],
            )
            .is_ok()
    }
}

/// Tekshiruv uchun kerakli barcha ma'lumot.
pub struct Ctx<'a> {
    pub project_id: i64,
    pub tasks: &'a [Task],
    pub elements: &'a [Element],
    pub links: &'a [ElementLink],
    pub items: &'a [EstimateItem],
    pub declared_total: f64,
    pub norms: &'a HashMap<String, Norm>,
}

impl<'a> Ctx<'a> {
    fn norm(&self, key: &str) -> Norm {
        self.norms.get(key).cloned().unwrap_or_default()
    }

    /// Chegara qiymati: reyestrda bo'lsa — undan, aks holda `fallback`.
    /// Ikkinchi qiymat qiymat tasdiqlanganini bildiradi.
    fn threshold(&self, key: &str, fallback: f64) -> (f64, bool) {
        let n = self.norm(key);
        if n.param_set {
            (n.param, true)
        } else {
            (fallback, false)
        }
    }
}

/// Nomuvofiqlik yaratish uchun yordamchi: normativ maydonlarini reyestrdan to'ldiradi.
/// `Ctx` ga bog'lanmagan — PPR tekshiruvi boshqa kontekstda ishlaydi.
struct Builder<'a> {
    norms: &'a HashMap<String, Norm>,
    project_id: i64,
    module: IssueModule,
    counter: HashMap<&'static str, i64>,
}

impl<'a> Builder<'a> {
    fn new(norms: &'a HashMap<String, Norm>, project_id: i64, module: IssueModule) -> Builder<'a> {
        Builder {
            norms,
            project_id,
            module,
            counter: HashMap::new(),
        }
    }

    fn norm(&self, key: &str) -> Norm {
        self.norms.get(key).cloned().unwrap_or_default()
    }

    #[allow(clippy::too_many_arguments)]
    fn make(
        &mut self,
        rule: &'static str,
        prefix: &'static str,
        section: Section,
        severity: Severity,
        title: String,
        description: String,
        location: String,
        element: String,
        sheet: String,
        recommendation: String,
        responsible: String,
    ) -> Issue {
        let n = self.counter.entry(prefix).or_insert(0);
        *n += 1;
        let code = format!("{prefix}-{:05}", n);
        let norm = self.norm(rule);

        // Normativ asos yo'q bo'lsa, TZ II.17 talabiga ko'ra buni ochiq aytamiz.
        let (norm_doc, norm_clause, norm_text) = if norm.filled() {
            (
                format!("{} {}", norm.doc, norm.edition).trim().to_string(),
                norm.clause.clone(),
                norm.text.clone(),
            )
        } else {
            (
                String::new(),
                String::new(),
                crate::i18n::t("norm_missing").to_string(),
            )
        };

        Issue {
            id: 0,
            project_id: self.project_id,
            module: self.module,
            section,
            code,
            sheet,
            location,
            element,
            title,
            description,
            severity,
            norm_doc,
            norm_clause,
            norm_text,
            recommendation,
            responsible,
            deadline: None,
            status: IssueStatus::Open,
            auto: true,
            created_at: String::new(),
        }
    }
}

/// HashMap guruhlarini kalit bo'yicha barqaror tartibda qaytaradi.
/// Tekshiruv natijasi ishga tushirishdan ishga tushirishga o'zgarmasligi kerak.
fn sorted_groups<V>(map: &HashMap<String, Vec<V>>) -> Vec<&Vec<V>> {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();
    keys.into_iter().map(|k| &map[k]).collect()
}

fn norm_name(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// O'lchov birligining o'lchamliligi: uzunlik, yuza, hajm, massa, dona.
fn unit_dim(u: &str) -> &'static str {
    let u = u.trim().to_lowercase().replace(['.', ' '], "");
    match u.as_str() {
        "m" | "м" | "mp" | "мп" | "pm" | "пм" => "L",
        "m2" | "м2" | "квм" | "kvm" => "A",
        "m3" | "м3" | "кубм" | "kubm" => "V",
        "kg" | "кг" | "t" | "т" | "tonna" | "тонна" => "M",
        "dona" | "шт" | "sht" | "pcs" => "N",
        "komplekt" | "компл" | "kompl" => "K",
        "soat" | "час" | "h" | "ч" => "T",
        _ => "?",
    }
}

// ================= III. Smeta tekshiruvi =================

/// Smetani tekshiradi (TZ III.4–III.9): arifmetika, birliklar, dublikatlar,
/// hajmlar, narx anomaliyalari va yetishmayotgan ishlar.
pub fn check_estimate(ctx: &Ctx) -> Vec<Issue> {
    let mut b = Builder::new(ctx.norms, ctx.project_id, IssueModule::Estimate);
    let mut out = Vec::new();

    // --- III.4 Arifmetika: miqdor × narx = summa ---
    let (tol, _) = ctx.threshold("EST_ARITH", 0.5);
    for it in ctx.items {
        let calc = it.computed();
        let diff = calc - it.cost;
        if diff.abs() > tol.max(calc.abs() * 0.0001) {
            out.push(b.make(
                "EST_ARITH",
                "SM",
                it.section,
                Severity::Critical,
                crate::i18n::t("chk_arith_title").to_string(),
                format!(
                    "{}: {} x {} = {}; {}: {}. {}: {}",
                    crate::i18n::t("chk_computed"),
                    fmt(it.qty),
                    fmt(it.price),
                    fmt(calc),
                    crate::i18n::t("chk_in_estimate"),
                    fmt(it.cost),
                    crate::i18n::t("chk_diff"),
                    fmt(diff)
                ),
                format!("{} {}", crate::i18n::t("chk_pos"), it.pos),
                it.name.clone(),
                String::new(),
                crate::i18n::t("chk_arith_fix").to_string(),
                crate::i18n::t("role_client").to_string(),
            ));
        }
    }

    // --- III.4 Yakuniy summa ---
    let sum: f64 = ctx.items.iter().map(|i| i.cost).sum();
    if ctx.declared_total > 0.0 && (sum - ctx.declared_total).abs() > 1.0 {
        out.push(b.make(
            "EST_TOTAL",
            "SM",
            Section::None,
            Severity::Critical,
            crate::i18n::t("chk_total_title").to_string(),
            format!(
                "{}: {}; {}: {}. {}: {}",
                crate::i18n::t("chk_items_sum"),
                fmt(sum),
                crate::i18n::t("chk_declared"),
                fmt(ctx.declared_total),
                crate::i18n::t("chk_diff"),
                fmt(sum - ctx.declared_total)
            ),
            crate::i18n::t("chk_estimate_total").to_string(),
            String::new(),
            String::new(),
            crate::i18n::t("chk_total_fix").to_string(),
            String::new(),
        ));
    }

    // --- III.8 Dublikatlar ---
    let mut by_name: HashMap<String, Vec<&EstimateItem>> = HashMap::new();
    for it in ctx.items {
        if it.name.trim().is_empty() {
            continue;
        }
        by_name.entry(norm_name(&it.name)).or_default().push(it);
    }
    // Tartib barqaror bo'lishi shart: xato kodlari (SM-00001) shu tartibda
    // beriladi va foydalanuvchi yopgan xato kodi bo'yicha eslab qolinadi.
    for group in sorted_groups(&by_name) {
        if group.len() < 2 {
            continue;
        }
        let positions: Vec<String> = group.iter().map(|i| i.pos.to_string()).collect();
        // Miqdorlar ham teng bo'lsa — dublikat ehtimoli yuqori.
        let same_qty = group
            .windows(2)
            .all(|w| (w[0].qty - w[1].qty).abs() < 0.001);
        out.push(b.make(
            "EST_DUP",
            "SM",
            group[0].section,
            if same_qty {
                Severity::Critical
            } else {
                Severity::Warning
            },
            crate::i18n::t("chk_dup_title").to_string(),
            format!(
                "«{}» {}: {}. {}",
                group[0].name,
                crate::i18n::t("chk_dup_positions"),
                positions.join(", "),
                if same_qty {
                    crate::i18n::t("chk_dup_same_qty")
                } else {
                    crate::i18n::t("chk_dup_diff_qty")
                }
            ),
            format!("{} {}", crate::i18n::t("chk_pos"), positions.join(", ")),
            group[0].name.clone(),
            String::new(),
            crate::i18n::t("chk_dup_fix").to_string(),
            String::new(),
        ));
    }

    // --- III.5 Birliklar: loyihadagi ish bilan solishtirish ---
    let task_by_name: HashMap<String, &Task> =
        ctx.tasks.iter().map(|t| (norm_name(&t.name), t)).collect();
    for it in ctx.items {
        if unit_dim(&it.unit) == "?" && !it.unit.trim().is_empty() {
            out.push(b.make(
                "EST_UNIT",
                "SM",
                it.section,
                Severity::Warning,
                crate::i18n::t("chk_unit_unknown").to_string(),
                format!("«{}»: {}", it.unit, crate::i18n::t("chk_unit_unknown_desc")),
                format!("{} {}", crate::i18n::t("chk_pos"), it.pos),
                it.name.clone(),
                String::new(),
                crate::i18n::t("chk_unit_fix").to_string(),
                String::new(),
            ));
            continue;
        }
        if let Some(task) = task_by_name.get(&norm_name(&it.name)) {
            if !task.unit.trim().is_empty()
                && unit_dim(&task.unit) != "?"
                && unit_dim(&task.unit) != unit_dim(&it.unit)
            {
                out.push(b.make(
                    "EST_UNIT",
                    "SM",
                    it.section,
                    Severity::Critical,
                    crate::i18n::t("chk_unit_title").to_string(),
                    format!(
                        "{}: {}; {}: {}",
                        crate::i18n::t("chk_in_project"),
                        task.unit,
                        crate::i18n::t("chk_in_estimate"),
                        it.unit
                    ),
                    format!("{} {}", crate::i18n::t("chk_pos"), it.pos),
                    it.name.clone(),
                    String::new(),
                    crate::i18n::t("chk_unit_fix").to_string(),
                    String::new(),
                ));
            }
        }
    }

    // --- III.6 Hajmlar: loyiha bo'yicha hajm bilan solishtirish ---
    let (dev_pct, _) = ctx.threshold("EST_VOLUME", 5.0);
    for it in ctx.items {
        let Some(task) = task_by_name.get(&norm_name(&it.name)) else {
            continue;
        };
        if task.volume <= 0.0 || it.qty <= 0.0 {
            continue;
        }
        if unit_dim(&task.unit) != unit_dim(&it.unit) {
            continue; // birliklar mos emas — yuqorida alohida xato berilgan
        }
        let delta = (it.qty - task.volume) / task.volume * 100.0;
        if delta.abs() > dev_pct {
            out.push(b.make(
                "EST_VOLUME",
                "SM",
                it.section,
                if delta.abs() > dev_pct * 3.0 {
                    Severity::Critical
                } else {
                    Severity::Major
                },
                if delta > 0.0 {
                    crate::i18n::t("chk_vol_over").to_string()
                } else {
                    crate::i18n::t("chk_vol_under").to_string()
                },
                format!(
                    "{}: {} {}; {}: {} {}. {}: {:+.1} %\n{}: {}",
                    crate::i18n::t("chk_in_project"),
                    fmt(task.volume),
                    task.unit,
                    crate::i18n::t("chk_in_estimate"),
                    fmt(it.qty),
                    it.unit,
                    crate::i18n::t("chk_deviation"),
                    delta,
                    crate::i18n::t("chk_conclusion"),
                    crate::i18n::t("chk_needs_check")
                ),
                format!("{} {}", crate::i18n::t("chk_pos"), it.pos),
                it.name.clone(),
                String::new(),
                crate::i18n::t("chk_vol_fix").to_string(),
                String::new(),
            ));
        }
    }

    // --- III.9 Narx anomaliyalari: bir xil ish uchun turli narx ---
    let mut price_by_code: HashMap<String, Vec<&EstimateItem>> = HashMap::new();
    for it in ctx.items {
        let key = if it.code.trim().is_empty() {
            norm_name(&it.name)
        } else {
            it.code.trim().to_lowercase()
        };
        if !key.is_empty() {
            price_by_code.entry(key).or_default().push(it);
        }
    }
    let (price_pct, _) = ctx.threshold("EST_PRICE", 15.0);
    for group in sorted_groups(&price_by_code) {
        if group.len() < 2 {
            continue;
        }
        let min = group.iter().map(|i| i.price).fold(f64::MAX, f64::min);
        let max = group.iter().map(|i| i.price).fold(0.0, f64::max);
        if min > 0.0 && (max - min) / min * 100.0 > price_pct {
            out.push(
                b.make(
                    "EST_PRICE",
                    "SM",
                    group[0].section,
                    Severity::Major,
                    crate::i18n::t("chk_price_title").to_string(),
                    format!(
                        "«{}»: {} {} … {} ({:+.1} %)",
                        group[0].name,
                        crate::i18n::t("chk_price_range"),
                        fmt(min),
                        fmt(max),
                        (max - min) / min * 100.0
                    ),
                    group
                        .iter()
                        .map(|i| i.pos.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                    group[0].name.clone(),
                    String::new(),
                    crate::i18n::t("chk_price_fix").to_string(),
                    String::new(),
                ),
            );
        }
    }

    // --- III.8 Nol yoki manfiy qiymatlar ---
    for it in ctx.items {
        if it.qty <= 0.0 || it.price < 0.0 {
            out.push(b.make(
                "EST_ZERO",
                "SM",
                it.section,
                Severity::Warning,
                crate::i18n::t("chk_zero_title").to_string(),
                format!(
                    "{}: {}; {}: {}",
                    crate::i18n::t("task_volume"),
                    fmt(it.qty),
                    crate::i18n::t("chk_price"),
                    fmt(it.price)
                ),
                format!("{} {}", crate::i18n::t("chk_pos"), it.pos),
                it.name.clone(),
                String::new(),
                crate::i18n::t("chk_zero_fix").to_string(),
                String::new(),
            ));
        }
    }

    // --- III.6 Yetishmayotgan ishlar: GPR da bor, smetada yo'q ---
    let est_names: std::collections::HashSet<String> =
        ctx.items.iter().map(|i| norm_name(&i.name)).collect();
    for task in ctx.tasks {
        let n = norm_name(&task.name);
        if n.is_empty() || est_names.contains(&n) {
            continue;
        }
        out.push(b.make(
            "EST_MISSING",
            "SM",
            task.section,
            Severity::Major,
            crate::i18n::t("chk_missing_title").to_string(),
            format!("«{}» {}", task.name, crate::i18n::t("chk_missing_desc")),
            task.wbs.clone(),
            task.name.clone(),
            String::new(),
            crate::i18n::t("chk_missing_fix").to_string(),
            task.responsible.clone(),
        ));
    }

    out.sort_by_key(|i| i.severity.rank());
    out
}

/// Smeta tekshiruvining pul ko'rinishidagi xulosasi (TZ III.31).
///
/// Har bir son alohida hisoblanadi va nima hisobga olinganini aytib turadi —
/// TZ III.32 ga ko'ra dastur «xato» deb hukm chiqarmaydi, faqat tekshirishga
/// arziydigan summani ko'rsatadi.
#[derive(Debug, Clone, Default)]
pub struct CostSummary {
    /// Pozitsiyalar yig'indisi.
    pub total: f64,
    /// Hujjatda ko'rsatilgan yakun bilan farq.
    pub declared_diff: f64,
    /// Loyihadagi hajmdan oshgan qismning qiymati.
    pub volume_excess: f64,
    /// Takrorlangan pozitsiyalarning qiymati (birinchisidan keyingilari).
    pub duplicate_cost: f64,
    /// Arifmetik xatolarning umumiy farqi.
    pub arithmetic_diff: f64,
    /// Bir xil ish uchun eng past narxga keltirilsa tejaladigan summa.
    pub price_saving: f64,
    /// Loyihaga mos kelmaydigan pozitsiyalar soni.
    pub mismatch_count: usize,
    pub duplicate_count: usize,
}

/// Smeta bo'yicha pul xulosasini hisoblaydi.
pub fn cost_summary(ctx: &Ctx) -> CostSummary {
    let mut s = CostSummary {
        total: ctx.items.iter().map(|i| i.cost).sum(),
        ..Default::default()
    };
    if ctx.declared_total > 0.0 {
        s.declared_diff = s.total - ctx.declared_total;
    }

    // Arifmetika: hujjatdagi summa bilan hisob orasidagi farqlar yig'indisi.
    s.arithmetic_diff = ctx
        .items
        .iter()
        .map(|i| i.cost - i.computed())
        .filter(|d| d.abs() > 0.5)
        .sum();

    // Hajm: loyihadagi hajmdan oshgan qismning narxi.
    let task_by_name: HashMap<String, &Task> =
        ctx.tasks.iter().map(|t| (norm_name(&t.name), t)).collect();
    for it in ctx.items {
        let Some(task) = task_by_name.get(&norm_name(&it.name)) else {
            continue;
        };
        if task.volume <= 0.0 || it.qty <= 0.0 {
            continue;
        }
        if unit_dim(&task.unit) != unit_dim(&it.unit) {
            s.mismatch_count += 1;
            continue;
        }
        if it.qty > task.volume {
            s.volume_excess += (it.qty - task.volume) * it.price;
            s.mismatch_count += 1;
        }
    }

    // Dublikatlar: bir xil nomdagi pozitsiyalarning birinchisidan keyingilari.
    let mut by_name: HashMap<String, Vec<&EstimateItem>> = HashMap::new();
    for it in ctx.items {
        if !it.name.trim().is_empty() {
            by_name.entry(norm_name(&it.name)).or_default().push(it);
        }
    }
    for group in sorted_groups(&by_name) {
        if group.len() < 2 {
            continue;
        }
        s.duplicate_count += group.len() - 1;
        s.duplicate_cost += group.iter().skip(1).map(|i| i.cost).sum::<f64>();
    }

    // Narx: bir xil rasenka kodidagi pozitsiyalarni eng past narxga keltirish.
    let mut by_code: HashMap<String, Vec<&EstimateItem>> = HashMap::new();
    for it in ctx.items {
        let key = if it.code.trim().is_empty() {
            norm_name(&it.name)
        } else {
            it.code.trim().to_lowercase()
        };
        if !key.is_empty() {
            by_code.entry(key).or_default().push(it);
        }
    }
    for group in sorted_groups(&by_code) {
        if group.len() < 2 {
            continue;
        }
        let min = group.iter().map(|i| i.price).fold(f64::MAX, f64::min);
        if min <= 0.0 {
            continue;
        }
        s.price_saving += group.iter().map(|i| (i.price - min) * i.qty).sum::<f64>();
    }

    s
}

// ================= II. Loyiha tekshiruvi =================

/// Loyihani element grafi bo'yicha tekshiradi (TZ II.11 cross-check).
pub fn check_project(ctx: &Ctx) -> Vec<Issue> {
    let mut b = Builder::new(ctx.norms, ctx.project_id, IssueModule::Project);
    let mut out = Vec::new();
    let by_id: HashMap<i64, &Element> = ctx.elements.iter().map(|e| (e.id, e)).collect();

    // Element uchun chiquvchi va kiruvchi bog'lanishlar.
    let mut rel_out: HashMap<i64, Vec<(&ElementLink, &Element)>> = HashMap::new();
    let mut rel_in: HashMap<i64, Vec<(&ElementLink, &Element)>> = HashMap::new();
    for l in ctx.links {
        if let (Some(a), Some(c)) = (by_id.get(&l.from_el), by_id.get(&l.to_el)) {
            rel_out.entry(l.from_el).or_default().push((l, c));
            rel_in.entry(l.to_el).or_default().push((l, a));
        }
    }

    // --- Takrorlanuvchi markalar ---
    let mut by_mark: HashMap<(String, Section), Vec<&Element>> = HashMap::new();
    for e in ctx.elements {
        if !e.mark.trim().is_empty() {
            by_mark
                .entry((e.mark.trim().to_lowercase(), e.section))
                .or_default()
                .push(e);
        }
    }
    let mut mark_keys: Vec<&(String, Section)> = by_mark.keys().collect();
    mark_keys.sort_by(|a, b| (a.1.code(), &a.0).cmp(&(b.1.code(), &b.0)));
    for key in mark_keys {
        let (_, sec) = key;
        let group = &by_mark[key];
        // Guruh kaliti kichik harfga keltirilgan — ekranga chizmadagi asl marka chiqadi.
        let mark = &group[0].mark;
        if group.len() > 1 {
            out.push(
                b.make(
                    "PRJ_DUP_MARK",
                    "PR",
                    *sec,
                    Severity::Warning,
                    crate::i18n::t("chk_dupmark_title").to_string(),
                    format!(
                        "«{}» — {} {}",
                        mark,
                        group.len(),
                        crate::i18n::t("chk_times")
                    ),
                    group
                        .iter()
                        .map(|e| e.sheet.clone())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(", "),
                    mark.clone(),
                    String::new(),
                    crate::i18n::t("chk_dupmark_fix").to_string(),
                    String::new(),
                ),
            );
        }
    }

    // --- Varaq ko'rsatilmagan, o'lcham nol ---
    for e in ctx.elements {
        if e.sheet.trim().is_empty() {
            out.push(b.make(
                "PRJ_NO_SHEET",
                "PR",
                e.section,
                Severity::Info,
                crate::i18n::t("chk_nosheet_title").to_string(),
                crate::i18n::t("chk_nosheet_desc").to_string(),
                e.room.clone(),
                e.mark.clone(),
                String::new(),
                crate::i18n::t("chk_nosheet_fix").to_string(),
                String::new(),
            ));
        }
        if e.size <= 0.0
            && matches!(
                e.kind,
                ElementKind::Pipe | ElementKind::Duct | ElementKind::Cable | ElementKind::Opening
            )
        {
            out.push(b.make(
                "PRJ_ZERO_SIZE",
                "PR",
                e.section,
                Severity::Major,
                crate::i18n::t("chk_zerosize_title").to_string(),
                crate::i18n::t("chk_zerosize_desc").to_string(),
                e.room.clone(),
                e.mark.clone(),
                e.sheet.clone(),
                crate::i18n::t("chk_zerosize_fix").to_string(),
                String::new(),
            ));
        }
    }

    // --- II.11 AR ↔ KJ: deraza/eshik bor, teshik yo'q ---
    for e in ctx.elements {
        if e.section != Section::Ar || !matches!(e.kind, ElementKind::Window | ElementKind::Door) {
            continue;
        }
        let has_opening = rel_out
            .get(&e.id)
            .map(|v| v.iter().any(|(_, o)| o.kind == ElementKind::Opening))
            .unwrap_or(false);
        if !has_opening {
            out.push(b.make(
                "PRJ_AR_KJ",
                "PR",
                Section::Ar,
                Severity::Critical,
                crate::i18n::t("chk_arkj_title").to_string(),
                format!(
                    "{} «{}» {}",
                    e.kind.label(),
                    e.mark,
                    crate::i18n::t("chk_arkj_desc")
                ),
                format!("{} {}", e.room, e.axis).trim().to_string(),
                e.mark.clone(),
                e.sheet.clone(),
                crate::i18n::t("chk_arkj_fix").to_string(),
                crate::i18n::t("role_designer").to_string(),
            ));
        }
    }

    // --- II.11 AR ↔ VK / OV / EOM: xona uchun muhandislik ta'minoti ---
    for room in ctx.elements.iter().filter(|e| e.kind == ElementKind::Room) {
        let serving: Vec<Section> = rel_in
            .get(&room.id)
            .map(|v| {
                v.iter()
                    .filter(|(l, _)| l.relation == Relation::Serves)
                    .map(|(_, e)| e.section)
                    .collect()
            })
            .unwrap_or_default();

        let checks: [(Section, &str, &str); 3] = [
            (Section::Vk, "PRJ_AR_VK", "chk_arvk_desc"),
            (Section::Ov, "PRJ_AR_OV", "chk_arov_desc"),
            (Section::Eom, "PRJ_AR_EOM", "chk_areom_desc"),
        ];
        for (sec, rule, desc_key) in checks {
            if serving.contains(&sec) {
                continue;
            }
            // Statik qoidalar: kalitni 'static qilib berish uchun moslashtiramiz.
            let rule_key: &'static str = match rule {
                "PRJ_AR_VK" => "PRJ_AR_VK",
                "PRJ_AR_OV" => "PRJ_AR_OV",
                _ => "PRJ_AR_EOM",
            };
            out.push(b.make(
                rule_key,
                "PR",
                sec,
                Severity::Major,
                crate::i18n::t("chk_room_service_title").to_string(),
                format!(
                    "{} «{}»: {}",
                    crate::i18n::t("ek_room"),
                    if room.mark.is_empty() {
                        room.room.clone()
                    } else {
                        room.mark.clone()
                    },
                    crate::i18n::t(desc_key)
                ),
                room.room.clone(),
                room.mark.clone(),
                room.sheet.clone(),
                crate::i18n::t("chk_room_service_fix").to_string(),
                crate::i18n::t("role_designer").to_string(),
            ));
        }
    }

    // --- II.11 Muhandislik tarmog'i konstruksiyani kesib o'tadi ---
    for l in ctx.links {
        if l.relation != Relation::Crosses {
            continue;
        }
        let (Some(a), Some(c)) = (by_id.get(&l.from_el), by_id.get(&l.to_el)) else {
            continue;
        };
        let is_net = matches!(
            a.kind,
            ElementKind::Pipe | ElementKind::Duct | ElementKind::Cable
        );
        let is_struct = matches!(
            c.kind,
            ElementKind::Beam | ElementKind::Column | ElementKind::Slab
        );
        if !(is_net && is_struct) {
            continue;
        }
        // Teshik ko'zda tutilganmi?
        let has_opening = rel_out
            .get(&c.id)
            .map(|v| v.iter().any(|(_, o)| o.kind == ElementKind::Opening))
            .unwrap_or(false);
        if !has_opening {
            out.push(b.make(
                "PRJ_CROSS",
                "PR",
                a.section,
                Severity::Critical,
                crate::i18n::t("chk_cross_title").to_string(),
                format!(
                    "{} «{}» {} {} «{}»",
                    a.kind.label(),
                    a.mark,
                    crate::i18n::t("chk_cross_desc"),
                    c.kind.label(),
                    c.mark
                ),
                format!("{} {}", a.room, a.axis).trim().to_string(),
                a.mark.clone(),
                a.sheet.clone(),
                crate::i18n::t("chk_cross_fix").to_string(),
                crate::i18n::t("role_designer").to_string(),
            ));
        }
    }

    // --- II.6 Kanalizatsiya uklonini tekshirish ---
    // Chegara reyestrdan olinadi; yo'q bo'lsa hisob ishlaydi, lekin
    // «tasdiqlanmagan chegara» sifatida belgilanadi.
    let (min_slope, confirmed) = ctx.threshold("PRJ_SLOPE", 0.02);
    for e in ctx.elements {
        if e.section != Section::Vk || e.kind != ElementKind::Pipe {
            continue;
        }
        if norm_name(&e.value_name) != norm_name("uklon")
            && !e.value_name.to_lowercase().contains("uklon")
            && !e.value_name.to_lowercase().contains("уклон")
        {
            continue;
        }
        if e.value > 0.0 && e.value < min_slope {
            let mut desc = format!(
                "{}: {:.4}; {}: {:.4}",
                crate::i18n::t("chk_slope_actual"),
                e.value,
                crate::i18n::t("chk_slope_min"),
                min_slope
            );
            if !confirmed {
                desc.push('\n');
                desc.push_str(crate::i18n::t("chk_threshold_unconfirmed"));
            }
            out.push(b.make(
                "PRJ_SLOPE",
                "PR",
                Section::Vk,
                Severity::Critical,
                crate::i18n::t("chk_slope_title").to_string(),
                desc,
                format!("{} {}", e.room, e.level).trim().to_string(),
                e.mark.clone(),
                e.sheet.clone(),
                crate::i18n::t("chk_slope_fix").to_string(),
                crate::i18n::t("role_designer").to_string(),
            ));
        }
    }

    // --- II.11 OV/VK/PB/SS <-> EOM: qurilma bor, elektr ta'minoti yo'q ---
    // «Ventilyator bor, elektr ta'minoti ko'zda tutilmagan», «nasos bor, ulanish yo'q»,
    // «yong'in tizimi bor, zaxira ta'minot yo'q» — TZ II.11 misollari.
    for e in ctx.elements {
        if e.kind != ElementKind::Device
            || !matches!(
                e.section,
                Section::Ov | Section::Vk | Section::Pb | Section::Ss
            )
        {
            continue;
        }
        let powered_by = |m: &HashMap<i64, Vec<(&ElementLink, &Element)>>| {
            m.get(&e.id)
                .map(|v| {
                    v.iter().any(|(l, o)| {
                        l.relation == Relation::PoweredBy && o.section == Section::Eom
                    })
                })
                .unwrap_or(false)
        };
        if powered_by(&rel_out) || powered_by(&rel_in) {
            continue;
        }
        out.push(b.make(
            "PRJ_POWER",
            "PR",
            e.section,
            // Yong'in xavfsizligi tizimi uchun bu kritik, qolganlari uchun jiddiy.
            if e.section == Section::Pb {
                Severity::Critical
            } else {
                Severity::Major
            },
            crate::i18n::t("chk_power_title").to_string(),
            format!(
                "{} «{}» {}",
                e.kind.label(),
                e.mark,
                crate::i18n::t("chk_power_desc")
            ),
            format!("{} {}", e.room, e.axis).trim().to_string(),
            e.mark.clone(),
            e.sheet.clone(),
            crate::i18n::t("chk_power_fix").to_string(),
            crate::i18n::t("role_designer").to_string(),
        ));
    }

    // --- II.11 KM <-> KJ: metall konstruksiya tayanchi aniqlanmagan ---
    for e in ctx.elements {
        if e.section != Section::Km || !matches!(e.kind, ElementKind::Column | ElementKind::Beam) {
            continue;
        }
        let supported = rel_out
            .get(&e.id)
            .map(|v| {
                v.iter()
                    .any(|(l, o)| l.relation == Relation::SupportedBy && o.section == Section::Kj)
            })
            .unwrap_or(false);
        if supported {
            continue;
        }
        out.push(b.make(
            "PRJ_KM_KJ",
            "PR",
            Section::Km,
            Severity::Major,
            crate::i18n::t("chk_kmkj_title").to_string(),
            format!(
                "{} «{}» {}",
                e.kind.label(),
                e.mark,
                crate::i18n::t("chk_kmkj_desc")
            ),
            format!("{} {}", e.room, e.axis).trim().to_string(),
            e.mark.clone(),
            e.sheet.clone(),
            crate::i18n::t("chk_kmkj_fix").to_string(),
            crate::i18n::t("role_designer").to_string(),
        ));
    }

    // --- Bog'lanmagan elementlar: grafda yolg'iz turibdi ---
    for e in ctx.elements {
        if e.kind == ElementKind::Room {
            continue;
        }
        let deg = rel_out.get(&e.id).map(|v| v.len()).unwrap_or(0)
            + rel_in.get(&e.id).map(|v| v.len()).unwrap_or(0);
        if deg == 0 {
            out.push(b.make(
                "PRJ_ORPHAN",
                "PR",
                e.section,
                Severity::Info,
                crate::i18n::t("chk_orphan_title").to_string(),
                crate::i18n::t("chk_orphan_desc").to_string(),
                e.room.clone(),
                e.mark.clone(),
                e.sheet.clone(),
                crate::i18n::t("chk_orphan_fix").to_string(),
                String::new(),
            ));
        }
    }

    out.sort_by_key(|i| i.severity.rank());
    out
}

/// TZ II.18: element o'zgarganda qaysi bo'limlarga ta'sir qilishini ko'rsatadi.
pub fn impact(elements: &[Element], links: &[ElementLink], root: i64) -> Vec<(Section, usize)> {
    let by_id: HashMap<i64, &Element> = elements.iter().map(|e| (e.id, e)).collect();
    let mut adj: HashMap<i64, Vec<i64>> = HashMap::new();
    for l in links {
        adj.entry(l.from_el).or_default().push(l.to_el);
        adj.entry(l.to_el).or_default().push(l.from_el);
    }

    let mut seen = std::collections::HashSet::new();
    let mut queue = vec![root];
    seen.insert(root);
    while let Some(id) = queue.pop() {
        for &n in adj.get(&id).map(|v| v.as_slice()).unwrap_or(&[]) {
            if seen.insert(n) {
                queue.push(n);
            }
        }
    }

    let mut counts: HashMap<Section, usize> = HashMap::new();
    for id in seen.iter().filter(|&&i| i != root) {
        if let Some(e) = by_id.get(id) {
            *counts.entry(e.section).or_default() += 1;
        }
    }
    let mut v: Vec<(Section, usize)> = counts.into_iter().collect();
    v.sort_by_key(|(s, _)| s.code());
    v
}

// ================= IX-X. Ta'minot zanjiri =================

/// Bitta ariza bo'yicha ta'minot holati (TZ IX-X).
#[derive(Debug, Clone)]
pub struct SupplyLine {
    pub request_id: i64,
    /// Shu arizaga bog'langan xaridlar soni.
    pub purchases: usize,
    /// Xaridlarda buyurtma qilingan umumiy miqdor.
    pub ordered: f64,
    /// Yetkazilgan deb belgilangan miqdor.
    pub delivered: f64,
    /// Xaridlarning umumiy summasi.
    pub amount: f64,
    /// Buyurtma miqdori arizadagi ehtiyojni qoplaydi.
    pub covered: bool,
    /// Kerak bo'lgan sana o'tgan, ammo yetkazilmagan.
    pub late: bool,
    /// Rejalashtirilgan yetkazish arizadagi muddatdan keyin.
    pub planned_after_need: bool,
}

/// TZ IX-X: ariza va unga bog'langan xaridlarni solishtiradi.
///
/// Dastur hukm chiqarmaydi (TZ III.32) — bu yerda faqat fakt qayd etiladi:
/// qancha so'raldi, qancha buyurtma qilindi, qancha keldi va sana qanday.
pub fn supply_status(
    requests: &[Request],
    purchases: &[Purchase],
    today: NaiveDate,
) -> Vec<SupplyLine> {
    let mut out = Vec::with_capacity(requests.len());
    for q in requests {
        let mine: Vec<&Purchase> = purchases
            .iter()
            .filter(|p| p.request_id == Some(q.id))
            .collect();
        let ordered: f64 = mine.iter().map(|p| p.qty).sum();
        let delivered: f64 = mine
            .iter()
            .filter(|p| matches!(p.status, PurchaseStatus::Delivered | PurchaseStatus::Closed))
            .map(|p| p.qty)
            .sum();
        let amount: f64 = mine.iter().map(|p| p.amount()).sum();
        let done = matches!(q.status, RequestStatus::Delivered | RequestStatus::Closed);
        let dropped = q.status == RequestStatus::Rejected;

        out.push(SupplyLine {
            request_id: q.id,
            purchases: mine.len(),
            ordered,
            delivered,
            amount,
            // Miqdor ko'rsatilmagan arizani qoplangan deb hisoblamaymiz.
            covered: q.qty > 0.0 && ordered + 1e-6 >= q.qty,
            late: !done && !dropped && q.need_date < today,
            planned_after_need: !done
                && !dropped
                && mine.iter().any(|p| p.delivery_date > q.need_date),
        });
    }
    out
}

// ================= XI-XII. Ombor =================

/// Bitta material bo'yicha ombor holati.
#[derive(Debug, Clone)]
pub struct StockLine {
    pub material_id: i64,
    /// Kirim va qaytarish minus chiqim va hisobdan chiqarish.
    pub balance: f64,
    pub incoming: f64,
    pub outgoing: f64,
    pub written_off: f64,
    /// Ishdan qaytarilgan miqdor (TZ XI.21).
    pub returned: f64,
    /// Aniq ish uchun band qilingan miqdor (TZ XI.17).
    pub reserved: f64,
    /// Erkin qoldiq: qoldiqdan rezerv ayrilgan.
    pub available: f64,
    /// Qoldiqning taxminiy qiymati.
    pub value: f64,
    /// Hisobda ishlatilgan birlik narxi (kirimlarning o'rtachasi yoki katalog narxi).
    pub unit_price: f64,
    pub last_move: Option<NaiveDate>,
    /// Erkin qoldiq minimal zaxiradan past.
    pub below_min: bool,
    /// Chiqim kirimdan ko'p — hujjatlarda xato bor.
    pub negative: bool,
}

/// Harakat qoldiqni oshiradimi.
///
/// Qaytarish ham kirim: ishdan ortgan material omborga qaytadi (TZ XI.21).
fn adds_to_stock(k: MoveKind) -> bool {
    matches!(k, MoveKind::In | MoveKind::Return)
}

/// TZ XI: har bir material bo'yicha qoldiq va uning qiymati.
///
/// Birlik narxi kirimlarning vaznlangan o'rtachasidan olinadi; narx
/// ko'rsatilmagan bo'lsa katalogdagi narx ishlatiladi. Shunday qilinganining
/// sababi: ombor qiymati haqiqiy xaridga tayanishi kerak, katalog narxi esa
/// eskirgan bo'lishi mumkin.
///
/// `warehouse` berilsa — faqat shu omborning qoldig'i hisoblanadi (TZ XI.3).
/// Rezervlar omborga bog'lanmagani uchun ular faqat umumiy hisobda ayriladi.
pub fn stock_balances_in(
    materials: &[Material],
    moves: &[StockMove],
    reservations: &[Reservation],
    today: NaiveDate,
    warehouse: Option<i64>,
) -> Vec<StockLine> {
    let mut out = Vec::with_capacity(materials.len());
    for m in materials {
        let mine: Vec<&StockMove> = moves
            .iter()
            .filter(|x| x.material_id == m.id)
            .filter(|x| warehouse.is_none_or(|w| x.warehouse_id == Some(w)))
            .collect();
        let sum = |k: MoveKind| -> f64 { mine.iter().filter(|x| x.kind == k).map(|x| x.qty).sum() };
        let incoming = sum(MoveKind::In);
        let returned = sum(MoveKind::Return);
        let outgoing = sum(MoveKind::Out);
        let written_off = sum(MoveKind::WriteOff);
        let balance = incoming + returned - outgoing - written_off;

        // Rezerv: muddati o'tmagan va shu materialga tegishli.
        let reserved: f64 = if warehouse.is_some() {
            0.0
        } else {
            reservations
                .iter()
                .filter(|r| r.material_id == m.id)
                .filter(|r| r.until.is_none_or(|d| d >= today))
                .map(|r| r.qty)
                .sum()
        };
        let available = balance - reserved;

        // Kirimlarning vaznlangan o'rtacha narxi.
        let priced: Vec<&&StockMove> = mine
            .iter()
            .filter(|x| adds_to_stock(x.kind) && x.price > 0.0 && x.qty > 0.0)
            .collect();
        let unit_price = if priced.is_empty() {
            m.price
        } else {
            let qty: f64 = priced.iter().map(|x| x.qty).sum();
            let cost: f64 = priced.iter().map(|x| x.qty * x.price).sum();
            if qty > 0.0 {
                cost / qty
            } else {
                m.price
            }
        };

        out.push(StockLine {
            material_id: m.id,
            balance,
            incoming,
            outgoing,
            written_off,
            returned,
            reserved,
            available,
            value: balance.max(0.0) * unit_price,
            unit_price,
            last_move: mine.iter().map(|x| x.date).max(),
            // Ogohlantirish erkin qoldiqqa qaraydi: rezervdagi material
            // boshqa ishga tegishli va uni ishlatib bo'lmaydi.
            below_min: m.min_stock > 0.0 && available < m.min_stock,
            negative: balance < -0.0001,
        });
    }
    out
}

/// Barcha omborlar bo'yicha umumiy qoldiq.
pub fn stock_balances(
    materials: &[Material],
    moves: &[StockMove],
    reservations: &[Reservation],
    today: NaiveDate,
) -> Vec<StockLine> {
    stock_balances_in(materials, moves, reservations, today, None)
}

/// Bitta partiyaning qoldig'i (TZ XI.9).
#[derive(Debug, Clone)]
pub struct BatchLine {
    pub batch_id: i64,
    pub material_id: i64,
    pub incoming: f64,
    pub outgoing: f64,
    pub balance: f64,
    /// Yaroqlilik muddati o'tgan.
    pub expired: bool,
    /// FEFO/FIFO tartibida navbatdagi partiya.
    pub next_to_use: bool,
}

/// Partiyalar bo'yicha qoldiq va navbat (TZ XI.28: FIFO / FEFO).
///
/// Navbat FEFO bo'yicha: yaroqlilik muddati birinchi tugaydigan partiya
/// birinchi ishlatiladi. Muddat ko'rsatilmagan partiyalarda FIFO — qaysi
/// birinchi kelgan bo'lsa, o'sha.
pub fn batch_balances(batches: &[Batch], moves: &[StockMove], today: NaiveDate) -> Vec<BatchLine> {
    let mut out: Vec<BatchLine> = batches
        .iter()
        .map(|b| {
            let mine: Vec<&StockMove> = moves.iter().filter(|m| m.batch_id == Some(b.id)).collect();
            let incoming: f64 = mine
                .iter()
                .filter(|m| adds_to_stock(m.kind))
                .map(|m| m.qty)
                .sum();
            let outgoing: f64 = mine
                .iter()
                .filter(|m| !adds_to_stock(m.kind))
                .map(|m| m.qty)
                .sum();
            BatchLine {
                batch_id: b.id,
                material_id: b.material_id,
                incoming,
                outgoing,
                balance: incoming - outgoing,
                expired: b.expires.is_some_and(|d| d < today),
                next_to_use: false,
            }
        })
        .collect();

    // Har material bo'yicha navbatdagi partiyani belgilaymiz.
    let mut materials: Vec<i64> = out.iter().map(|l| l.material_id).collect();
    materials.sort_unstable();
    materials.dedup();
    for mid in materials {
        let mut candidates: Vec<(usize, Option<NaiveDate>, NaiveDate, i64)> = out
            .iter()
            .enumerate()
            .filter(|(_, l)| l.material_id == mid && l.balance > 0.0)
            .filter_map(|(i, l)| {
                let b = batches.iter().find(|b| b.id == l.batch_id)?;
                Some((i, b.expires, b.received, b.id))
            })
            .collect();
        // Muddati borlar oldinda; keyin kelgan sana; oxirida id — tartib barqaror.
        candidates.sort_by(|a, b| match (a.1, b.1) {
            (Some(x), Some(y)) => x.cmp(&y).then(a.2.cmp(&b.2)).then(a.3.cmp(&b.3)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.2.cmp(&b.2).then(a.3.cmp(&b.3)),
        });
        if let Some((i, _, _, _)) = candidates.first() {
            out[*i].next_to_use = true;
        }
    }
    out
}

/// Inventarizatsiya farqlari (TZ XI.25).
#[derive(Debug, Clone)]
pub struct InventoryDiff {
    pub material_id: i64,
    pub book: f64,
    pub fact: f64,
    pub diff: f64,
}

/// Nolga teng bo'lmagan farqlar. Bo'sh ro'yxat — hamma narsa joyida.
pub fn inventory_diffs(lines: &[InventoryLine], inventory_id: i64) -> Vec<InventoryDiff> {
    lines
        .iter()
        .filter(|l| l.inventory_id == inventory_id)
        .filter(|l| l.diff().abs() > 0.0001)
        .map(|l| InventoryDiff {
            material_id: l.material_id,
            book: l.book,
            fact: l.fact,
            diff: l.diff(),
        })
        .collect()
}

/// Bitta ish + material bo'yicha normativ va haqiqiy sarf (TZ XI.14–15, XII.21–22).
#[derive(Debug, Clone)]
pub struct ConsumptionLine {
    pub task_id: i64,
    pub material_id: i64,
    /// Bajarilgan hajm — ish hajmining progress ulushi.
    pub done_volume: f64,
    /// Normativ sarf: `per_unit × bajarilgan hajm`.
    pub norm: f64,
    /// Haqiqiy sarf: shu ishga berilgan material.
    pub fact: f64,
    /// Fakt minus norma. Manfiy — tejalgan.
    pub diff: f64,
    /// Normadan foizda og'ish. Norma nol bo'lsa — 0.
    pub diff_pct: f64,
    /// Ruxsat etilgan chegaradan oshgan.
    pub over: bool,
    /// Ortiqcha sarfning puldagi qiymati (faqat oshgan qismi).
    pub over_cost: f64,
}

/// TZ XI.14–15: normativ sarf bilan haqiqiy sarfni solishtirish.
///
/// Normativ sarf **bajarilgan** hajmga qarab hisoblanadi, rejadagi hajmga emas:
/// ish yarim bitgan bo'lsa, materialning ham yarmi ketishi kerak. Aks holda har
/// bir tugallanmagan ish «tejab ishlayapti» ko'rinib qolardi.
///
/// Ortiqcha sarf normaning `tolerance` foizidan oshganda belgilanadi —
/// texnologik yo'qotish (kesim qoldig'i, to'kilish) normal hisoblanadi.
pub fn consumption(
    norms: &[MaterialNorm],
    tasks: &[Task],
    materials: &[Material],
    moves: &[StockMove],
) -> Vec<ConsumptionLine> {
    norms
        .iter()
        .filter_map(|n| {
            let task = tasks.iter().find(|t| t.id == n.task_id)?;
            let done_volume = task.volume * (task.progress / 100.0);
            let norm = n.per_unit * done_volume;
            // Haqiqiy sarf — shu ishga berilgan va hisobdan chiqarilgan material,
            // qaytarilgani ayriladi.
            let fact: f64 = moves
                .iter()
                .filter(|m| m.task_id == Some(n.task_id) && m.material_id == n.material_id)
                .map(|m| match m.kind {
                    MoveKind::Out | MoveKind::WriteOff => m.qty,
                    MoveKind::Return => -m.qty,
                    MoveKind::In => 0.0,
                })
                .sum();
            let diff = fact - norm;
            let diff_pct = if norm > 0.0 { diff / norm * 100.0 } else { 0.0 };
            let price = materials
                .iter()
                .find(|m| m.id == n.material_id)
                .map_or(0.0, |m| m.price);
            let allowed = norm * (1.0 + n.tolerance / 100.0);
            let over = fact > allowed + 0.0001;
            Some(ConsumptionLine {
                task_id: n.task_id,
                material_id: n.material_id,
                done_volume,
                norm,
                fact,
                diff,
                diff_pct,
                over,
                over_cost: if over { (fact - allowed) * price } else { 0.0 },
            })
        })
        .collect()
}

// ================= XIII. Tabel: ish haqi, brigada, tannarx =================

/// Bir kunda shu soatdan oshgani ortiqcha ish deb belgilanadi (TZ XIII.13).
pub const NORM_HOURS: f64 = 8.0;

/// Ortiqcha ish soatiga qo'shimcha haq. Koeffitsiyent kodda turibdi va ekranda
/// ochiq yozilgan — buni tashkilot o'z ichki hujjati bilan belgilaydi.
pub const OVERTIME_RATE: f64 = 1.5;

/// Bitta ishchining davr bo'yicha tabel yakuni (TZ XIII.29).
#[derive(Debug, Clone, Default)]
pub struct WageLine {
    pub worker_id: i64,
    /// Jami tabelga yozilgan soat.
    pub hours: f64,
    /// Haqiqatda ishlangan soat (bo'sh turish va yo'qlik kirmaydi).
    pub worked_hours: f64,
    /// Normadan oshgan soat.
    pub overtime_hours: f64,
    /// Tungi smenadagi soat.
    pub night_hours: f64,
    /// Bo'sh turish soati (TZ XIII.22).
    pub downtime_hours: f64,
    /// Yo'qlik kunlari (ta'til, kasallik, safar, sababsiz).
    pub absence_days: i64,
    /// Sababsiz yo'qlik kunlari — alohida ko'rsatiladi.
    pub absent_days: i64,
    /// Hisoblangan ish haqi.
    pub wage: f64,
}

/// TZ XIII.29: tabeldan ish haqi.
///
/// Har bir kun o'z smenasi va turi bilan hisoblanadi:
/// normadagi soat `stavka × smena koeffitsiyenti`, undan oshgani esa
/// yana `OVERTIME_RATE` ga ko'paytiriladi. To'lanmaydigan kun turlari
/// (ta'til, kasallik, sababsiz yo'qlik) summaga kirmaydi — ular
/// buxgalteriyada boshqacha hisoblanadi.
pub fn wages(
    workers: &[Worker],
    entries: &[TimesheetEntry],
    from: NaiveDate,
    to: NaiveDate,
) -> Vec<WageLine> {
    workers
        .iter()
        .map(|w| {
            let mut l = WageLine {
                worker_id: w.id,
                ..Default::default()
            };
            for e in entries
                .iter()
                .filter(|e| e.worker_id == w.id && e.date >= from && e.date <= to)
            {
                l.hours += e.hours;
                if e.kind.worked() {
                    l.worked_hours += e.hours;
                }
                if e.kind == DayKind::Downtime {
                    l.downtime_hours += e.hours;
                }
                if e.kind.absence() {
                    l.absence_days += 1;
                    if e.kind == DayKind::Absent {
                        l.absent_days += 1;
                    }
                }
                if e.shift == Shift::Night {
                    l.night_hours += e.hours;
                }
                let over = (e.hours - NORM_HOURS).max(0.0);
                l.overtime_hours += over;
                if e.kind.paid() {
                    let base = e.hours - over;
                    l.wage += (base + over * OVERTIME_RATE) * e.shift.rate() * w.hourly_rate;
                }
            }
            l
        })
        .collect()
}

/// Brigadalarni solishtirish uchun yakun (TZ XIII.24–25).
#[derive(Debug, Clone, Default)]
pub struct BrigadeLine {
    pub brigade_id: i64,
    pub workers: usize,
    pub hours: f64,
    pub worked_hours: f64,
    pub downtime_hours: f64,
    pub wage: f64,
    /// Bo'sh turish ulushi, foizda — brigada qanchalik uzilishsiz ishlaganini
    /// ko'rsatadi.
    pub downtime_pct: f64,
    /// Bir soatning o'rtacha tannarxi.
    pub cost_per_hour: f64,
}

pub fn brigade_lines(
    brigades: &[Brigade],
    workers: &[Worker],
    entries: &[TimesheetEntry],
    from: NaiveDate,
    to: NaiveDate,
) -> Vec<BrigadeLine> {
    let all = wages(workers, entries, from, to);
    brigades
        .iter()
        .map(|b| {
            let mut l = BrigadeLine {
                brigade_id: b.id,
                ..Default::default()
            };
            for w in workers.iter().filter(|w| w.brigade_id == Some(b.id)) {
                l.workers += 1;
                if let Some(x) = all.iter().find(|x| x.worker_id == w.id) {
                    l.hours += x.hours;
                    l.worked_hours += x.worked_hours;
                    l.downtime_hours += x.downtime_hours;
                    l.wage += x.wage;
                }
            }
            l.downtime_pct = if l.hours > 0.0 {
                l.downtime_hours / l.hours * 100.0
            } else {
                0.0
            };
            l.cost_per_hour = if l.worked_hours > 0.0 {
                l.wage / l.worked_hours
            } else {
                0.0
            };
            l
        })
        .collect()
}

/// Aniq ishning tannarxi (TZ XIII.30–31).
#[derive(Debug, Clone)]
pub struct TaskCost {
    pub task_id: i64,
    pub hours: f64,
    /// Tabeldan yig'ilgan ish haqi.
    pub labour: f64,
    /// Shu ishga berilgan materialning qiymati.
    pub material: f64,
    /// Shu ishda ishlagan texnika xarajati (TZ XVI.35).
    pub machine: f64,
    pub total: f64,
    /// Bir birlik ish hajmining tannarxi. Hajm nol bo'lsa — 0.
    pub per_unit: f64,
}

/// Ish tannarxi: tabeldagi soat va omborga berilgan material.
///
/// Ikkalasi ham allaqachon kiritilgan ma'lumotdan yig'iladi — tannarx alohida
/// kiritilmaydi, shuning uchun u hujjatlar bilan hech qachon zid bo'lmaydi.
pub fn task_costs(
    tasks: &[Task],
    workers: &[Worker],
    entries: &[TimesheetEntry],
    materials: &[Material],
    moves: &[StockMove],
    machines: &[Machine],
    logs: &[MachineLog],
) -> Vec<TaskCost> {
    tasks
        .iter()
        .map(|t| {
            let mut hours = 0.0;
            let mut labour = 0.0;
            for e in entries.iter().filter(|e| e.task_id == Some(t.id)) {
                hours += e.hours;
                if !e.kind.paid() {
                    continue;
                }
                let rate = workers
                    .iter()
                    .find(|w| w.id == e.worker_id)
                    .map_or(0.0, |w| w.hourly_rate);
                let over = (e.hours - NORM_HOURS).max(0.0);
                labour += (e.hours - over + over * OVERTIME_RATE) * e.shift.rate() * rate;
            }
            let material: f64 = moves
                .iter()
                .filter(|m| m.task_id == Some(t.id))
                .filter(|m| matches!(m.kind, MoveKind::Out | MoveKind::WriteOff))
                .map(|m| {
                    let price = if m.price > 0.0 {
                        m.price
                    } else {
                        materials
                            .iter()
                            .find(|x| x.id == m.material_id)
                            .map_or(0.0, |x| x.price)
                    };
                    m.qty * price
                })
                .sum();
            // Texnika: shu ishda ishlagan motosoat × soatlik stavka.
            let machine: f64 = logs
                .iter()
                .filter(|l| l.task_id == Some(t.id))
                .map(|l| {
                    let rate = machines
                        .iter()
                        .find(|m| m.id == l.machine_id)
                        .map_or(0.0, |m| m.hour_rate);
                    l.hours * rate
                })
                .sum();
            let total = labour + material + machine;
            TaskCost {
                task_id: t.id,
                hours,
                labour,
                material,
                machine,
                total,
                per_unit: if t.volume > 0.0 {
                    total / t.volume
                } else {
                    0.0
                },
            }
        })
        .filter(|c| c.hours > 0.0 || c.total > 0.0)
        .collect()
}

// ================= X. Yetkazib beruvchilar, KP, byudjet =================

/// Narx katalogdagidan shuncha foizga farq qilsa — anomaliya (TZ X.13).
pub const PRICE_ANOMALY_PCT: f64 = 20.0;

/// Yetkazib beruvchining xaridlardan hisoblangan tarixi (TZ X.8, 40).
#[derive(Debug, Clone, Default)]
pub struct SupplierLine {
    pub supplier: String,
    pub orders: usize,
    pub amount: f64,
    /// Muddatida yetkazilgan buyurtmalar ulushi, foizda.
    pub on_time_pct: f64,
    /// O'rtacha kechikish, kunlarda (faqat kechikkanlar bo'yicha).
    pub avg_delay: f64,
    pub last_order: Option<NaiveDate>,
    /// To'liq yetkazilmagan buyurtmalar soni.
    pub open_orders: usize,
}

/// Yetkazib beruvchilar bo'yicha yakun.
///
/// Ro'yxat kartochkalardan emas, xaridlardan yig'iladi: kartochkasi yo'q
/// yetkazib beruvchi ham chetda qolmasligi kerak.
pub fn supplier_lines(purchases: &[Purchase], today: NaiveDate) -> Vec<SupplierLine> {
    let mut names: Vec<String> = purchases
        .iter()
        .map(|p| p.supplier.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    names.sort();
    names.dedup();

    names
        .into_iter()
        .map(|name| {
            let mine: Vec<&Purchase> = purchases
                .iter()
                .filter(|p| p.supplier.trim() == name)
                .collect();
            // Kechikish: yetkazilgan buyurtmada yopilish sanasi bilan emas,
            // rejadagi sana bilan bugungi kun solishtiriladi — yopilmagan
            // buyurtma ham kechikkan hisoblanadi.
            let mut on_time = 0usize;
            let mut delays: Vec<i64> = Vec::new();
            for p in &mine {
                let late = if p.fully_delivered() {
                    false
                } else {
                    p.delivery_date < today
                };
                if late {
                    delays.push((today - p.delivery_date).num_days());
                } else {
                    on_time += 1;
                }
            }
            SupplierLine {
                orders: mine.len(),
                amount: mine.iter().map(|p| p.amount()).sum(),
                on_time_pct: if mine.is_empty() {
                    0.0
                } else {
                    on_time as f64 / mine.len() as f64 * 100.0
                },
                avg_delay: if delays.is_empty() {
                    0.0
                } else {
                    delays.iter().sum::<i64>() as f64 / delays.len() as f64
                },
                last_order: mine.iter().map(|p| p.date).max(),
                open_orders: mine.iter().filter(|p| !p.fully_delivered()).count(),
                supplier: name,
            }
        })
        .collect()
}

/// Bitta tijorat taklifining bahosi (TZ X.11–12).
#[derive(Debug, Clone)]
pub struct QuoteLine {
    pub quote_id: i64,
    pub amount: f64,
    /// Eng arzon taklifdan qancha qimmat, foizda. Eng arzonida 0.
    pub over_best_pct: f64,
    /// Eng arzon taklif.
    pub cheapest: bool,
    /// Eng tez yetkazadigan taklif.
    pub fastest: bool,
    /// Amal qilish muddati o'tgan.
    pub expired: bool,
}

/// Takliflarni ariza kesimida solishtiradi.
///
/// «Eng yaxshi» deb bitta taklif tanlanmaydi: eng arzoni va eng tezi alohida
/// belgilanadi, qaror odamniki bo'lib qoladi — muddat va narx orasidagi
/// muvozanatni faqat loyihani biladigan odam tanlay oladi.
pub fn quote_lines(quotes: &[Quote], today: NaiveDate) -> Vec<QuoteLine> {
    let mut out: Vec<QuoteLine> = quotes
        .iter()
        .map(|q| QuoteLine {
            quote_id: q.id,
            amount: q.amount(),
            over_best_pct: 0.0,
            cheapest: false,
            fastest: false,
            expired: q.valid_until.is_some_and(|d| d < today),
        })
        .collect();

    let mut groups: Vec<Option<i64>> = quotes.iter().map(|q| q.request_id).collect();
    groups.sort();
    groups.dedup();
    for g in groups {
        let idx: Vec<usize> = quotes
            .iter()
            .enumerate()
            .filter(|(_, q)| q.request_id == g)
            .map(|(i, _)| i)
            .collect();
        let best = idx
            .iter()
            .filter(|i| !out[**i].expired)
            .map(|i| out[*i].amount)
            .filter(|a| *a > 0.0)
            .fold(f64::INFINITY, f64::min);
        let quick = idx
            .iter()
            .filter(|i| !out[**i].expired)
            .map(|i| quotes[*i].delivery_days)
            .min();
        for i in idx {
            if out[i].expired {
                continue;
            }
            if best.is_finite() && best > 0.0 {
                out[i].over_best_pct = (out[i].amount - best) / best * 100.0;
                out[i].cheapest = (out[i].amount - best).abs() < 0.01;
            }
            out[i].fastest = quick.is_some_and(|d| quotes[i].delivery_days == d);
        }
    }
    out
}

/// Xarid narxi katalog narxidan keskin farq qilsa — belgilanadi (TZ X.13–14).
///
/// Farq har doim ham xato emas: bozor narxi o'zgargan bo'lishi mumkin.
/// Shuning uchun bu taqiq emas, e'tiborni tortadigan belgi.
pub fn price_anomaly(price: f64, catalog: f64) -> Option<f64> {
    if price <= 0.0 || catalog <= 0.0 {
        return None;
    }
    let pct = (price - catalog) / catalog * 100.0;
    (pct.abs() >= PRICE_ANOMALY_PCT).then_some(pct)
}

/// Bo'lim bo'yicha xarid byudjeti holati (TZ X.34–35).
#[derive(Debug, Clone)]
pub struct BudgetLine {
    pub section: Section,
    pub planned: f64,
    /// Buyurtma qilingan summa (barcha xaridlar).
    pub ordered: f64,
    /// Yetkazilgan qismning summasi.
    pub delivered: f64,
    /// Rejadan qolgan. Manfiy — byudjet oshib ketgan.
    pub left: f64,
    pub used_pct: f64,
    pub over: bool,
}

/// Byudjet va haqiqiy xaridlar. Byudjeti belgilanmagan bo'lim ham chiqadi —
/// unda reja nol bo'lib, sarflangani ko'rinib turadi.
pub fn budget_lines(budgets: &[PurchaseBudget], purchases: &[Purchase]) -> Vec<BudgetLine> {
    let mut sections: Vec<Section> = budgets.iter().map(|b| b.section).collect();
    sections.extend(purchases.iter().map(|p| p.section));
    sections.sort_by_key(|s| s.code());
    sections.dedup();

    sections
        .into_iter()
        .map(|section| {
            let planned: f64 = budgets
                .iter()
                .filter(|b| b.section == section)
                .map(|b| b.planned)
                .sum();
            let mine: Vec<&Purchase> = purchases.iter().filter(|p| p.section == section).collect();
            let ordered: f64 = mine.iter().map(|p| p.amount()).sum();
            let delivered: f64 = mine.iter().map(|p| p.delivered_qty * p.price).sum();
            BudgetLine {
                section,
                planned,
                ordered,
                delivered,
                left: planned - ordered,
                used_pct: if planned > 0.0 {
                    ordered / planned * 100.0
                } else {
                    0.0
                },
                over: planned > 0.0 && ordered > planned + 0.01,
            }
        })
        .collect()
}

// ================= IX. Kelishuv marshruti va limitlar =================

/// Shu summagacha bir kishi — prorab — kelishadi (TZ IX.9).
pub const APPROVAL_LIMIT_1: f64 = 10_000_000.0;

/// Shu summagacha loyiha rahbari ham kelishadi.
pub const APPROVAL_LIMIT_2: f64 = 100_000_000.0;

/// Ariza summasiga qarab kelishuv marshruti (TZ IX.8–9).
///
/// Limitlar kodda turibdi va ekranda ochiq yozilgan: kichik ariza uchun uzun
/// marshrut ish sekinlashtiradi, katta summa esa bir kishining qaroriga
/// qoldirilmasligi kerak.
pub fn approval_route(amount: f64) -> Vec<Role> {
    if amount <= APPROVAL_LIMIT_1 {
        vec![Role::Foreman]
    } else if amount <= APPROVAL_LIMIT_2 {
        vec![Role::Foreman, Role::ProjectManager]
    } else {
        vec![Role::Foreman, Role::ProjectManager, Role::Director]
    }
}

/// Arizaning kelishuv holati.
#[derive(Debug, Clone, PartialEq)]
pub enum RouteState {
    /// Marshrut hali qurilmagan.
    None,
    /// Navbatdagi bosqich: kim kutilmoqda.
    Waiting { step: i64, role: Role },
    /// Hamma bosqich kelishdi.
    Approved,
    /// Biror bosqichda rad etilgan.
    Rejected { step: i64, role: Role },
}

/// Arizaning kelishuv holati: kim navbatda, kelishildimi yoki rad etildimi.
///
/// Bosqichlar tartib bilan o'tiladi — oldingisi kelishmaguncha keyingisi
/// navbat kutadi. Bitta rad etish butun marshrutni to'xtatadi.
pub fn route_state(request_id: i64, approvals: &[Approval]) -> RouteState {
    let mut mine: Vec<&Approval> = approvals
        .iter()
        .filter(|a| a.request_id == request_id)
        .collect();
    if mine.is_empty() {
        return RouteState::None;
    }
    mine.sort_by_key(|a| a.step);

    for a in &mine {
        match a.decision {
            ApprovalDecision::Rejected => {
                return RouteState::Rejected {
                    step: a.step,
                    role: Role::parse(&a.role),
                }
            }
            ApprovalDecision::Pending => {
                return RouteState::Waiting {
                    step: a.step,
                    role: Role::parse(&a.role),
                }
            }
            ApprovalDecision::Approved => continue,
        }
    }
    RouteState::Approved
}

/// Ariza summasi: miqdor × katalogdagi narx. Narx yo'q bo'lsa 0.
pub fn request_amount(request: &Request, materials: &[Material]) -> f64 {
    request
        .material_id
        .and_then(|id| materials.iter().find(|m| m.id == id))
        .map_or(0.0, |m| request.qty * m.price)
}

/// Ariza byudjetga sig'adimi (TZ IX.10).
///
/// `None` — tekshiradigan byudjet yo'q. `Some(qolgan)` — shu bo'limda
/// buyurtmalardan keyin qolgan summa; manfiy bo'lsa ariza byudjetdan oshadi.
pub fn request_budget_left(
    request: &Request,
    materials: &[Material],
    budgets: &[PurchaseBudget],
    purchases: &[Purchase],
) -> Option<f64> {
    let section = request
        .material_id
        .and_then(|id| materials.iter().find(|m| m.id == id))
        .map(|m| m.section)?;
    let line = budget_lines(budgets, purchases)
        .into_iter()
        .find(|l| l.section == section)?;
    (line.planned > 0.0).then(|| line.left - request_amount(request, materials))
}

// ================= XIV. Sifat: ball, bloklash, brak tahlili =================

/// Ish shu foizdan oshgan bo'lsa «yopilmoqda» deb hisoblaymiz (TZ XIV.35).
pub const CLOSING_PROGRESS: f64 = 95.0;

/// Ishni yopishga to'sqinlik qiladigan sabab (TZ XIV.10, 35).
#[derive(Debug, Clone)]
pub struct TaskBlock {
    pub task_id: i64,
    /// Yopilmagan nuqsonlar soni.
    pub open_defects: usize,
    /// Shundan muddati o'tganlari.
    pub overdue: usize,
    /// To'ldirilmagan nazorat nuqtalari.
    pub pending_points: usize,
    /// Qabul nazorati umuman o'tkazilmagan.
    pub no_acceptance: bool,
}

impl TaskBlock {
    /// Ishni yopish mumkin emas.
    pub fn blocked(&self) -> bool {
        self.open_defects > 0 || self.pending_points > 0 || self.no_acceptance
    }

    /// Jiddiy to'siq: nuqson bartaraf etilmagan.
    ///
    /// Qabul nazorati yozilmagani ham to'siq, lekin u boshqa xil muammo —
    /// hujjat yetishmaydi, ish esa buzuq emas. Ikkalasini bir xil ko'rsatish
    /// haqiqiy nuqsonni ko'zdan yashiradi.
    pub fn severe(&self) -> bool {
        self.open_defects > 0
    }
}

/// TZ XIV.10, 35: yopilishga yaqin ishlarni sifat bo'yicha tekshiradi.
///
/// Bloklash — taqiq emas, ogohlantirish: dastur ishni yopishga ruxsat berishi
/// yoki bermasligi tashkiliy qaror, lekin nima yopilmaganini aytib turishi
/// shart. Aks holda nuqson bosqich ostida ko'milib qoladi.
pub fn task_blocks(
    tasks: &[Task],
    quality: &[QualityCheck],
    points: &[CheckPoint],
    today: NaiveDate,
) -> Vec<TaskBlock> {
    tasks
        .iter()
        .filter(|t| t.progress >= CLOSING_PROGRESS)
        .map(|t| {
            let mine: Vec<&QualityCheck> =
                quality.iter().filter(|q| q.task_id == Some(t.id)).collect();
            let open: Vec<&&QualityCheck> = mine.iter().filter(|q| q.open_defect()).collect();
            let ids: Vec<i64> = mine.iter().map(|q| q.id).collect();
            TaskBlock {
                task_id: t.id,
                open_defects: open.len(),
                overdue: open
                    .iter()
                    .filter(|q| q.deadline.is_some_and(|d| d < today))
                    .count(),
                pending_points: points
                    .iter()
                    .filter(|p| ids.contains(&p.check_id))
                    .filter(|p| p.result == PointResult::Pending)
                    .count(),
                no_acceptance: !mine.iter().any(|q| q.kind == QualityKind::Acceptance),
            }
        })
        .filter(|b| b.blocked())
        .collect::<Vec<_>>()
        .into_iter()
        // Haqiqiy nuqsonlar yuqorida: ular birinchi navbatdagi ish.
        .fold(Vec::new(), |mut acc, b| {
            let pos = acc
                .iter()
                .position(|x: &TaskBlock| !x.severe() && b.severe())
                .unwrap_or(acc.len());
            acc.insert(pos, b);
            acc
        })
}

/// Takrorlanuvchi nuqson (TZ XIV.30–31).
#[derive(Debug, Clone)]
pub struct DefectGroup {
    /// Nuqson matni — solishtirish uchun tartibga solingan holda.
    pub defect: String,
    pub count: usize,
    /// Hali yopilmaganlari.
    pub open: usize,
    pub last: Option<NaiveDate>,
    /// Qaysi ishlarda uchradi.
    pub tasks: Vec<i64>,
}

/// Brak sabablarini guruhlaydi: bir xil nuqson necha marta takrorlangan.
///
/// Solishtirish uchun matn kichik harflarga o'tkaziladi va ortiqcha bo'shliq
/// olib tashlanadi — «Beton kavakligi» va «beton  kavakligi» bitta sabab.
pub fn defect_groups(quality: &[QualityCheck]) -> Vec<DefectGroup> {
    let key = |s: &str| {
        s.to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let mut keys: Vec<String> = quality
        .iter()
        .filter(|q| q.result != QualityResult::Pass)
        .map(|q| key(&q.defect))
        .filter(|s| !s.is_empty())
        .collect();
    keys.sort();
    keys.dedup();

    let mut out: Vec<DefectGroup> = keys
        .into_iter()
        .map(|k| {
            let mine: Vec<&QualityCheck> = quality
                .iter()
                .filter(|q| q.result != QualityResult::Pass && key(&q.defect) == k)
                .collect();
            let mut tasks: Vec<i64> = mine.iter().filter_map(|q| q.task_id).collect();
            tasks.sort_unstable();
            tasks.dedup();
            DefectGroup {
                // Ko'rsatish uchun birinchi uchragan yozuvdagi matn olinadi.
                defect: mine
                    .first()
                    .map(|q| q.defect.trim().to_string())
                    .unwrap_or_else(|| k.clone()),
                count: mine.len(),
                open: mine.iter().filter(|q| q.open_defect()).count(),
                last: mine.iter().map(|q| q.date).max(),
                tasks,
            }
        })
        .collect();
    // Ko'p takrorlangani yuqorida.
    out.sort_by(|a, b| b.count.cmp(&a.count).then(b.open.cmp(&a.open)));
    out
}

/// Sifat balli (TZ XIV.33).
#[derive(Debug, Clone)]
pub struct QualityScore {
    /// 0..100.
    pub score: f64,
    pub checks: usize,
    pub passed: usize,
    pub conditional: usize,
    pub failed: usize,
    /// Yopilmagan nuqsonlar.
    pub open: usize,
    /// Muddati o'tgan nuqsonlar.
    pub overdue: usize,
}

/// TZ XIV.33: sifat balli — o'tgan tekshiruvlar ulushidan ochiq nuqsonlar
/// jarimasi ayrilgan.
///
/// Shartli o'tgan tekshiruv yarim ball beradi: nuqson bor, lekin ish
/// to'xtamagan. Yopilmagan har bir nuqson 2 ball, muddati o'tgani yana
/// 3 ball oladi — vaqt o'tgani sari ball o'zi pasayadi va e'tibor tortadi.
/// Tekshiruv umuman bo'lmasa ball qo'yilmaydi: nol tekshiruv «yaxshi» degani
/// emas, shuning uchun bunday holda 100 emas, 0 qaytadi va `checks` nol bo'ladi.
pub fn quality_score(quality: &[QualityCheck], today: NaiveDate) -> QualityScore {
    let checks = quality.len();
    let passed = quality
        .iter()
        .filter(|q| q.result == QualityResult::Pass)
        .count();
    let conditional = quality
        .iter()
        .filter(|q| q.result == QualityResult::Conditional)
        .count();
    let failed = quality
        .iter()
        .filter(|q| q.result == QualityResult::Fail)
        .count();
    let open = quality.iter().filter(|q| q.open_defect()).count();
    let overdue = quality
        .iter()
        .filter(|q| q.open_defect() && q.deadline.is_some_and(|d| d < today))
        .count();

    let score = if checks == 0 {
        0.0
    } else {
        let base = (passed as f64 + conditional as f64 * 0.5) / checks as f64 * 100.0;
        (base - open as f64 * 2.0 - overdue as f64 * 3.0).clamp(0.0, 100.0)
    };

    QualityScore {
        score,
        checks,
        passed,
        conditional,
        failed,
        open,
        overdue,
    }
}

// ================= XV. Xavfsizlik: ruxsat, SIZ, naryad, ball =================

/// Ruxsat shu kundan kam qolganda «tugayapti» deb belgilanadi (TZ XV.7).
pub const PERMIT_WARN_DAYS: i64 = 30;

/// Bitta ishchining ruxsat va SIZ holati (TZ XV.4–5, 8–9).
#[derive(Debug, Clone)]
pub struct WorkerSafety {
    pub worker_id: i64,
    /// Amal qiladigan ruxsatlar.
    pub valid: Vec<PermitKind>,
    /// Muddati o'tganlari.
    pub expired: Vec<PermitKind>,
    /// Tez orada tugaydiganlari.
    pub expiring: Vec<PermitKind>,
    /// Berilmagan majburiy SIZ.
    pub ppe_missing: Vec<PpeItem>,
    /// Muddati o'tgan SIZ.
    pub ppe_expired: Vec<PpeItem>,
}

impl WorkerSafety {
    /// Ishchini ishga qo'yish mumkin emas: kirish instruktaji yo'q yoki
    /// majburiy SIZ berilmagan.
    pub fn blocked(&self) -> bool {
        !self.valid.contains(&PermitKind::Induction) || !self.ppe_missing.is_empty()
    }

    /// Shu turdagi ishga ruxsati bormi.
    pub fn allows(&self, kind: PermitKind) -> bool {
        self.valid.contains(&kind)
    }
}

/// Har bir ishchi bo'yicha ruxsat va SIZ holati.
pub fn worker_safety(
    workers: &[Worker],
    permits: &[WorkerPermit],
    ppe: &[PpeIssue],
    today: NaiveDate,
) -> Vec<WorkerSafety> {
    workers
        .iter()
        .map(|w| {
            let mine: Vec<&WorkerPermit> = permits.iter().filter(|p| p.worker_id == w.id).collect();
            let mut valid = Vec::new();
            let mut expired = Vec::new();
            let mut expiring = Vec::new();
            for k in PermitKind::ALL {
                let best = mine
                    .iter()
                    .filter(|p| p.kind == *k)
                    .max_by_key(|p| p.valid_until);
                match best {
                    None => {}
                    Some(p) if p.expired(today) => expired.push(*k),
                    Some(p) => {
                        valid.push(*k);
                        if p.expires_soon(today, PERMIT_WARN_DAYS) {
                            expiring.push(*k);
                        }
                    }
                }
            }

            // SIZ: har bir majburiy buyum berilgan va muddati o'tmagan bo'lishi kerak.
            let issued: Vec<&PpeIssue> = ppe.iter().filter(|p| p.worker_id == w.id).collect();
            let mut ppe_missing = Vec::new();
            let mut ppe_expired = Vec::new();
            for item in PpeItem::REQUIRED {
                let best = issued
                    .iter()
                    .filter(|p| p.item == *item)
                    .max_by_key(|p| p.issued);
                match best {
                    None => ppe_missing.push(*item),
                    Some(p) if p.expired(today) => ppe_expired.push(*item),
                    Some(_) => {}
                }
            }

            WorkerSafety {
                worker_id: w.id,
                valid,
                expired,
                expiring,
                ppe_missing,
                ppe_expired,
            }
        })
        .collect()
}

/// Naryad-dopuskdagi kamchilik (TZ XV.12).
#[derive(Debug, Clone, PartialEq)]
pub enum PermitIssue {
    /// Bajaruvchi ko'rsatilmagan.
    NoWorkers,
    /// Chora-tadbirlar yozilmagan.
    NoMeasures,
    /// Mas'ul ko'rsatilmagan.
    NoIssuer,
    /// Muddat noto'g'ri: tugash sanasi boshlanishdan oldin.
    BadPeriod,
    /// Naryad muddati o'tgan, lekin yopilmagan.
    Overdue,
    /// Ishchining shu turdagi ishga ruxsati yo'q.
    WorkerNotAllowed(i64),
    /// Ishchining majburiy SIZ yetishmaydi.
    WorkerNoPpe(i64),
}

/// TZ XV.12: naryadni berishdan oldin tekshiradi.
///
/// Tekshiruv taqiq emas — ro'yxat. Lekin bo'sh ro'yxat bo'lmasa, naryadni
/// imzolashdan oldin nima yetishmayotgani ko'rinib turadi.
pub fn permit_issues(
    permit: &WorkPermit,
    safety: &[WorkerSafety],
    today: NaiveDate,
) -> Vec<PermitIssue> {
    let mut out = Vec::new();
    let ids = permit.worker_ids();
    if ids.is_empty() {
        out.push(PermitIssue::NoWorkers);
    }
    if permit.measures.trim().is_empty() {
        out.push(PermitIssue::NoMeasures);
    }
    if permit.issuer.trim().is_empty() || permit.supervisor.trim().is_empty() {
        out.push(PermitIssue::NoIssuer);
    }
    if permit.date_to < permit.date_from {
        out.push(PermitIssue::BadPeriod);
    }
    if permit.status == PermitStatus::Open && permit.date_to < today {
        out.push(PermitIssue::Overdue);
    }
    for id in ids {
        let Some(w) = safety.iter().find(|s| s.worker_id == id) else {
            continue;
        };
        if !w.allows(permit.kind) {
            out.push(PermitIssue::WorkerNotAllowed(id));
        }
        if !w.ppe_missing.is_empty() || !w.ppe_expired.is_empty() {
            out.push(PermitIssue::WorkerNoPpe(id));
        }
    }
    out
}

/// Xavfsizlik balli (TZ XV.33).
#[derive(Debug, Clone)]
pub struct SafetyScore {
    /// 0..100.
    pub score: f64,
    pub incidents: usize,
    pub violations: usize,
    pub near_misses: usize,
    /// Bartaraf etilmagan va muddati o'tgan chora-tadbirlar.
    pub overdue: usize,
    /// Muddati o'tgan ruxsatlar soni.
    pub expired_permits: usize,
    /// Majburiy SIZ yetishmayotgan ishchilar.
    pub without_ppe: usize,
    /// Kamchiligi bor ochiq naryadlar.
    pub bad_permits: usize,
}

/// TZ XV.33: xavfsizlik balli 100 dan boshlanadi va kamchiliklar ayriladi.
///
/// Hodisa eng og'ir jarima (25 ball): bir marta yuz bergan baxtsiz hodisa
/// oylik statistikadan muhimroq. Near miss kichik jarima (2 ball) — uni
/// yozganlik yaxshi, lekin takrorlanishi tizimli muammo.
pub fn safety_score(
    events: &[SafetyEvent],
    safety: &[WorkerSafety],
    permits: &[WorkPermit],
    today: NaiveDate,
) -> SafetyScore {
    // Oxirgi 90 kundagi hodisalar hisobga olinadi: eski buzilish bugungi
    // holatni belgilamaydi.
    let since = today - chrono::Duration::days(90);
    let recent: Vec<&SafetyEvent> = events.iter().filter(|e| e.date >= since).collect();
    let count = |k: SafetyKind| recent.iter().filter(|e| e.kind == k).count();

    let incidents = count(SafetyKind::Incident);
    let violations = count(SafetyKind::Violation);
    let near_misses = count(SafetyKind::NearMiss);
    let overdue = events
        .iter()
        .filter(|e| e.status != IssueStatus::Fixed && e.status != IssueStatus::Rejected)
        .filter(|e| e.deadline.is_some_and(|d| d < today))
        .count();
    let expired_permits: usize = safety.iter().map(|s| s.expired.len()).sum();
    let without_ppe = safety
        .iter()
        .filter(|s| !s.ppe_missing.is_empty() || !s.ppe_expired.is_empty())
        .count();
    let bad_permits = permits
        .iter()
        .filter(|p| p.status == PermitStatus::Open)
        .filter(|p| !permit_issues(p, safety, today).is_empty())
        .count();

    let penalty = incidents as f64 * 25.0
        + violations as f64 * 5.0
        + near_misses as f64 * 2.0
        + overdue as f64 * 4.0
        + expired_permits as f64 * 3.0
        + without_ppe as f64 * 3.0
        + bad_permits as f64 * 6.0;

    SafetyScore {
        score: (100.0 - penalty).clamp(0.0, 100.0),
        incidents,
        violations,
        near_misses,
        overdue,
        expired_permits,
        without_ppe,
        bad_permits,
    }
}

// ================= XVI. Texnika: foydalanish, yoqilg'i, TX =================

/// Bir smenadagi normal ish vaqti — foydalanish koeffitsiyenti shunga nisbatan.
pub const SHIFT_HOURS: f64 = 8.0;

/// Yoqilg'i normadan shu foizga oshsa — ortiqcha sarf (TZ XVI.18).
pub const FUEL_OVERUSE_PCT: f64 = 10.0;

/// Texnikani ishlatishga to'sqinlik (TZ XVI.27).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MachineBlock {
    /// Texnik ko'rik muddati o'tgan.
    Inspection,
    /// Rejali TX muddati o'tgan.
    Service,
    /// Ta'mirda turibdi.
    Repair,
}

/// Bitta texnikaning davr bo'yicha yakuni (TZ XVI.34–38).
#[derive(Debug, Clone)]
pub struct MachineLine {
    pub machine_id: i64,
    /// Ishlangan motosoat.
    pub hours: f64,
    /// Yurgan masofa.
    pub distance: f64,
    /// Sarflangan yoqilg'i.
    pub fuel: f64,
    /// Normativ yoqilg'i (norma × motosoat). Norma yo'q bo'lsa 0.
    pub fuel_norm: f64,
    /// Normadan ortiqcha sarf. Manfiy — tejalgan.
    pub fuel_diff: f64,
    /// Ortiqcha sarf chegaradan oshgan.
    pub fuel_over: bool,
    /// Xarajat: motosoat × soatlik stavka.
    pub cost: f64,
    /// Smena bo'lgan kunlar soni.
    pub work_days: usize,
    /// Foydalanish koeffitsiyenti, foizda: ishlangan soat / mavjud soat.
    pub utilization: f64,
    /// Bo'sh turgan kunlar.
    pub idle_days: usize,
    /// Keyingi TX gacha qolgan motosoat. Reja yo'q bo'lsa `None`.
    pub service_left: Option<f64>,
    /// Ishlatishga to'siq.
    pub blocks: Vec<MachineBlock>,
}

impl MachineLine {
    pub fn blocked(&self) -> bool {
        !self.blocks.is_empty()
    }
}

/// TZ XVI.34–38: texnika bo'yicha soat, yoqilg'i, xarajat va foydalanish.
///
/// Foydalanish koeffitsiyenti davrdagi **ish kunlariga** nisbatan hisoblanadi:
/// dam olish kunida turgan kran bo'sh turgan hisoblanmaydi, aks holda har bir
/// texnika bir xil «yomon» ko'rinardi.
pub fn machine_lines(
    machines: &[Machine],
    logs: &[MachineLog],
    from: NaiveDate,
    to: NaiveDate,
    today: NaiveDate,
) -> Vec<MachineLine> {
    // Davrdagi ish kunlari (yakshanbadan boshqa hammasi).
    let mut work_days_total = 0usize;
    let mut d = from;
    while d <= to {
        if chrono::Datelike::weekday(&d) != chrono::Weekday::Sun {
            work_days_total += 1;
        }
        d += chrono::Duration::days(1);
    }
    let available = work_days_total as f64 * SHIFT_HOURS;

    machines
        .iter()
        .map(|m| {
            let mine: Vec<&MachineLog> = logs
                .iter()
                .filter(|l| l.machine_id == m.id && l.date >= from && l.date <= to)
                .collect();
            let hours: f64 = mine.iter().map(|l| l.hours).sum();
            let fuel: f64 = mine.iter().map(|l| l.fuel).sum();
            let fuel_norm = m.fuel_norm * hours;
            let fuel_diff = fuel - fuel_norm;
            let mut days: Vec<NaiveDate> = mine.iter().map(|l| l.date).collect();
            days.sort_unstable();
            days.dedup();

            // Rejali TX: umumiy motosoatdan hisoblanadi, davrdan emas.
            let total_hours: f64 = logs
                .iter()
                .filter(|l| l.machine_id == m.id)
                .map(|l| l.hours)
                .sum();
            let service_left =
                (m.service_hours > 0.0).then_some(m.service_done + m.service_hours - total_hours);

            let mut blocks = Vec::new();
            if m.inspection_until.is_some_and(|d| d < today) {
                blocks.push(MachineBlock::Inspection);
            }
            if service_left.is_some_and(|v| v <= 0.0) {
                blocks.push(MachineBlock::Service);
            }
            if m.status == MachineStatus::Repair {
                blocks.push(MachineBlock::Repair);
            }

            MachineLine {
                machine_id: m.id,
                hours,
                distance: mine.iter().map(|l| l.distance()).sum(),
                fuel,
                fuel_norm,
                fuel_diff,
                fuel_over: fuel_norm > 0.0 && fuel_diff > fuel_norm * FUEL_OVERUSE_PCT / 100.0,
                cost: hours * m.hour_rate,
                work_days: days.len(),
                utilization: if available > 0.0 {
                    hours / available * 100.0
                } else {
                    0.0
                },
                idle_days: work_days_total.saturating_sub(days.len()),
                service_left,
                blocks,
            }
        })
        .collect()
}

// ================= IV. Ijro hujjatlari =================

/// Ish uchun talab qilinadigan bitta hujjat.
#[derive(Debug, Clone)]
pub struct RequiredDoc {
    pub task_id: i64,
    pub task_name: String,
    pub section: Section,
    pub kind: ExecDocKind,
    /// Ish tugallanganmi — tugallangan bo'lsa hujjat kechiktirilgan hisoblanadi.
    pub task_done: bool,
    /// Shu turdagi hujjat mavjudmi va u imzolanganmi.
    pub exists: bool,
    pub signed: bool,
}

/// TZ IV.1: «tizim har bosqichda qaysi ijro hujjati kerakligini bilishi kerak».
///
/// Talab ishning bo'limiga qarab aniqlanadi: yashirin ishlar dalolatnomasi
/// konstruksiya va muhandislik tarmoqlari uchun, sinov bayonnomasi bosim va
/// izolyatsiya sinaladigan tarmoqlar uchun, ijro sxemasi esa geometriyasi
/// nazorat qilinadigan ishlar uchun.
pub fn required_docs(tasks: &[Task], docs: &[ExecDoc], only_started: bool) -> Vec<RequiredDoc> {
    let mut out = Vec::new();
    for task in tasks {
        let done = task.progress >= 99.999 || task.fact_end.is_some();
        let started = task.progress > 0.0 || task.fact_start.is_some();
        if only_started && !started {
            continue;
        }

        // Bo'lim bo'yicha talab qilinadigan hujjat turlari.
        let kinds: &[ExecDocKind] = match task.section {
            // Konstruksiyalar: yashirin ishlar + ijro sxemasi.
            Section::Kj | Section::Km => &[ExecDocKind::Hidden, ExecDocKind::Scheme],
            // Muhandislik tarmoqlari: yashirin ishlar + sinov bayonnomasi.
            Section::Vk | Section::Ov => &[ExecDocKind::Hidden, ExecDocKind::Test],
            // Elektr va kuchsiz tok: yashirin ishlar + sinov.
            Section::Eom | Section::Ss => &[ExecDocKind::Hidden, ExecDocKind::Test],
            // Yong'in xavfsizligi: sinov + qabul dalolatnomasi.
            Section::Pb => &[ExecDocKind::Test, ExecDocKind::Acceptance],
            // Arxitektura: qabul dalolatnomasi.
            Section::Ar => &[ExecDocKind::Acceptance],
            Section::None => &[],
        };

        for kind in kinds {
            let found = docs
                .iter()
                .find(|d| d.task_id == Some(task.id) && d.kind == *kind);
            out.push(RequiredDoc {
                task_id: task.id,
                task_name: task.name.clone(),
                section: task.section,
                kind: *kind,
                task_done: done,
                exists: found.is_some(),
                signed: found
                    .map(|d| d.status == ExecDocStatus::Signed)
                    .unwrap_or(false),
            });
        }
    }
    out
}

// ================= I.3. PPR tekshiruvi =================

/// PPR tekshiruvi uchun kontekst: grafik, kartalar va mavjud resurs.
pub struct PprCtx<'a> {
    pub project_id: i64,
    pub tasks: &'a [Task],
    /// GPR bog'lanishlari — texnologik ketma-ketlikni shular belgilaydi.
    pub links: &'a [crate::model::Link],
    pub schedule: &'a crate::cpm::Schedule,
    pub docs: &'a [PprDoc],
    /// Grafikning nol kuni va bugungi sana — risk ufqini hisoblash uchun.
    pub origin: NaiveDate,
    pub today: NaiveDate,
    /// Obyektda mavjud ishchi va texnika soni. Nol bo'lsa, yetarlilik
    /// tekshirilmaydi — chegara ma'lum bo'lmasa, xulosa chiqarmaymiz.
    pub avail_workers: i64,
    pub avail_machines: i64,
    pub norms: &'a HashMap<String, Norm>,
}

/// Kartalarni ishlar bo'yicha guruhlaydi.
fn cards_by_task(docs: &[PprDoc]) -> HashMap<i64, Vec<&PprDoc>> {
    docs.iter()
        .filter_map(|d| d.task_id.map(|t| (t, d)))
        .fold(HashMap::new(), |mut m, (t, d)| {
            m.entry(t).or_default().push(d);
            m
        })
}

/// Kunlik resurs talabi: grafikning har bir kuni uchun o'sha kuni ketayotgan
/// ishlarning kartalaridagi ishchi (yoki texnika) sonlari yig'indisi.
///
/// Bir xil hisob ham tekshiruvda, ham ekrandagi gistogrammada ishlatiladi —
/// shunda diagramma va xato matni hech qachon bir-biriga zid chiqmaydi.
pub fn resource_demand(
    tasks: &[Task],
    schedule: &crate::cpm::Schedule,
    docs: &[PprDoc],
    machines: bool,
) -> Vec<i64> {
    let days = schedule.project_days.max(1) as usize;
    let mut out = vec![0i64; days];
    let by_task = cards_by_task(docs);
    for task in tasks {
        let Some(c) = schedule.get(task.id) else {
            continue;
        };
        let Some(cards) = by_task.get(&task.id) else {
            continue;
        };
        let need: i64 = cards
            .iter()
            .map(|d| if machines { d.machines } else { d.workers })
            .sum();
        if need <= 0 {
            continue;
        }
        for day in c.es.max(0)..=c.ef.min(days as i64 - 1) {
            out[day as usize] += need;
        }
    }
    out
}

/// TZ I.3: PPR loyihaga mos keladimi, texnika va odam yetadimi,
/// ketma-ketlik va risklar.
pub fn check_ppr(ctx: &PprCtx) -> Vec<Issue> {
    let mut b = Builder::new(ctx.norms, ctx.project_id, IssueModule::Ppr);
    let mut out = Vec::new();

    let by_task = cards_by_task(ctx.docs);
    let task_ids: std::collections::HashSet<i64> = ctx.tasks.iter().map(|t| t.id).collect();
    let today_off = (ctx.today - ctx.origin).num_days();
    // Risk ufqi: shu muddat ichida boshlanadigan ish uchun karta tayyor bo'lishi kerak.
    const RISK_HORIZON: i64 = 30;

    // --- Karta yo'q: kritik yo'ldagi yoki boshlangan ish uchun ---
    for task in ctx.tasks {
        let has_card = by_task
            .get(&task.id)
            .map(|v| {
                v.iter()
                    .any(|d| d.kind == PprKind::TechCard || d.kind == PprKind::Ppr)
            })
            .unwrap_or(false);
        if has_card {
            continue;
        }
        let critical = ctx.schedule.is_critical(task.id);
        // Boshlangan ishda karta bo'lishi shart. Hali boshlanmagan kritik ish
        // esa PPR_RISK qoidasida — u yaqinlashganda ogohlantiriladi.
        let started = task.progress > 0.0 || task.fact_start.is_some();
        if !started {
            continue;
        }
        out.push(b.make(
            "PPR_MISSING",
            "PP",
            task.section,
            if critical {
                Severity::Critical
            } else {
                Severity::Major
            },
            crate::i18n::t("chk_ppr_missing_title").to_string(),
            format!(
                "«{}» — {}",
                task.name,
                if critical {
                    crate::i18n::t("chk_ppr_missing_critical")
                } else {
                    crate::i18n::t("chk_ppr_missing_started")
                }
            ),
            task.wbs.clone(),
            task.name.clone(),
            String::new(),
            crate::i18n::t("chk_ppr_missing_fix").to_string(),
            task.responsible.clone(),
        ));
    }

    // --- Ish boshlangan, karta esa tasdiqlanmagan ---
    for task in ctx.tasks {
        if task.progress <= 0.0 && task.fact_start.is_none() {
            continue;
        }
        let Some(cards) = by_task.get(&task.id) else {
            continue;
        };
        let unapproved: Vec<&&PprDoc> = cards.iter().filter(|d| !d.approved).collect();
        if unapproved.is_empty() {
            continue;
        }
        out.push(b.make(
            "PPR_NOT_APPROVED",
            "PP",
            task.section,
            Severity::Critical,
            crate::i18n::t("chk_ppr_unapproved_title").to_string(),
            format!(
                "«{}»: {} — {}",
                task.name,
                unapproved
                    .iter()
                    .map(|d| d.kind.label())
                    .collect::<Vec<_>>()
                    .join(", "),
                crate::i18n::t("chk_ppr_unapproved_desc")
            ),
            task.wbs.clone(),
            unapproved[0].number.clone(),
            String::new(),
            crate::i18n::t("chk_ppr_unapproved_fix").to_string(),
            task.responsible.clone(),
        ));
    }

    // --- Karta ishga bog'lanmagan yoki ish o'chirilgan ---
    for d in ctx.docs {
        let dangling = match d.task_id {
            None => true,
            Some(t) => !task_ids.contains(&t),
        };
        if !dangling {
            continue;
        }
        out.push(b.make(
            "PPR_ORPHAN",
            "PP",
            d.section,
            Severity::Warning,
            crate::i18n::t("chk_ppr_orphan_title").to_string(),
            format!(
                "{} «{}» — {}",
                d.kind.label(),
                if d.number.is_empty() {
                    &d.name
                } else {
                    &d.number
                },
                crate::i18n::t("chk_ppr_orphan_desc")
            ),
            String::new(),
            d.number.clone(),
            String::new(),
            crate::i18n::t("chk_ppr_orphan_fix").to_string(),
            String::new(),
        ));
    }

    // --- TZ I.3: texnologik ketma-ketlik ---
    // Ish boshlangan, lekin uni ta'minlaydigan oldingi ish tugallanmagan.
    // «Pardozlash bor, karkas tugamagan» — TZ dagi tipik misol.
    let by_id: HashMap<i64, &Task> = ctx.tasks.iter().map(|t| (t.id, t)).collect();
    for task in ctx.tasks {
        let started = task.progress > 0.0 || task.fact_start.is_some();
        if !started {
            continue;
        }
        for link in ctx.links {
            // Faqat «tugagach boshlanadi» bog'lanishi qat'iy ketma-ketlik beradi.
            if link.succ != task.id || link.kind != crate::model::LinkType::Fs {
                continue;
            }
            let Some(pred) = by_id.get(&link.pred) else {
                continue;
            };
            let pred_done = pred.progress >= 99.999 || pred.fact_end.is_some();
            if pred_done {
                continue;
            }
            out.push(b.make(
                "PPR_SEQ",
                "PP",
                task.section,
                // Oldingi ish yarmigacha ham yetmagan bo'lsa — kritik.
                if pred.progress < 50.0 {
                    Severity::Critical
                } else {
                    Severity::Major
                },
                crate::i18n::t("chk_ppr_seq_title").to_string(),
                format!(
                    "«{}» ({:.0} %) {} «{}» ({:.0} %)",
                    task.name,
                    task.progress,
                    crate::i18n::t("chk_ppr_seq_desc"),
                    pred.name,
                    pred.progress
                ),
                task.wbs.clone(),
                task.name.clone(),
                String::new(),
                crate::i18n::t("chk_ppr_seq_fix").to_string(),
                task.responsible.clone(),
            ));
        }
    }

    // --- TZ I.3: risklar ---
    // Kritik yo'ldagi ish yaqin kunlarda boshlanadi, tasdiqlangan kartasi yo'q.
    for task in ctx.tasks {
        if task.progress > 0.0 || task.fact_start.is_some() {
            continue;
        }
        if !ctx.schedule.is_critical(task.id) {
            continue;
        }
        let Some(c) = ctx.schedule.get(task.id) else {
            continue;
        };
        let days_left = c.es - today_off;
        if !(0..=RISK_HORIZON).contains(&days_left) {
            continue;
        }
        let ready = by_task
            .get(&task.id)
            .map(|v| {
                v.iter()
                    .any(|d| d.approved && matches!(d.kind, PprKind::TechCard | PprKind::Ppr))
            })
            .unwrap_or(false);
        if ready {
            continue;
        }
        out.push(b.make(
            "PPR_RISK",
            "PP",
            task.section,
            Severity::Major,
            crate::i18n::t("chk_ppr_risk_title").to_string(),
            format!(
                "«{}» {} {} {}. {}",
                task.name,
                crate::i18n::t("chk_ppr_risk_starts"),
                days_left,
                crate::i18n::t("days_short"),
                crate::i18n::t("chk_ppr_risk_desc")
            ),
            format!(
                "{}: {}",
                crate::i18n::t("col_start_short"),
                (ctx.origin + chrono::Duration::days(c.es)).format("%d.%m.%Y")
            ),
            task.name.clone(),
            String::new(),
            crate::i18n::t("chk_ppr_risk_fix").to_string(),
            task.responsible.clone(),
        ));
    }

    // --- Resurs yetarliligi: bir kunda parallel ketayotgan ishlar talabi ---
    for (avail, pick, rule, title_key, unit_key) in [
        (
            ctx.avail_workers,
            0usize,
            "PPR_RESOURCE",
            "chk_ppr_workers_title",
            "chk_ppr_workers",
        ),
        (
            ctx.avail_machines,
            1usize,
            "PPR_RESOURCE",
            "chk_ppr_machines_title",
            "chk_ppr_machines",
        ),
    ] {
        if avail <= 0 {
            continue; // mavjud resurs kiritilmagan — xulosa chiqarmaymiz
        }
        let demand = resource_demand(ctx.tasks, ctx.schedule, ctx.docs, pick == 1);
        let (peak_day, peak) = demand
            .iter()
            .enumerate()
            .max_by_key(|(_, v)| **v)
            .map(|(i, v)| (i as i64, *v))
            .unwrap_or((0, 0));
        // Cho'qqi kunida qaysi ishlar ketayotganini ko'rsatamiz.
        let peak_tasks: Vec<String> = ctx
            .tasks
            .iter()
            .filter(|task| {
                ctx.schedule
                    .get(task.id)
                    .map(|c| peak_day >= c.es && peak_day <= c.ef)
                    .unwrap_or(false)
                    && by_task
                        .get(&task.id)
                        .map(|v| {
                            v.iter().any(|d| {
                                if pick == 1 {
                                    d.machines > 0
                                } else {
                                    d.workers > 0
                                }
                            })
                        })
                        .unwrap_or(false)
            })
            .map(|task| task.name.clone())
            .collect();
        // Ortiqcha yuklangan kunlar soni — xato matnida ko'rsatiladi.
        let over_days = demand.iter().filter(|v| **v > avail).count();
        if peak > avail {
            out.push(b.make(
                rule,
                "PP",
                Section::None,
                Severity::Major,
                crate::i18n::t(title_key).to_string(),
                format!(
                    "{}: {} {}; {}: {} {}. {}: {} {} · {} {}\n{}",
                    crate::i18n::t("chk_ppr_need"),
                    peak,
                    crate::i18n::t(unit_key),
                    crate::i18n::t("chk_ppr_have"),
                    avail,
                    crate::i18n::t(unit_key),
                    crate::i18n::t("chk_ppr_peak_day"),
                    peak_day + 1,
                    crate::i18n::t("days_short"),
                    over_days,
                    crate::i18n::t("chk_ppr_over_days"),
                    peak_tasks.join("; ")
                ),
                format!("{} {}", crate::i18n::t("chk_ppr_peak_day"), peak_day + 1),
                String::new(),
                String::new(),
                crate::i18n::t("chk_ppr_resource_fix").to_string(),
                String::new(),
            ));
        }
    }

    out.sort_by_key(|i| i.severity.rank());
    out
}

fn fmt(v: f64) -> String {
    if (v - v.round()).abs() < 0.001 {
        crate::ui::money(v)
    } else {
        format!("{v:.3}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        Element, ElementKind, ElementLink, EstimateItem, PprDoc, PprKind, Relation, Severity,
    };
    use crate::model::{Section, Task};

    fn item(pos: i64, name: &str, unit: &str, qty: f64, price: f64, cost: f64) -> EstimateItem {
        EstimateItem {
            id: pos,
            estimate_id: 1,
            pos,
            section: Section::Ar,
            code: String::new(),
            name: name.into(),
            unit: unit.into(),
            qty,
            price,
            cost,
            note: String::new(),
        }
    }

    fn task(name: &str, volume: f64, unit: &str) -> Task {
        Task {
            id: 1,
            project_id: 1,
            wbs: "1".into(),
            name: name.into(),
            section: Section::Ar,
            responsible: String::new(),
            duration: 5,
            plan_start: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            fact_start: None,
            fact_end: None,
            progress: 0.0,
            pinned: false,
            volume,
            unit: unit.into(),
        }
    }

    fn el(id: i64, section: Section, kind: ElementKind, mark: &str, sheet: &str) -> Element {
        Element {
            id,
            project_id: 1,
            section,
            kind,
            mark: mark.into(),
            room: String::new(),
            axis: String::new(),
            level: String::new(),
            size: 100.0,
            unit: "mm".into(),
            value: 0.0,
            value_name: String::new(),
            sheet: sheet.into(),
            note: String::new(),
        }
    }

    struct Fixture {
        tasks: Vec<Task>,
        elements: Vec<Element>,
        links: Vec<ElementLink>,
        items: Vec<EstimateItem>,
        declared: f64,
        norms: HashMap<String, Norm>,
    }

    impl Fixture {
        fn ctx(&self) -> Ctx<'_> {
            Ctx {
                project_id: 1,
                tasks: &self.tasks,
                elements: &self.elements,
                links: &self.links,
                items: &self.items,
                declared_total: self.declared,
                norms: &self.norms,
            }
        }
    }

    fn empty() -> Fixture {
        Fixture {
            tasks: Vec::new(),
            elements: Vec::new(),
            links: Vec::new(),
            items: Vec::new(),
            declared: 0.0,
            norms: HashMap::new(),
        }
    }

    /// TZ III.4: miqdor x narx summaga teng bo'lmasa — kritik xato.
    #[test]
    fn arithmetic_error_is_critical() {
        let mut f = empty();
        f.items = vec![item(1, "Beton", "m3", 10.0, 100.0, 1500.0)];
        let out = check_estimate(&f.ctx());
        let arith: Vec<_> = out
            .iter()
            .filter(|i| i.title == crate::i18n::t("chk_arith_title"))
            .collect();
        assert_eq!(arith.len(), 1);
        assert_eq!(arith[0].severity, Severity::Critical);
    }

    /// Arifmetika to'g'ri bo'lsa, xato chiqmaydi.
    #[test]
    fn correct_arithmetic_passes() {
        let mut f = empty();
        f.items = vec![item(1, "Beton", "m3", 10.0, 100.0, 1000.0)];
        f.declared = 1000.0;
        let out = check_estimate(&f.ctx());
        assert!(out
            .iter()
            .all(|i| i.title != crate::i18n::t("chk_arith_title")));
    }

    /// TZ III.6: smetadagi hajm loyihadagidan sezilarli farq qilsa — xato.
    #[test]
    fn volume_deviation_is_reported() {
        let mut f = empty();
        f.tasks = vec![task("Devor", 100.0, "m2")];
        f.items = vec![item(1, "Devor", "m2", 130.0, 10.0, 1300.0)];
        f.declared = 1300.0;
        let out = check_estimate(&f.ctx());
        assert!(out
            .iter()
            .any(|i| i.title == crate::i18n::t("chk_vol_over")));
    }

    /// Birliklar mos kelmasa, hajm solishtirilmaydi — aks holda ikkita xato chiqardi.
    #[test]
    fn unit_mismatch_skips_volume_check() {
        let mut f = empty();
        f.tasks = vec![task("Devor", 100.0, "m2")];
        f.items = vec![item(1, "Devor", "m3", 130.0, 10.0, 1300.0)];
        f.declared = 1300.0;
        let out = check_estimate(&f.ctx());
        assert!(out
            .iter()
            .any(|i| i.title == crate::i18n::t("chk_unit_title")));
        assert!(out
            .iter()
            .all(|i| i.title != crate::i18n::t("chk_vol_over")));
    }

    /// Bir xil natija — bir xil kodlar. `replace_auto_issues` shunga tayanadi.
    #[test]
    fn issue_codes_are_stable_between_runs() {
        let mut f = empty();
        f.items = vec![
            item(1, "Styashka", "m2", 100.0, 10.0, 1000.0),
            item(2, "Styashka", "m2", 100.0, 10.0, 1000.0),
            item(3, "Gruntovka", "m2", 50.0, 5.0, 250.0),
            item(4, "Gruntovka", "m2", 70.0, 5.0, 350.0),
            item(5, "Shpaklyovka", "m2", 30.0, 3.0, 90.0),
        ];
        f.declared = 2690.0;
        let first: Vec<(String, String)> = check_estimate(&f.ctx())
            .into_iter()
            .map(|i| (i.code, i.title.to_string()))
            .collect();
        for _ in 0..8 {
            let again: Vec<(String, String)> = check_estimate(&f.ctx())
                .into_iter()
                .map(|i| (i.code, i.title.to_string()))
                .collect();
            assert_eq!(first, again);
        }
    }

    /// TZ II.11: deraza bor, konstruksiyada teshik yo'q — kritik nomuvofiqlik.
    #[test]
    fn window_without_opening_is_critical() {
        let mut f = empty();
        f.elements = vec![
            el(1, Section::Ar, ElementKind::Window, "OK-1", "AR-06"),
            el(2, Section::Ar, ElementKind::Window, "OK-2", "AR-06"),
            el(3, Section::Kj, ElementKind::Opening, "PR-1", "KJ-12"),
        ];
        f.links = vec![ElementLink {
            id: 1,
            from_el: 1,
            to_el: 3,
            relation: Relation::Contains,
        }];
        let out = check_project(&f.ctx());
        let arkj: Vec<_> = out
            .iter()
            .filter(|i| i.title == crate::i18n::t("chk_arkj_title"))
            .collect();
        assert_eq!(arkj.len(), 1, "teshigi yo'q faqat OK-2");
        assert_eq!(arkj[0].element, "OK-2");
        assert_eq!(arkj[0].severity, Severity::Critical);
    }

    /// TZ II.11: qurilma bor, elektr ta'minoti ko'zda tutilmagan.
    #[test]
    fn device_without_power_is_reported() {
        let mut f = empty();
        f.elements = vec![
            el(1, Section::Ov, ElementKind::Device, "VN-1", "OV-02"),
            el(2, Section::Vk, ElementKind::Device, "N-1", "VK-01"),
            el(3, Section::Eom, ElementKind::Cable, "W-1", "EOM-01"),
            el(4, Section::Pb, ElementKind::Device, "IP-1", "PB-03"),
        ];
        // Faqat nasos quvvat manbaiga ulangan.
        f.links = vec![ElementLink {
            id: 1,
            from_el: 2,
            to_el: 3,
            relation: Relation::PoweredBy,
        }];
        let out = check_project(&f.ctx());
        let power: Vec<_> = out
            .iter()
            .filter(|i| i.title == crate::i18n::t("chk_power_title"))
            .collect();
        assert_eq!(power.len(), 2, "ventilyator va izveshchatel ta'minotsiz");
        assert!(power
            .iter()
            .any(|i| i.element == "VN-1" && i.severity == Severity::Major));
        // Yong'in xavfsizligi uchun bu kritik.
        assert!(power
            .iter()
            .any(|i| i.element == "IP-1" && i.severity == Severity::Critical));
        assert!(power.iter().all(|i| i.element != "N-1"));
    }

    /// TZ II.11: KM konstruksiyasi uchun KJ dagi tayanch aniqlanmagan.
    #[test]
    fn steel_member_without_support_is_reported() {
        let mut f = empty();
        f.elements = vec![
            el(1, Section::Km, ElementKind::Beam, "MB-1", "KM-02"),
            el(2, Section::Km, ElementKind::Column, "MK-1", "KM-02"),
            el(3, Section::Kj, ElementKind::Column, "K-1", "KJ-05"),
        ];
        f.links = vec![ElementLink {
            id: 1,
            from_el: 2,
            to_el: 3,
            relation: Relation::SupportedBy,
        }];
        let out = check_project(&f.ctx());
        let sup: Vec<_> = out
            .iter()
            .filter(|i| i.title == crate::i18n::t("chk_kmkj_title"))
            .collect();
        assert_eq!(sup.len(), 1);
        assert_eq!(sup[0].element, "MB-1");
    }

    /// TZ III.31: pul xulosasidagi har bir son alohida to'g'ri hisoblanadi.
    #[test]
    fn cost_summary_counts_each_kind_separately() {
        let mut f = empty();
        f.tasks = vec![task("Devor", 100.0, "m2")];
        f.items = vec![
            // Loyihada 100 m2, smetada 130 — 30 x 10 = 300 oshiq.
            item(1, "Devor", "m2", 130.0, 10.0, 1300.0),
            // Arifmetika: 10 x 5 = 50, hujjatda 80 — farq +30.
            item(2, "Grunt", "m2", 10.0, 5.0, 80.0),
            // Dublikat: ikkinchisi 200 so'm.
            item(3, "Styashka", "m2", 20.0, 10.0, 200.0),
            item(4, "Styashka", "m2", 20.0, 10.0, 200.0),
        ];
        f.declared = 1600.0;

        let s = cost_summary(&f.ctx());
        assert_eq!(s.total, 1780.0);
        // Yakun bilan farq: 1780 - 1600.
        assert_eq!(s.declared_diff, 180.0);
        // Faqat 2-pozitsiyada arifmetik farq bor.
        assert_eq!(s.arithmetic_diff, 30.0);
        // Hajm oshig'i: (130 - 100) x 10.
        assert_eq!(s.volume_excess, 300.0);
        // Dublikat: bitta ortiqcha pozitsiya, 200 so'm.
        assert_eq!(s.duplicate_count, 1);
        assert_eq!(s.duplicate_cost, 200.0);
    }

    /// Bir xil ish turli narxda bo'lsa, eng pastiga keltirish tejamini beradi.
    #[test]
    fn cost_summary_price_saving() {
        let mut f = empty();
        let mut a = item(1, "Kabel", "m", 100.0, 30.0, 3000.0);
        a.code = "E21-1".into();
        let mut b = item(2, "Kabel 2-bosqich", "m", 100.0, 50.0, 5000.0);
        b.code = "E21-1".into();
        f.items = vec![a, b];

        let s = cost_summary(&f.ctx());
        // Ikkinchisini 30 ga tushirsa: (50 - 30) x 100 = 2000.
        assert_eq!(s.price_saving, 2000.0);
    }

    /// Toza smetada tekshirishga arziydigan summa bo'lmaydi.
    #[test]
    fn clean_estimate_has_no_cost_flags() {
        let mut f = empty();
        f.tasks = vec![task("Devor", 100.0, "m2")];
        f.items = vec![item(1, "Devor", "m2", 100.0, 10.0, 1000.0)];
        f.declared = 1000.0;

        let s = cost_summary(&f.ctx());
        assert_eq!(s.declared_diff, 0.0);
        assert_eq!(s.arithmetic_diff, 0.0);
        assert_eq!(s.volume_excess, 0.0);
        assert_eq!(s.duplicate_count, 0);
        assert_eq!(s.price_saving, 0.0);
        assert_eq!(s.mismatch_count, 0);
    }

    // ---------- I.3. PPR ----------

    fn gtask(id: i64, name: &str, dur: i64) -> Task {
        let mut t = task(name, 0.0, "");
        t.id = id;
        t.duration = dur;
        t
    }

    fn card(id: i64, task_id: Option<i64>, workers: i64, approved: bool) -> PprDoc {
        PprDoc {
            id,
            project_id: 1,
            kind: PprKind::TechCard,
            number: format!("TK-{id:02}"),
            name: "karta".into(),
            section: Section::Kj,
            task_id,
            workers,
            machines: 0,
            path: String::new(),
            approved,
            author: String::new(),
            approved_at: None,
            note: String::new(),
        }
    }

    /// Tekshiruvni ishga tushiradi. `today_off` — grafikning nechanchi kuni
    /// «bugun» hisoblanishi; risk qoidasi shunga qarab ishlaydi.
    fn ppr_out_at(
        tasks: &[Task],
        links: &[crate::model::Link],
        docs: &[PprDoc],
        workers: i64,
        today_off: i64,
    ) -> Vec<Issue> {
        let origin = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let sched = crate::cpm::compute(tasks, links, origin);
        let norms = HashMap::new();
        check_ppr(&PprCtx {
            project_id: 1,
            tasks,
            links,
            schedule: &sched,
            docs,
            origin,
            today: origin + chrono::Duration::days(today_off),
            avail_workers: workers,
            avail_machines: 0,
            norms: &norms,
        })
    }

    /// Uzoq kelajakdagi «bugun»: risk qoidasi ishlamaydi, qolganlari tekshiriladi.
    fn ppr_out(
        tasks: &[Task],
        links: &[crate::model::Link],
        docs: &[PprDoc],
        workers: i64,
    ) -> Vec<Issue> {
        ppr_out_at(tasks, links, docs, workers, 500)
    }

    /// TZ I.3: boshlangan ish uchun texnologik karta bo'lishi shart.
    #[test]
    fn started_task_without_card_is_reported() {
        let mut t1 = gtask(1, "Poydevor", 10);
        t1.progress = 100.0;
        let mut t2 = gtask(2, "Karkas", 20);
        t2.progress = 30.0;
        let tasks = vec![t1, t2];
        // 1-ish uchun karta bor, 2-ish uchun yo'q.
        let docs = vec![card(1, Some(1), 0, true)];
        let out = ppr_out(&tasks, &[], &docs, 0);
        let missing: Vec<_> = out
            .iter()
            .filter(|i| i.title == crate::i18n::t("chk_ppr_missing_title"))
            .collect();
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].element, "Karkas");
    }

    /// TZ I.3 «риски»: kritik ish yaqinda boshlanadi, tasdiqlangan kartasi yo'q.
    #[test]
    fn upcoming_critical_task_without_card_is_a_risk() {
        // 1-ish 0..9 kun, 2-ish 10..29 — ikkalasi ham kritik yo'lda.
        let tasks = vec![gtask(1, "Poydevor", 10), gtask(2, "Karkas", 20)];
        let links = vec![crate::model::Link {
            id: 1,
            pred: 1,
            succ: 2,
            kind: crate::model::LinkType::Fs,
            lag: 0,
        }];
        let docs = vec![card(1, Some(1), 0, true)];

        // «Bugun» 5-kun: 2-ish 5 kundan keyin boshlanadi — risk ufqida.
        let out = ppr_out_at(&tasks, &links, &docs, 0, 5);
        let risks: Vec<_> = out
            .iter()
            .filter(|i| i.title == crate::i18n::t("chk_ppr_risk_title"))
            .collect();
        assert_eq!(risks.len(), 1);
        assert_eq!(risks[0].element, "Karkas");

        // Tasdiqlangan karta qo'shilsa, risk yo'qoladi.
        let docs = vec![card(1, Some(1), 0, true), card(2, Some(2), 0, true)];
        let out = ppr_out_at(&tasks, &links, &docs, 0, 5);
        assert!(out
            .iter()
            .all(|i| i.title != crate::i18n::t("chk_ppr_risk_title")));

        // Ish hali uzoqda bo'lsa ham ogohlantirilmaydi.
        let docs = vec![card(1, Some(1), 0, true)];
        let mut far = tasks.clone();
        far[0].duration = 400;
        let out = ppr_out_at(&far, &links, &docs, 0, 0);
        assert!(out
            .iter()
            .all(|i| i.title != crate::i18n::t("chk_ppr_risk_title")));
    }

    /// TZ I.3 «последовательность работ»: keyingi ish oldingisi tugamasdan boshlangan.
    #[test]
    fn sequence_violation_is_reported() {
        let mut t1 = gtask(1, "Karkas", 20);
        t1.progress = 40.0;
        let mut t2 = gtask(2, "Pardozlash", 30);
        t2.progress = 10.0;
        let tasks = vec![t1, t2];
        let links = vec![crate::model::Link {
            id: 1,
            pred: 1,
            succ: 2,
            kind: crate::model::LinkType::Fs,
            lag: 0,
        }];
        let docs = vec![card(1, Some(1), 0, true), card(2, Some(2), 0, true)];
        let out = ppr_out(&tasks, &links, &docs, 0);
        let seq: Vec<_> = out
            .iter()
            .filter(|i| i.title == crate::i18n::t("chk_ppr_seq_title"))
            .collect();
        assert_eq!(seq.len(), 1);
        assert_eq!(seq[0].element, "Pardozlash");
        // Oldingi ish yarmigacha yetmagan — kritik.
        assert_eq!(seq[0].severity, Severity::Critical);

        // Oldingi ish tugallansa, xato yo'qoladi.
        let mut ok = tasks.clone();
        ok[0].progress = 100.0;
        let out = ppr_out(&ok, &links, &docs, 0);
        assert!(out
            .iter()
            .all(|i| i.title != crate::i18n::t("chk_ppr_seq_title")));
    }

    /// Kunlik resurs talabi grafik bo'ylab to'g'ri yig'iladi.
    #[test]
    fn resource_demand_sums_parallel_tasks() {
        let tasks = vec![gtask(1, "A", 10), gtask(2, "B", 10)];
        // SS bog'lanish: ikkalasi ham 0..9 kunlarda ketadi.
        let links = vec![crate::model::Link {
            id: 1,
            pred: 1,
            succ: 2,
            kind: crate::model::LinkType::Ss,
            lag: 0,
        }];
        let docs = vec![card(1, Some(1), 12, true), card(2, Some(2), 8, true)];
        let origin = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let sched = crate::cpm::compute(&tasks, &links, origin);
        let demand = resource_demand(&tasks, &sched, &docs, false);
        assert_eq!(demand.len(), 10);
        assert!(demand.iter().all(|v| *v == 20));
        // Texnika ko'rsatilmagan — nol.
        assert!(resource_demand(&tasks, &sched, &docs, true)
            .iter()
            .all(|v| *v == 0));
    }

    /// Ish boshlangan, karta esa tasdiqlanmagan — kritik.
    #[test]
    fn started_work_with_unapproved_card_is_critical() {
        let mut t1 = gtask(1, "Karkas", 20);
        t1.progress = 40.0;
        let tasks = vec![t1];
        let docs = vec![card(1, Some(1), 0, false)];
        let out = ppr_out(&tasks, &[], &docs, 0);
        let bad: Vec<_> = out
            .iter()
            .filter(|i| i.title == crate::i18n::t("chk_ppr_unapproved_title"))
            .collect();
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].severity, Severity::Critical);
    }

    /// Mavjud bo'lmagan ishga bog'langan karta belgilanadi.
    #[test]
    fn dangling_card_is_reported() {
        let tasks = vec![gtask(1, "Karkas", 20)];
        // Ish boshlanmagan — PPR_MISSING chiqmaydi, faqat bog'lanish xatolari.
        let docs = vec![
            card(1, Some(1), 0, true),
            card(2, Some(99), 0, true),
            card(3, None, 0, true),
        ];
        let out = ppr_out(&tasks, &[], &docs, 0);
        assert_eq!(
            out.iter()
                .filter(|i| i.title == crate::i18n::t("chk_ppr_orphan_title"))
                .count(),
            2
        );
    }

    /// Parallel ketayotgan ishlar talabi mavjud resursdan oshsa — ogohlantirish.
    #[test]
    fn resource_peak_over_capacity_is_reported() {
        let tasks = vec![gtask(1, "Karkas", 20), gtask(2, "Devor", 20)];
        // SS bog'lanish: ikkala ish bir vaqtda ketadi.
        let link = crate::model::Link {
            id: 1,
            pred: 1,
            succ: 2,
            kind: crate::model::LinkType::Ss,
            lag: 0,
        };
        let docs = vec![card(1, Some(1), 20, true), card(2, Some(2), 20, true)];

        // 30 ta ishchi bor — 40 talab qilinadi.
        let out = ppr_out(&tasks, std::slice::from_ref(&link), &docs, 30);
        assert_eq!(
            out.iter()
                .filter(|i| i.title == crate::i18n::t("chk_ppr_workers_title"))
                .count(),
            1
        );

        // 50 ta ishchi bo'lsa, xato yo'q.
        let out = ppr_out(&tasks, std::slice::from_ref(&link), &docs, 50);
        assert!(out
            .iter()
            .all(|i| i.title != crate::i18n::t("chk_ppr_workers_title")));
    }

    /// Mavjud resurs kiritilmagan bo'lsa, yetarlilik haqida xulosa chiqarilmaydi.
    #[test]
    fn unknown_capacity_produces_no_verdict() {
        let tasks = vec![gtask(1, "Karkas", 20)];
        let docs = vec![card(1, Some(1), 500, true)];
        let out = ppr_out(&tasks, &[], &docs, 0);
        assert!(out
            .iter()
            .all(|i| i.title != crate::i18n::t("chk_ppr_workers_title")));
    }

    /// TZ II.17: reyestr bo'sh bo'lsa, me'yoriy asos o'ylab topilmaydi.
    #[test]
    fn missing_norm_is_stated_not_invented() {
        let mut f = empty();
        f.items = vec![item(1, "Beton", "m3", 10.0, 100.0, 1500.0)];
        let out = check_estimate(&f.ctx());
        let i = &out[0];
        assert!(i.norm_doc.is_empty());
        assert!(i.norm_clause.is_empty());
        assert_eq!(i.norm_text, crate::i18n::t("norm_missing"));
    }

    /// Reyestr to'ldirilgan bo'lsa, asos xatoga o'tadi.
    #[test]
    fn filled_norm_reaches_the_issue() {
        let mut f = empty();
        f.items = vec![item(1, "Beton", "m3", 10.0, 100.0, 1500.0)];
        f.norms.insert(
            "EST_ARITH".into(),
            Norm {
                doc: "ShNQ 4.01.16".into(),
                edition: "2021".into(),
                clause: "5.2".into(),
                text: "Smeta hisobi arifmetik jihatdan to'g'ri bo'lishi kerak.".into(),
                param: 0.5,
                param_set: true,
                source: "lex.uz".into(),
            },
        );
        let out = check_estimate(&f.ctx());
        assert_eq!(out[0].norm_doc, "ShNQ 4.01.16 2021");
        assert_eq!(out[0].norm_clause, "5.2");
    }

    /// TZ II.18: element o'zgarsa, bog'liq bo'limlar ro'yxati qaytadi.
    #[test]
    fn impact_walks_the_graph() {
        let elements = vec![
            el(1, Section::Ar, ElementKind::Room, "101", "AR-04"),
            el(2, Section::Vk, ElementKind::Pipe, "K1-1", "VK-03"),
            el(3, Section::Eom, ElementKind::Cable, "W-1", "EOM-01"),
        ];
        let links = vec![
            ElementLink {
                id: 1,
                from_el: 2,
                to_el: 1,
                relation: Relation::Serves,
            },
            ElementLink {
                id: 2,
                from_el: 3,
                to_el: 1,
                relation: Relation::Serves,
            },
        ];
        let out = impact(&elements, &links, 1);
        assert_eq!(out.len(), 2, "VK va EOM bo'limlari ta'sirlanadi");
        assert!(out.iter().any(|(s, n)| *s == Section::Vk && *n == 1));
        assert!(out.iter().any(|(s, n)| *s == Section::Eom && *n == 1));
    }
}
