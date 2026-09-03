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
use chrono::{Datelike, NaiveDate};
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
    ("PRJ_PB_ROOM", "rule_prj_pb_room"),
    ("PRJ_PB_EXIT", "rule_prj_pb_exit"),
    ("PRJ_PB_WATER", "rule_prj_pb_water"),
    ("PRJ_SS_CABLE", "rule_prj_ss_cable"),
    ("PRJ_SS_PLACE", "rule_prj_ss_place"),
    ("PRJ_SPEC_QTY", "rule_prj_spec_qty"),
    ("PRJ_SPEC_UNIT", "rule_prj_spec_unit"),
    ("PRJ_BUILD_SIZE", "rule_prj_build_size"),
    ("PRJ_BUILD_LEVEL", "rule_prj_build_level"),
    ("PRJ_AR_DOOR", "rule_prj_ar_door"),
    ("PRJ_AR_LIGHT", "rule_prj_ar_light"),
    ("PRJ_KJ_SECTION", "rule_prj_kj_section"),
    ("PRJ_KJ_CLASS", "rule_prj_kj_class"),
    ("PRJ_KM_STEEL", "rule_prj_km_steel"),
    ("PRJ_VK_VELOCITY", "rule_prj_vk_velocity"),
    ("PRJ_OV_VELOCITY", "rule_prj_ov_velocity"),
    ("PRJ_EOM_SECTION", "rule_prj_eom_section"),
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

    // --- II.10 PB: yong'in xavfsizligi ---
    // Uch savol: xonani kim qo'riqlaydi, undan qanday chiqiladi va
    // o'chirish tizimiga suv qayerdan keladi.
    let (pb_area, pb_confirmed) = ctx.threshold("PRJ_PB_AREA", 20.0);

    // Eshiklar umuman modellanmagan loyihada «chiqish yo'q» deyish
    // noto'g'ri bo'ladi: bu chizmaning kamchiligi emas, modelning
    // to'liqsizligi. Shuning uchun qoida faqat eshik bog'lanishi
    // ishlatilgan loyihalarda yoqiladi.
    let doors_modelled = ctx.links.iter().any(|l| {
        l.relation == Relation::Contains
            && by_id
                .get(&l.to_el)
                .map(|e| e.kind == ElementKind::Door)
                .unwrap_or(false)
    });

    for room in ctx.elements.iter().filter(|e| e.kind == ElementKind::Room) {
        // Maydon `size` da ham, `value` da ham yozilishi mumkin — kartochka
        // qaysi maydonni ishlatgani import manbasiga bog'liq.
        let area = if room.size > 0.0 {
            room.size
        } else {
            room.value
        };
        // Kichik xonalar tekshiruvdan chetda: chegara reyestrda sozlanadi.
        if area <= pb_area {
            continue;
        }
        let room_name = if room.mark.is_empty() {
            room.room.clone()
        } else {
            room.mark.clone()
        };

        let served_by = |section: Section| -> bool {
            rel_in
                .get(&room.id)
                .map(|v| {
                    v.iter()
                        .any(|(l, e)| l.relation == Relation::Serves && e.section == section)
                })
                .unwrap_or(false)
        };

        if !served_by(Section::Pb) {
            let mut desc = format!(
                "{} «{}», {:.1} {}: {}",
                crate::i18n::t("ek_room"),
                room_name,
                area,
                if room.unit.is_empty() {
                    "m2"
                } else {
                    &room.unit
                },
                crate::i18n::t("chk_pb_room_desc")
            );
            if !pb_confirmed {
                desc.push('\n');
                desc.push_str(crate::i18n::t("chk_threshold_unconfirmed"));
            }
            out.push(b.make(
                "PRJ_PB_ROOM",
                "PR",
                Section::Pb,
                Severity::Critical,
                crate::i18n::t("chk_pb_room_title").to_string(),
                desc,
                room.room.clone(),
                room.mark.clone(),
                room.sheet.clone(),
                crate::i18n::t("chk_pb_room_fix").to_string(),
                crate::i18n::t("role_designer").to_string(),
            ));
        }

        // Evakuatsiya: xonada eshik bo'lishi shart.
        let has_door = rel_out
            .get(&room.id)
            .into_iter()
            .chain(rel_in.get(&room.id))
            .flatten()
            .any(|(l, e)| l.relation == Relation::Contains && e.kind == ElementKind::Door);
        if !has_door && doors_modelled {
            out.push(b.make(
                "PRJ_PB_EXIT",
                "PR",
                Section::Pb,
                Severity::Critical,
                crate::i18n::t("chk_pb_exit_title").to_string(),
                format!(
                    "{} «{}»: {}",
                    crate::i18n::t("ek_room"),
                    room_name,
                    crate::i18n::t("chk_pb_exit_desc")
                ),
                room.room.clone(),
                room.mark.clone(),
                room.sheet.clone(),
                crate::i18n::t("chk_pb_exit_fix").to_string(),
                crate::i18n::t("role_designer").to_string(),
            ));
        }
    }

    // O'chirish tizimi qurilmasiga suv ta'minoti (VK) kerak.
    for e in ctx
        .elements
        .iter()
        .filter(|e| e.section == Section::Pb && e.kind == ElementKind::Device)
    {
        let watered = rel_out
            .get(&e.id)
            .into_iter()
            .chain(rel_in.get(&e.id))
            .flatten()
            .any(|(_, o)| o.section == Section::Vk);
        if watered {
            continue;
        }
        out.push(b.make(
            "PRJ_PB_WATER",
            "PR",
            Section::Pb,
            Severity::Major,
            crate::i18n::t("chk_pb_water_title").to_string(),
            format!(
                "{} «{}» {}",
                e.kind.label(),
                e.mark,
                crate::i18n::t("chk_pb_water_desc")
            ),
            format!("{} {}", e.room, e.axis).trim().to_string(),
            e.mark.clone(),
            e.sheet.clone(),
            crate::i18n::t("chk_pb_water_fix").to_string(),
            crate::i18n::t("role_designer").to_string(),
        ));
    }

    // --- II.9 SS: kuchsiz tok ---
    // Kuchsiz tok qurilmasi ikki narsasiz ishlamaydi: kabel va o'rnatish joyi.
    for e in ctx
        .elements
        .iter()
        .filter(|e| e.section == Section::Ss && e.kind == ElementKind::Device)
    {
        let cabled = rel_out
            .get(&e.id)
            .into_iter()
            .chain(rel_in.get(&e.id))
            .flatten()
            .any(|(_, o)| o.kind == ElementKind::Cable);
        if !cabled {
            out.push(b.make(
                "PRJ_SS_CABLE",
                "PR",
                Section::Ss,
                Severity::Major,
                crate::i18n::t("chk_ss_cable_title").to_string(),
                format!(
                    "{} «{}» {}",
                    e.kind.label(),
                    e.mark,
                    crate::i18n::t("chk_ss_cable_desc")
                ),
                format!("{} {}", e.room, e.axis).trim().to_string(),
                e.mark.clone(),
                e.sheet.clone(),
                crate::i18n::t("chk_ss_cable_fix").to_string(),
                crate::i18n::t("role_designer").to_string(),
            ));
        }

        let placed = !e.room.trim().is_empty()
            || rel_in
                .get(&e.id)
                .map(|v| v.iter().any(|(l, _)| l.relation == Relation::Contains))
                .unwrap_or(false);
        if !placed {
            out.push(b.make(
                "PRJ_SS_PLACE",
                "PR",
                Section::Ss,
                Severity::Info,
                crate::i18n::t("chk_ss_place_title").to_string(),
                format!(
                    "{} «{}» {}",
                    e.kind.label(),
                    e.mark,
                    crate::i18n::t("chk_ss_place_desc")
                ),
                e.axis.clone(),
                e.mark.clone(),
                e.sheet.clone(),
                crate::i18n::t("chk_ss_place_fix").to_string(),
                crate::i18n::t("role_designer").to_string(),
            ));
        }
    }

    // --- II.12 Spetsifikatsiya: miqdor va o'lchov birligi ---
    // Spetsifikatsiya — bu «nima, qancha va qanday o'lchovda» degan
    // jadval. Miqdorsiz yoki birliksiz qator spetsifikatsiya emas:
    // undan na buyurtma berib bo'ladi, na smetaga qo'yib bo'ladi.
    for e in ctx.elements {
        let countable = matches!(
            e.kind,
            ElementKind::Door | ElementKind::Window | ElementKind::Device
        );
        if countable && e.size <= 0.0 && e.value <= 0.0 {
            out.push(b.make(
                "PRJ_SPEC_QTY",
                "PR",
                e.section,
                Severity::Major,
                crate::i18n::t("chk_spec_qty_title").to_string(),
                format!(
                    "{} «{}» {}",
                    e.kind.label(),
                    e.mark,
                    crate::i18n::t("chk_spec_qty_desc")
                ),
                format!("{} {}", e.room, e.axis).trim().to_string(),
                e.mark.clone(),
                e.sheet.clone(),
                crate::i18n::t("chk_spec_qty_fix").to_string(),
                crate::i18n::t("role_designer").to_string(),
            ));
        }
        // Son bor, birlik yo'q: «250» — bu metrmi, millimetrmi yoki dona?
        if e.size > 0.0 && e.unit.trim().is_empty() {
            out.push(b.make(
                "PRJ_SPEC_UNIT",
                "PR",
                e.section,
                Severity::Info,
                crate::i18n::t("chk_spec_unit_title").to_string(),
                format!(
                    "{} «{}»: {}",
                    e.kind.label(),
                    e.mark,
                    crate::i18n::t("chk_spec_unit_desc")
                ),
                format!("{} {}", e.room, e.axis).trim().to_string(),
                e.mark.clone(),
                e.sheet.clone(),
                crate::i18n::t("chk_spec_unit_fix").to_string(),
                crate::i18n::t("role_designer").to_string(),
            ));
        }
    }

    // --- II.14 Qurilish amalga oshirilishi ---
    // Teshik o'zi joylashgan konstruksiyadan katta bo'lsa, uni qurib
    // bo'lmaydi: konstruksiyadan hech narsa qolmaydi.
    for l in ctx.links {
        if l.relation != Relation::Contains {
            continue;
        }
        let (Some(host), Some(hole)) = (by_id.get(&l.from_el), by_id.get(&l.to_el)) else {
            continue;
        };
        if hole.kind != ElementKind::Opening {
            continue;
        }
        let structural = matches!(
            host.kind,
            ElementKind::Beam | ElementKind::Column | ElementKind::Slab | ElementKind::Wall
        );
        if !structural || host.size <= 0.0 || hole.size <= 0.0 {
            continue;
        }
        if hole.size >= host.size {
            out.push(b.make(
                "PRJ_BUILD_SIZE",
                "PR",
                host.section,
                Severity::Critical,
                crate::i18n::t("chk_build_size_title").to_string(),
                format!(
                    "{} «{}» {:.0} {} ≥ {} «{}» {:.0}",
                    hole.kind.label(),
                    hole.mark,
                    hole.size,
                    crate::i18n::t("chk_build_size_desc"),
                    host.kind.label(),
                    host.mark,
                    host.size
                ),
                format!("{} {}", host.room, host.axis).trim().to_string(),
                hole.mark.clone(),
                hole.sheet.clone(),
                crate::i18n::t("chk_build_size_fix").to_string(),
                crate::i18n::t("role_designer").to_string(),
            ));
        }
    }

    // Qavatda konstruksiya yo'q bo'lsa, unga hech narsani mahkamlab
    // bo'lmaydi. Tekshiruv faqat qavatlar ko'rsatilgan loyihalarda
    // ishlaydi: qavatsiz modelda bu savol o'rinsiz.
    let levels_used = ctx.elements.iter().any(|e| !e.level.trim().is_empty());
    if levels_used {
        let mut levels: Vec<String> = Vec::new();
        for e in ctx.elements {
            let l = e.level.trim().to_string();
            if !l.is_empty() && !levels.contains(&l) {
                levels.push(l);
            }
        }
        for level in levels {
            let has_structure = ctx.elements.iter().any(|e| {
                e.level.trim() == level
                    && matches!(e.section, Section::Kj | Section::Km)
                    && matches!(
                        e.kind,
                        ElementKind::Column | ElementKind::Beam | ElementKind::Slab
                    )
            });
            if has_structure {
                continue;
            }
            // Shu qavatda o'rnatiladigan qurilma yoki tarmoq bormi.
            let mounted: Vec<&Element> = ctx
                .elements
                .iter()
                .filter(|e| {
                    e.level.trim() == level
                        && matches!(
                            e.kind,
                            ElementKind::Device | ElementKind::Duct | ElementKind::Pipe
                        )
                })
                .collect();
            if let Some(first) = mounted.first() {
                out.push(b.make(
                    "PRJ_BUILD_LEVEL",
                    "PR",
                    first.section,
                    Severity::Major,
                    crate::i18n::t("chk_build_level_title").to_string(),
                    format!(
                        "{} {}: {} {}",
                        crate::i18n::t("chk_build_level_desc"),
                        level,
                        mounted.len(),
                        crate::i18n::t("chk_build_level_count")
                    ),
                    level.clone(),
                    first.mark.clone(),
                    first.sheet.clone(),
                    crate::i18n::t("chk_build_level_fix").to_string(),
                    crate::i18n::t("role_designer").to_string(),
                ));
            }
        }
    }

    // ================= II.3-8. Bo'limlar bo'yicha muhandislik hisobi =================
    //
    // Quyidagi tekshiruvlar **hisoblanadigan** qoidalar: har birining
    // orqasida oddiy formula turadi va formulaning taxminlari ekranda
    // ochiq yoziladi. Ma'lumot yetishmasa tekshiruv **o'tkazilmaydi** —
    // taxmin qilib xulosa chiqarilmaydi.

    // --- II.3 AR: eshik kengligi ---
    let (door_min, door_confirmed) = ctx.threshold("PRJ_AR_DOOR", 800.0);
    for e in ctx
        .elements
        .iter()
        .filter(|e| e.kind == ElementKind::Door && e.size > 0.0)
    {
        // O'lchov millimetrda kutiladi; metrda berilgan bo'lsa o'giramiz.
        let width = if e.size < 10.0 {
            e.size * 1000.0
        } else {
            e.size
        };
        if width >= door_min {
            continue;
        }
        let mut desc = format!(
            "{}: {:.0} {}; {}: {:.0}",
            crate::i18n::t("chk_ar_door_actual"),
            width,
            crate::i18n::t("unit_mm"),
            crate::i18n::t("chk_ar_door_min"),
            door_min
        );
        if !door_confirmed {
            desc.push('\n');
            desc.push_str(crate::i18n::t("chk_threshold_unconfirmed"));
        }
        out.push(b.make(
            "PRJ_AR_DOOR",
            "PR",
            Section::Ar,
            Severity::Major,
            crate::i18n::t("chk_ar_door_title").to_string(),
            desc,
            format!("{} {}", e.room, e.axis).trim().to_string(),
            e.mark.clone(),
            e.sheet.clone(),
            crate::i18n::t("chk_ar_door_fix").to_string(),
            crate::i18n::t("role_designer").to_string(),
        ));
    }

    // --- II.3 AR: tabiiy yoritish ---
    // Deraza yuzasining xona maydoniga nisbati. Nisbat reyestrda
    // sozlanadi; sukut bo'yicha 1/8 — turar joy uchun keng tarqalgan talab.
    let (light_ratio, light_confirmed) = ctx.threshold("PRJ_AR_LIGHT", 0.125);
    for room in ctx.elements.iter().filter(|e| e.kind == ElementKind::Room) {
        let area = if room.size > 0.0 {
            room.size
        } else {
            room.value
        };
        if area <= 0.0 {
            continue;
        }
        // Xonaga tegishli derazalar: `Contains` bog'lanishi bo'yicha.
        let windows: Vec<&Element> = rel_out
            .get(&room.id)
            .into_iter()
            .chain(rel_in.get(&room.id))
            .flatten()
            .filter(|(l, o)| l.relation == Relation::Contains && o.kind == ElementKind::Window)
            .map(|(_, o)| *o)
            .collect();
        if windows.is_empty() {
            continue;
        }
        let glass: f64 = windows
            .iter()
            .map(|w| if w.size > 0.0 { w.size } else { w.value })
            .sum();
        if glass <= 0.0 {
            continue;
        }
        let ratio = glass / area;
        if ratio >= light_ratio {
            continue;
        }
        let mut desc = format!(
            "{}: {:.2} / {:.2} = 1/{:.0}; {}: 1/{:.0}",
            crate::i18n::t("chk_ar_light_actual"),
            glass,
            area,
            1.0 / ratio.max(0.0001),
            crate::i18n::t("chk_ar_light_min"),
            1.0 / light_ratio
        );
        if !light_confirmed {
            desc.push('\n');
            desc.push_str(crate::i18n::t("chk_threshold_unconfirmed"));
        }
        out.push(b.make(
            "PRJ_AR_LIGHT",
            "PR",
            Section::Ar,
            Severity::Major,
            crate::i18n::t("chk_ar_light_title").to_string(),
            desc,
            room.room.clone(),
            room.mark.clone(),
            room.sheet.clone(),
            crate::i18n::t("chk_ar_light_fix").to_string(),
            crate::i18n::t("role_designer").to_string(),
        ));
    }

    // --- II.4 KJ: kesim o'lchami va beton sinfi ---
    let (min_section, section_confirmed) = ctx.threshold("PRJ_KJ_SECTION", 200.0);
    for e in ctx.elements.iter().filter(|e| {
        e.section == Section::Kj
            && matches!(
                e.kind,
                ElementKind::Column | ElementKind::Beam | ElementKind::Slab
            )
    }) {
        // Beton sinfi: marka yoki izohda «B25», «C20/25» ko'rinishida.
        let text = format!("{} {}", e.mark, e.note).to_lowercase();
        let has_class = text
            .split(|c: char| !c.is_alphanumeric() && c != '/')
            .any(|w| {
                (w.starts_with('b') || w.starts_with('c') || w.starts_with('в'))
                    && w.chars().skip(1).any(|c| c.is_ascii_digit())
            });
        if !has_class {
            out.push(b.make(
                "PRJ_KJ_CLASS",
                "PR",
                Section::Kj,
                Severity::Major,
                crate::i18n::t("chk_kj_class_title").to_string(),
                format!(
                    "{} «{}» {}",
                    e.kind.label(),
                    e.mark,
                    crate::i18n::t("chk_kj_class_desc")
                ),
                format!("{} {}", e.room, e.axis).trim().to_string(),
                e.mark.clone(),
                e.sheet.clone(),
                crate::i18n::t("chk_kj_class_fix").to_string(),
                crate::i18n::t("role_designer").to_string(),
            ));
        }

        if e.size <= 0.0 {
            continue;
        }
        let size = if e.size < 10.0 {
            e.size * 1000.0
        } else {
            e.size
        };
        // Plita uchun chegara pastroq: u qalinlik bo'yicha o'lchanadi.
        let limit = if e.kind == ElementKind::Slab {
            min_section * 0.4
        } else {
            min_section
        };
        if size >= limit {
            continue;
        }
        let mut desc = format!(
            "{}: {:.0} {}; {}: {:.0}",
            crate::i18n::t("chk_kj_section_actual"),
            size,
            crate::i18n::t("unit_mm"),
            crate::i18n::t("chk_kj_section_min"),
            limit
        );
        if !section_confirmed {
            desc.push('\n');
            desc.push_str(crate::i18n::t("chk_threshold_unconfirmed"));
        }
        out.push(b.make(
            "PRJ_KJ_SECTION",
            "PR",
            Section::Kj,
            Severity::Critical,
            crate::i18n::t("chk_kj_section_title").to_string(),
            desc,
            format!("{} {}", e.room, e.axis).trim().to_string(),
            e.mark.clone(),
            e.sheet.clone(),
            crate::i18n::t("chk_kj_section_fix").to_string(),
            crate::i18n::t("role_designer").to_string(),
        ));
    }

    // --- II.5 KM: po'lat markasi ---
    // Payvand chokini belgilash uchun po'lat markasi kerak: marka
    // bo'lmasa, elektrod ham, chok o'lchami ham tanlanmaydi.
    for e in ctx.elements.iter().filter(|e| {
        e.section == Section::Km && matches!(e.kind, ElementKind::Column | ElementKind::Beam)
    }) {
        let text = format!("{} {}", e.mark, e.note).to_lowercase();
        let has_steel = [
            "s235", "s245", "s255", "s345", "09g2s", "09г2с", "ст3", "st3",
        ]
        .iter()
        .any(|k| text.contains(k));
        if has_steel {
            continue;
        }
        out.push(b.make(
            "PRJ_KM_STEEL",
            "PR",
            Section::Km,
            Severity::Major,
            crate::i18n::t("chk_km_steel_title").to_string(),
            format!(
                "{} «{}» {}",
                e.kind.label(),
                e.mark,
                crate::i18n::t("chk_km_steel_desc")
            ),
            format!("{} {}", e.room, e.axis).trim().to_string(),
            e.mark.clone(),
            e.sheet.clone(),
            crate::i18n::t("chk_km_steel_fix").to_string(),
            crate::i18n::t("role_designer").to_string(),
        ));
    }

    // --- II.6 VK: suv tezligi ---
    // v = Q / A. Diametr `size` (mm), sarf `value` (l/s) bo'lganda
    // hisoblanadi; ikkalasi bo'lmasa tekshiruv o'tkazilmaydi.
    let (max_water, water_confirmed) = ctx.threshold("PRJ_VK_VELOCITY", 3.0);
    for e in ctx.elements.iter().filter(|e| {
        e.section == Section::Vk && e.kind == ElementKind::Pipe && e.size > 0.0 && e.value > 0.0
    }) {
        let name = e.value_name.to_lowercase();
        if !["sarf", "расход", "flow", "q"]
            .iter()
            .any(|k| name.contains(k))
        {
            continue;
        }
        let d_m = if e.size < 10.0 {
            e.size
        } else {
            e.size / 1000.0
        };
        let area = std::f64::consts::PI * d_m * d_m / 4.0;
        if area <= 0.0 {
            continue;
        }
        // l/s -> m3/s.
        let v = (e.value / 1000.0) / area;
        if v <= max_water {
            continue;
        }
        let mut desc = format!(
            "{}: {:.2} {}; {}: {:.2}",
            crate::i18n::t("chk_velocity_actual"),
            v,
            crate::i18n::t("unit_ms"),
            crate::i18n::t("chk_velocity_max"),
            max_water
        );
        desc.push('\n');
        desc.push_str(crate::i18n::t("chk_vk_velocity_note"));
        if !water_confirmed {
            desc.push('\n');
            desc.push_str(crate::i18n::t("chk_threshold_unconfirmed"));
        }
        out.push(b.make(
            "PRJ_VK_VELOCITY",
            "PR",
            Section::Vk,
            Severity::Major,
            crate::i18n::t("chk_vk_velocity_title").to_string(),
            desc,
            format!("{} {}", e.room, e.axis).trim().to_string(),
            e.mark.clone(),
            e.sheet.clone(),
            crate::i18n::t("chk_vk_velocity_fix").to_string(),
            crate::i18n::t("role_designer").to_string(),
        ));
    }

    // --- II.7 OV: havo tezligi ---
    // v = L / (3600 · A). Sarf `value` (m3/soat), kesim `size` (mm).
    let (max_air, air_confirmed) = ctx.threshold("PRJ_OV_VELOCITY", 6.0);
    for e in ctx.elements.iter().filter(|e| {
        e.section == Section::Ov && e.kind == ElementKind::Duct && e.size > 0.0 && e.value > 0.0
    }) {
        let name = e.value_name.to_lowercase();
        if !["sarf", "расход", "havo", "воздух", "l"]
            .iter()
            .any(|k| name.contains(k))
        {
            continue;
        }
        let d_m = if e.size < 10.0 {
            e.size
        } else {
            e.size / 1000.0
        };
        let area = std::f64::consts::PI * d_m * d_m / 4.0;
        if area <= 0.0 {
            continue;
        }
        let v = e.value / 3600.0 / area;
        if v <= max_air {
            continue;
        }
        let mut desc = format!(
            "{}: {:.2} {}; {}: {:.2}",
            crate::i18n::t("chk_velocity_actual"),
            v,
            crate::i18n::t("unit_ms"),
            crate::i18n::t("chk_velocity_max"),
            max_air
        );
        desc.push('\n');
        desc.push_str(crate::i18n::t("chk_ov_velocity_note"));
        if !air_confirmed {
            desc.push('\n');
            desc.push_str(crate::i18n::t("chk_threshold_unconfirmed"));
        }
        out.push(b.make(
            "PRJ_OV_VELOCITY",
            "PR",
            Section::Ov,
            Severity::Major,
            crate::i18n::t("chk_ov_velocity_title").to_string(),
            desc,
            format!("{} {}", e.room, e.axis).trim().to_string(),
            e.mark.clone(),
            e.sheet.clone(),
            crate::i18n::t("chk_ov_velocity_fix").to_string(),
            crate::i18n::t("role_designer").to_string(),
        ));
    }

    // --- II.8 EOM: kabel kesimi ---
    // I = P / (√3 · U · cosφ), keyin S = I / J. Taxminlar ekranda ochiq
    // yoziladi: 380 V, cosφ 0.85, mis o'tkazgich, J = 5 A/mm2.
    let (density, density_confirmed) = ctx.threshold("PRJ_EOM_SECTION", 5.0);
    for device in ctx
        .elements
        .iter()
        .filter(|e| e.section == Section::Eom && e.kind == ElementKind::Device && e.value > 0.0)
    {
        let name = device.value_name.to_lowercase();
        if !["quvvat", "мощн", "power", "kw", "kvt"]
            .iter()
            .any(|k| name.contains(k))
        {
            continue;
        }
        // Qurilmani ta'minlaydigan kabel.
        let cable = rel_out
            .get(&device.id)
            .into_iter()
            .chain(rel_in.get(&device.id))
            .flatten()
            .map(|(_, o)| *o)
            .find(|o| o.kind == ElementKind::Cable && o.size > 0.0);
        let Some(cable) = cable else { continue };

        let current = device.value * 1000.0 / (3.0_f64.sqrt() * 380.0 * 0.85);
        let needed = current / density;
        if cable.size >= needed {
            continue;
        }
        let mut desc = format!(
            "{}: {:.1} {}; {}: {:.1} {}; {}: {:.1}",
            crate::i18n::t("chk_eom_current"),
            current,
            crate::i18n::t("unit_a"),
            crate::i18n::t("chk_eom_needed"),
            needed,
            crate::i18n::t("unit_mm2"),
            crate::i18n::t("chk_eom_actual"),
            cable.size
        );
        desc.push('\n');
        desc.push_str(crate::i18n::t("chk_eom_note"));
        if !density_confirmed {
            desc.push('\n');
            desc.push_str(crate::i18n::t("chk_threshold_unconfirmed"));
        }
        out.push(
            b.make(
                "PRJ_EOM_SECTION",
                "PR",
                Section::Eom,
                Severity::Critical,
                crate::i18n::t("chk_eom_title").to_string(),
                desc,
                format!("{} {}", device.room, device.axis)
                    .trim()
                    .to_string(),
                cable.mark.clone(),
                cable.sheet.clone(),
                crate::i18n::t("chk_eom_fix").to_string(),
                crate::i18n::t("role_designer").to_string(),
            ),
        );
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
#[derive(Debug, Clone, Default)]
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
                    // Kirim va yetkazib beruvchiga qaytarish ishga sarf emas.
                    MoveKind::In | MoveKind::ToSupplier => 0.0,
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

// ================= XII. Material: tarix, kuzatuvchanlik, reyting =================

/// Materialning bitta kirimi — narx tarixi uchun (TZ XII.19).
#[derive(Debug, Clone)]
pub struct PricePoint {
    pub date: NaiveDate,
    pub price: f64,
    pub qty: f64,
    pub supplier: String,
    pub document: String,
    /// Oldingi kirimdan farq, foizda. Birinchi kirimda `None`.
    pub change_pct: Option<f64>,
}

/// TZ XII.19: material narxining kirimlar bo'yicha tarixi.
///
/// Narx katalogda bitta son bo'lib turadi, lekin u vaqt o'tishi bilan
/// o'zgaradi. Tarix ombor kirimlaridan olinadi — bu haqiqatda to'langan narx,
/// katalogdagi taxmin emas.
pub fn price_history(material_id: i64, moves: &[StockMove]) -> Vec<PricePoint> {
    let mut list: Vec<&StockMove> = moves
        .iter()
        .filter(|m| m.material_id == material_id)
        .filter(|m| m.kind == MoveKind::In && m.price > 0.0)
        .collect();
    list.sort_by_key(|m| (m.date, m.id));

    let mut out: Vec<PricePoint> = Vec::with_capacity(list.len());
    let mut prev: Option<f64> = None;
    for m in list {
        out.push(PricePoint {
            date: m.date,
            price: m.price,
            qty: m.qty,
            supplier: m.counterparty.clone(),
            document: m.document.clone(),
            change_pct: prev.filter(|p| *p > 0.0).map(|p| (m.price - p) / p * 100.0),
        });
        prev = Some(m.price);
    }
    out
}

/// Materialning to'liq kuzatuvchanligi (TZ XII.29–30).
#[derive(Debug, Clone, Default)]
pub struct MaterialTrace {
    /// Qayerdan kelgan: yetkazib beruvchilar.
    pub suppliers: Vec<String>,
    /// Qaysi partiyalar bilan kelgan.
    pub batches: Vec<i64>,
    /// Qaysi ishlarga berilgan.
    pub tasks: Vec<i64>,
    /// Shu ishlarni qamragan ijro hujjatlari.
    pub docs: Vec<i64>,
    /// Shu materialga tegishli sifat tekshiruvlari.
    pub checks: Vec<i64>,
    pub received: f64,
    pub issued: f64,
}

/// TZ XII.30: material qayerdan kelib, qayerga ketganini bir joyda ko'rsatadi.
///
/// Zanjir hujjatlarga borib taqaladi: material → ish → ijro hujjati. Shunda
/// «bu beton qaysi dalolatnomaga kirgan» degan savolga javob bor.
pub fn material_trace(
    material_id: i64,
    moves: &[StockMove],
    docs: &[ExecDoc],
    quality: &[QualityCheck],
) -> MaterialTrace {
    let mine: Vec<&StockMove> = moves
        .iter()
        .filter(|m| m.material_id == material_id)
        .collect();
    let mut t = MaterialTrace {
        received: mine
            .iter()
            .filter(|m| m.kind == MoveKind::In)
            .map(|m| m.qty)
            .sum(),
        issued: mine
            .iter()
            .filter(|m| m.kind == MoveKind::Out)
            .map(|m| m.qty)
            .sum(),
        ..Default::default()
    };

    t.suppliers = mine
        .iter()
        .filter(|m| m.kind == MoveKind::In)
        .map(|m| m.counterparty.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    t.suppliers.sort();
    t.suppliers.dedup();

    t.batches = mine.iter().filter_map(|m| m.batch_id).collect();
    t.batches.sort_unstable();
    t.batches.dedup();

    t.tasks = mine
        .iter()
        .filter(|m| m.kind == MoveKind::Out)
        .filter_map(|m| m.task_id)
        .collect();
    t.tasks.sort_unstable();
    t.tasks.dedup();

    // Ijro hujjati ishga bog'langan bo'lsa — zanjir shu yerda yopiladi.
    t.docs = docs
        .iter()
        .filter(|d| d.task_id.is_some_and(|id| t.tasks.contains(&id)))
        .map(|d| d.id)
        .collect();

    t.checks = quality
        .iter()
        .filter(|q| {
            q.material_id == Some(material_id) || q.task_id.is_some_and(|id| t.tasks.contains(&id))
        })
        .map(|q| q.id)
        .collect();
    t
}

/// Material reytingi (TZ XII.35).
#[derive(Debug, Clone)]
pub struct MaterialRating {
    pub material_id: i64,
    /// Nechta kirim bo'lgan.
    pub deliveries: usize,
    /// Oxirgi narx.
    pub last_price: f64,
    /// Birinchi kirimdan narx o'zgarishi, foizda.
    pub price_change_pct: f64,
    /// Kirish nazoratida nechta marta rad etilgan.
    pub rejected: usize,
    /// Jami kirish nazorati.
    pub checks: usize,
    /// Sifat ulushi, foizda. Tekshiruv bo'lmasa `None` — nol emas.
    pub pass_pct: Option<f64>,
}

/// TZ XII.35: materialni tarixdan baholaydi.
///
/// Ball qo'yilmaydi — sonlar ko'rsatiladi: nechta kirim, narx qanday
/// o'zgargan, kirish nazoratidan qanday o'tgan. Bitta raqamga siqib qo'yish
/// qaror qabul qilishga yordam bermaydi, sabab ko'rinmay qoladi.
pub fn material_ratings(
    materials: &[Material],
    moves: &[StockMove],
    quality: &[QualityCheck],
) -> Vec<MaterialRating> {
    materials
        .iter()
        .map(|m| {
            let hist = price_history(m.id, moves);
            let checks: Vec<&QualityCheck> = quality
                .iter()
                .filter(|q| q.material_id == Some(m.id))
                .filter(|q| q.kind == QualityKind::Input)
                .collect();
            let rejected = checks
                .iter()
                .filter(|q| q.result == QualityResult::Fail)
                .count();
            MaterialRating {
                material_id: m.id,
                deliveries: hist.len(),
                last_price: hist.last().map_or(0.0, |p| p.price),
                price_change_pct: match (hist.first(), hist.last()) {
                    (Some(a), Some(b)) if a.price > 0.0 => (b.price - a.price) / a.price * 100.0,
                    _ => 0.0,
                },
                rejected,
                checks: checks.len(),
                pass_pct: (!checks.is_empty())
                    .then(|| (checks.len() - rejected) as f64 / checks.len() as f64 * 100.0),
            }
        })
        .collect()
}

/// Brak yozuvi (TZ XII.36–37).
#[derive(Debug, Clone)]
pub struct DefectLine {
    pub material_id: i64,
    /// Kirish nazoratida rad etilgan marta.
    pub rejected: usize,
    /// Hisobdan chiqarilgan miqdor.
    pub written_off: f64,
    /// Yetkazib beruvchiga qaytarilgan miqdor.
    pub returned: f64,
    /// Qaytarilmagan brak: rad etilgan, lekin ombordan chiqmagan.
    pub unresolved: bool,
    pub reasons: Vec<String>,
}

/// TZ XII.36: brakka chiqarilgan material va u bilan nima bo'lgani.
///
/// Rad etilgan material ombordan chiqmagan bo'lsa — bu ochiq masala: u hali
/// ham ishlatilishi mumkin. Shuning uchun alohida belgilanadi.
pub fn defect_lines(
    materials: &[Material],
    moves: &[StockMove],
    quality: &[QualityCheck],
) -> Vec<DefectLine> {
    materials
        .iter()
        .filter_map(|m| {
            let bad: Vec<&QualityCheck> = quality
                .iter()
                .filter(|q| q.material_id == Some(m.id) && q.result == QualityResult::Fail)
                .collect();
            if bad.is_empty() {
                return None;
            }
            let sum = |k: MoveKind| -> f64 {
                moves
                    .iter()
                    .filter(|x| x.material_id == m.id && x.kind == k)
                    .map(|x| x.qty)
                    .sum()
            };
            let written_off = sum(MoveKind::WriteOff);
            let returned = sum(MoveKind::ToSupplier);
            Some(DefectLine {
                material_id: m.id,
                rejected: bad.len(),
                written_off,
                returned,
                unresolved: written_off + returned <= 0.0001,
                reasons: bad
                    .iter()
                    .map(|q| q.defect.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
            })
        })
        .collect()
}

/// Ish boshlanishidan oldin material yetarlimi (TZ XII.28).
#[derive(Debug, Clone)]
pub struct Readiness {
    pub task_id: i64,
    pub material_id: i64,
    /// Ish uchun kerak bo'lgan miqdor: norma × butun hajm.
    pub needed: f64,
    /// Omborda erkin qoldiq.
    pub available: f64,
    /// Yetishmayotgan miqdor.
    pub short: f64,
    /// Ish shuncha kundan keyin boshlanadi. Manfiy — allaqachon boshlangan.
    pub days_left: i64,
}

/// TZ XII.28: yaqinda boshlanadigan ishlar uchun material yetarlimi.
///
/// Tekshiruv **butun hajmga** qaraydi, bajarilganiga emas: ish boshlanishidan
/// oldin materialning hammasi kerak bo'lmasa ham, yetishmasligini oldindan
/// bilish kerak — buyurtma vaqt oladi.
pub fn readiness(
    norms: &[MaterialNorm],
    tasks: &[Task],
    stock: &[StockLine],
    today: NaiveDate,
    within_days: i64,
) -> Vec<Readiness> {
    norms
        .iter()
        .filter_map(|n| {
            let task = tasks.iter().find(|t| t.id == n.task_id)?;
            // Tugallangan ishga material kerak emas.
            if task.progress >= 100.0 {
                return None;
            }
            let start = task.fact_start.unwrap_or(task.plan_start);
            let days_left = (start - today).num_days();
            if days_left > within_days {
                return None;
            }
            let needed = n.per_unit * task.volume;
            let available = stock
                .iter()
                .find(|l| l.material_id == n.material_id)
                .map_or(0.0, |l| l.available);
            let short = needed - available;
            (short > 0.0001).then_some(Readiness {
                task_id: n.task_id,
                material_id: n.material_id,
                needed,
                available,
                short,
                days_left,
            })
        })
        .collect()
}

// ================= III. Smeta: tuzilish, bog'lanish, variantlar =================

/// Smeta tuzilishidagi kamchilik (TZ III.17–20).
#[derive(Debug, Clone, PartialEq)]
pub enum EstimateIssue {
    /// Ustama xarajat foizi ko'rsatilmagan.
    NoOverhead,
    /// Smeta foydasi ko'rsatilmagan.
    NoProfit,
    /// QQS ko'rsatilmagan.
    NoVat,
    /// Koeffitsiyent haqiqatga to'g'ri kelmaydigan darajada katta.
    Suspicious { name: &'static str, pct: f64 },
    /// Hisoblangan yakuniy summa hujjatdagidan farq qiladi.
    TotalMismatch { computed: f64, declared: f64 },
}

/// Koeffitsiyent shu foizdan oshsa — tekshirish kerak.
///
/// Chegara qat'iy taqiq emas: murakkab obyektlarda ustama yuqori bo'lishi
/// mumkin. Lekin 60% dan oshgan ustama yoki 40% dan oshgan foyda odatda
/// xato yoki asossiz — buni jim o'tkazib yubormaymiz.
pub const OVERHEAD_LIMIT: f64 = 60.0;
pub const PROFIT_LIMIT: f64 = 40.0;

/// TZ III.17–20: smetaning tuzilishini tekshiradi.
///
/// Nol koeffitsiyent «xato» emas, «ko'rsatilmagan»: smeta to'liq emas degani.
/// Shuning uchun bu ogohlantirish, taqiq emas.
pub fn estimate_issues(estimate: &Estimate, items: &[EstimateItem]) -> Vec<EstimateIssue> {
    let mut out = Vec::new();
    if estimate.overhead_pct <= 0.0 {
        out.push(EstimateIssue::NoOverhead);
    } else if estimate.overhead_pct > OVERHEAD_LIMIT {
        out.push(EstimateIssue::Suspicious {
            name: "overhead",
            pct: estimate.overhead_pct,
        });
    }
    if estimate.profit_pct <= 0.0 {
        out.push(EstimateIssue::NoProfit);
    } else if estimate.profit_pct > PROFIT_LIMIT {
        out.push(EstimateIssue::Suspicious {
            name: "profit",
            pct: estimate.profit_pct,
        });
    }
    if estimate.vat_pct <= 0.0 {
        out.push(EstimateIssue::NoVat);
    }

    // Hujjatdagi summa hisoblangani bilan solishtiriladi.
    let direct: f64 = items
        .iter()
        .filter(|i| i.estimate_id == estimate.id)
        .map(|i| i.computed())
        .sum();
    let totals = estimate.totals(direct);
    if estimate.declared_total > 0.0 {
        let diff = (totals.total - estimate.declared_total).abs();
        // 0.1% — yaxlitlash farqi; undan kattasi haqiqiy nomuvofiqlik.
        if diff > estimate.declared_total * 0.001 {
            out.push(EstimateIssue::TotalMismatch {
                computed: totals.total,
                declared: estimate.declared_total,
            });
        }
    }
    out
}

/// Smeta va GPR bog'lanishi (TZ III.26).
#[derive(Debug, Clone, Default)]
pub struct EstimateCoverage {
    /// Ishga bog'lanmagan pozitsiyalar va ularning summasi.
    pub free_items: Vec<i64>,
    pub free_cost: f64,
    /// Smetada pozitsiyasi yo'q ishlar.
    pub free_tasks: Vec<i64>,
    /// Bog'langan pozitsiyalar ulushi, foizda.
    pub linked_pct: f64,
}

/// TZ III.26: smeta pozitsiyalari GPR ishlariga bog'langanmi.
///
/// Bog'lanish ikki tomonlama tekshiriladi: pozitsiya ishsiz qolsa uni kim
/// bajarishi noma'lum, ish pozitsiyasiz qolsa uning qiymati noma'lum.
/// Nomi bo'yicha taxminiy moslik hisoblanmaydi — bog'lanish aniq bo'lishi
/// kerak, aks holda KS-2 da noto'g'ri narx chiqadi.
pub fn estimate_coverage(items: &[EstimateItem], tasks: &[Task]) -> EstimateCoverage {
    let mut c = EstimateCoverage::default();
    if items.is_empty() {
        return c;
    }
    for i in items {
        match i.task_id {
            Some(id) if tasks.iter().any(|t| t.id == id) => {}
            _ => {
                c.free_items.push(i.id);
                c.free_cost += i.computed();
            }
        }
    }
    c.free_tasks = tasks
        .iter()
        .filter(|t| !items.iter().any(|i| i.task_id == Some(t.id)))
        .map(|t| t.id)
        .collect();
    let linked = items.len() - c.free_items.len();
    c.linked_pct = linked as f64 / items.len() as f64 * 100.0;
    c
}

/// Smeta va byudjetni bo'lim kesimida solishtirish (TZ III.23).
#[derive(Debug, Clone)]
pub struct EstimateBudgetLine {
    pub section: Section,
    /// Smetadagi to'g'ridan-to'g'ri xarajat.
    pub estimate: f64,
    /// Bo'lim uchun belgilangan xarid byudjeti.
    pub budget: f64,
    /// Byudjet minus smeta. Manfiy — byudjet yetmaydi.
    pub gap: f64,
}

/// Bo'lim bo'yicha smeta va byudjetni yonma-yon qo'yadi.
///
/// Byudjet belgilanmagan bo'lim ham chiqadi: nol byudjet ham javob — u yerda
/// reja yo'q degani.
pub fn estimate_vs_budget(
    items: &[EstimateItem],
    budgets: &[PurchaseBudget],
) -> Vec<EstimateBudgetLine> {
    let mut sections: Vec<Section> = items.iter().map(|i| i.section).collect();
    sections.extend(budgets.iter().map(|b| b.section));
    sections.sort_by_key(|s| s.code());
    sections.dedup();

    sections
        .into_iter()
        .map(|section| {
            let estimate: f64 = items
                .iter()
                .filter(|i| i.section == section)
                .map(|i| i.computed())
                .sum();
            let budget: f64 = budgets
                .iter()
                .filter(|b| b.section == section)
                .map(|b| b.planned)
                .sum();
            EstimateBudgetLine {
                section,
                estimate,
                budget,
                gap: budget - estimate,
            }
        })
        .collect()
}

/// Ikki smeta variantini solishtirish natijasi (TZ III.21).
#[derive(Debug, Clone)]
pub struct EstimateDiffLine {
    pub section: Section,
    pub left: f64,
    pub right: f64,
    pub diff: f64,
    /// Farq foizda. Chap tomon nol bo'lsa `None`.
    pub diff_pct: Option<f64>,
}

/// Ikki smetani bo'lim kesimida solishtiradi.
pub fn compare_estimates(
    items: &[EstimateItem],
    left_id: i64,
    right_id: i64,
) -> Vec<EstimateDiffLine> {
    let mut sections: Vec<Section> = items
        .iter()
        .filter(|i| i.estimate_id == left_id || i.estimate_id == right_id)
        .map(|i| i.section)
        .collect();
    sections.sort_by_key(|s| s.code());
    sections.dedup();

    let sum = |eid: i64, section: Section| -> f64 {
        items
            .iter()
            .filter(|i| i.estimate_id == eid && i.section == section)
            .map(|i| i.computed())
            .sum()
    };
    sections
        .into_iter()
        .map(|section| {
            let left = sum(left_id, section);
            let right = sum(right_id, section);
            EstimateDiffLine {
                section,
                left,
                right,
                diff: right - left,
                diff_pct: (left > 0.0).then(|| (right - left) / left * 100.0),
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
            task_id: None,
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

// ================================================================ VII. Texnik nazorat

/// Tekshiruvlar bo'yicha yakun (TZ VII.3, 34).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InspectionSummary {
    /// Bugunga rejalashtirilgan.
    pub today: usize,
    /// Kelgusi yetti kun ichida.
    pub week: usize,
    /// Muddati o'tgan, lekin o'tkazilmagan.
    pub overdue: usize,
    /// O'tkazilgan va ijobiy.
    pub passed: usize,
    /// Salbiy natija, bartaraf etilmagan.
    pub open_defects: usize,
    /// Jami yozuv.
    pub total: usize,
}

/// Tekshiruvlar bo'yicha yakunni hisoblaydi.
pub fn inspection_summary(list: &[Inspection], today: NaiveDate) -> InspectionSummary {
    let week_end = today + chrono::Duration::days(7);
    InspectionSummary {
        today: list
            .iter()
            .filter(|x| x.open() && x.planned == today)
            .count(),
        week: list
            .iter()
            .filter(|x| x.open() && x.planned > today && x.planned <= week_end)
            .count(),
        overdue: list.iter().filter(|x| x.overdue(today)).count(),
        passed: list
            .iter()
            .filter(|x| !x.open() && x.result == InspectionResult::Pass)
            .count(),
        open_defects: list.iter().filter(|x| x.open_defect()).count(),
        total: list.len(),
    }
}

/// Kalendar uchun bir kun (TZ VII.3).
#[derive(Debug, Clone, PartialEq)]
pub struct InspectionDay {
    pub date: NaiveDate,
    pub ids: Vec<i64>,
    /// Shu kunda muddati o'tgan tekshiruv bormi.
    pub overdue: bool,
}

/// Tekshiruvlarni kunlar bo'yicha guruhlaydi — kalendar shu ro'yxatdan chiziladi.
///
/// Faqat o'tkazilmagan tekshiruvlar kiradi: kalendar «nima qilish kerak»
/// degan savolga javob beradi, arxiv emas.
pub fn inspection_calendar(list: &[Inspection], today: NaiveDate) -> Vec<InspectionDay> {
    let mut days: Vec<InspectionDay> = Vec::new();
    let mut open: Vec<&Inspection> = list.iter().filter(|x| x.open()).collect();
    open.sort_by_key(|x| x.planned);
    for x in open {
        match days.last_mut() {
            Some(d) if d.date == x.planned => {
                d.ids.push(x.id);
                d.overdue |= x.planned < today;
            }
            _ => days.push(InspectionDay {
                date: x.planned,
                ids: vec![x.id],
                overdue: x.planned < today,
            }),
        }
    }
    days
}

/// Beton sinovlari bo'yicha yakun (TZ VII.12).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConcreteSummary {
    pub total: usize,
    /// Natijasi kelgan sinovlar.
    pub tested: usize,
    pub passed: usize,
    pub failed: usize,
    /// Natijasi kutilayotgan, sinov sanasi kelgan.
    pub due: usize,
    /// O'rtacha ko'rsatkich, talabga nisbatan foizda.
    pub avg_pct: f64,
    /// Eng past natija, foizda.
    pub worst_pct: f64,
}

/// Beton sinovlarini yig'adi.
pub fn concrete_summary(list: &[ConcreteTest], today: NaiveDate) -> ConcreteSummary {
    let pcts: Vec<f64> = list.iter().filter_map(|x| x.pct()).collect();
    ConcreteSummary {
        total: list.len(),
        tested: list.iter().filter(|x| x.actual.is_some()).count(),
        passed: list.iter().filter(|x| x.passed() == Some(true)).count(),
        failed: list.iter().filter(|x| x.passed() == Some(false)).count(),
        due: list
            .iter()
            .filter(|x| x.actual.is_none() && x.test_date() <= today)
            .count(),
        avg_pct: if pcts.is_empty() {
            0.0
        } else {
            pcts.iter().sum::<f64>() / pcts.len() as f64
        },
        worst_pct: pcts.iter().copied().fold(f64::INFINITY, f64::min),
    }
}

/// Dopuskdan chiqqan geodezik nuqtalar (TZ VII.14).
///
/// Eng katta chetlanish oldinda: texnik nazorat avval shu nuqtaga qaraydi.
pub fn geodesy_issues(list: &[GeodesyPoint]) -> Vec<&GeodesyPoint> {
    let mut out: Vec<&GeodesyPoint> = list.iter().filter(|x| !x.within()).collect();
    out.sort_by(|a, b| {
        (b.deviation().abs() - b.tolerance).total_cmp(&(a.deviation().abs() - a.tolerance))
    });
    out
}

/// Texnik nazoratning kunlik hisoboti (TZ VII.34).
///
/// Bir kunda nima bo'lganini bitta yozuvga yig'adi: hisobot qo'lda emas,
/// bazadagi yozuvlardan tug'iladi.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SupervisionDay {
    pub date: NaiveDate,
    /// O'tkazilgan tekshiruvlar.
    pub inspections: usize,
    pub inspections_failed: usize,
    /// Shu kuni ochilgan nomuvofiqliklar.
    pub issues_opened: usize,
    /// Shu kuni yopilgan nomuvofiqliklar.
    pub issues_closed: usize,
    /// Sifat tekshiruvlari.
    pub quality_checks: usize,
    pub quality_failed: usize,
    /// Imzolangan ijro hujjatlari.
    pub docs_signed: usize,
    /// Beton sinovi natijalari.
    pub concrete_results: usize,
    /// Dopuskdan chiqqan geodezik nuqtalar.
    pub geodesy_out: usize,
}

impl SupervisionDay {
    /// Kunda hech narsa bo'lmagan bo'lsa hisobot ham keraksiz.
    pub fn is_empty(&self) -> bool {
        self.inspections == 0
            && self.issues_opened == 0
            && self.issues_closed == 0
            && self.quality_checks == 0
            && self.docs_signed == 0
            && self.concrete_results == 0
            && self.geodesy_out == 0
    }
}

/// Bir kunlik texnik nazorat hisobotini yig'adi.
pub fn supervision_day(
    day: NaiveDate,
    inspections: &[Inspection],
    issues: &[Issue],
    quality: &[QualityCheck],
    docs: &[ExecDoc],
    concrete: &[ConcreteTest],
    geodesy: &[GeodesyPoint],
) -> SupervisionDay {
    let done: Vec<&Inspection> = inspections.iter().filter(|x| x.done == Some(day)).collect();
    SupervisionDay {
        date: day,
        inspections: done.len(),
        inspections_failed: done
            .iter()
            .filter(|x| x.result != InspectionResult::Pass)
            .count(),
        // Nomuvofiqlikda sana matn ko'rinishida saqlanadi — boshini solishtiramiz.
        issues_opened: issues
            .iter()
            .filter(|i| i.created_at.starts_with(&day.to_string()))
            .count(),
        issues_closed: issues
            .iter()
            .filter(|i| i.status == IssueStatus::Fixed && i.deadline == Some(day))
            .count(),
        quality_checks: quality.iter().filter(|q| q.date == day).count(),
        quality_failed: quality
            .iter()
            .filter(|q| q.date == day && q.result != QualityResult::Pass)
            .count(),
        docs_signed: docs
            .iter()
            .filter(|d| d.date == day && d.status == ExecDocStatus::Signed)
            .count(),
        concrete_results: concrete
            .iter()
            .filter(|c| c.actual.is_some() && c.test_date() == day)
            .count(),
        geodesy_out: geodesy
            .iter()
            .filter(|g| g.measured == day && !g.within())
            .count(),
    }
}

// ================================================================ VIII. Buyurtmachi

/// Shartnoma bo'yicha yakuniy holat (TZ VIII.27-28).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContractState {
    pub contract_id: i64,
    /// Shartnomadagi dastlabki summa.
    pub base: f64,
    /// Tasdiqlangan o'zgarishlar summasi (musbat yoki manfiy).
    pub approved_changes: f64,
    /// Qaror kutayotgan o'zgarishlar summasi.
    pub pending_changes: f64,
    /// Amaldagi summa: dastlabki + tasdiqlangan.
    pub current: f64,
    /// Tasdiqlangan muddat surilishi, kunlarda.
    pub approved_days: i64,
    /// To'lov jadvali bo'yicha jami.
    pub planned: f64,
    pub paid: f64,
    /// Muddati o'tgan qarz.
    pub overdue: f64,
    /// Eng katta kechikish, kunlarda.
    pub max_delay: i64,
}

impl ContractState {
    /// Dastlabki summadan chetlanish, foizda.
    pub fn change_pct(&self) -> f64 {
        if self.base <= 0.0 {
            return 0.0;
        }
        self.approved_changes / self.base * 100.0
    }

    /// To'lov jadvali amaldagi summani qoplaydimi.
    ///
    /// Farq katta bo'lsa — jadval eskirgan yoki to'liq tuzilmagan.
    pub fn schedule_gap(&self) -> f64 {
        self.current - self.planned
    }
}

/// Bitta shartnoma bo'yicha holatni yig'adi.
pub fn contract_state(
    c: &Contract,
    changes: &[ContractChange],
    stages: &[PaymentStage],
    today: NaiveDate,
) -> ContractState {
    let mine = |id: Option<i64>| id == Some(c.id);
    let approved: f64 = changes
        .iter()
        .filter(|x| mine(x.contract_id) && x.counts())
        .map(|x| x.amount)
        .sum();
    let pending: f64 = changes
        .iter()
        .filter(|x| mine(x.contract_id) && x.pending())
        .map(|x| x.amount)
        .sum();
    let my_stages: Vec<&PaymentStage> = stages.iter().filter(|x| mine(x.contract_id)).collect();

    ContractState {
        contract_id: c.id,
        base: c.sum,
        approved_changes: approved,
        pending_changes: pending,
        current: c.sum + approved,
        approved_days: changes
            .iter()
            .filter(|x| mine(x.contract_id) && x.counts())
            .map(|x| x.days)
            .sum(),
        planned: my_stages.iter().map(|x| x.amount).sum(),
        paid: my_stages.iter().map(|x| x.paid).sum(),
        overdue: my_stages
            .iter()
            .filter(|x| x.overdue(today))
            .map(|x| x.left())
            .sum(),
        max_delay: my_stages
            .iter()
            .map(|x| x.delay_days(today))
            .max()
            .unwrap_or(0),
    }
}

/// To'lov intizomi (TZ VIII.30, XVII.32-33).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PaymentDiscipline {
    pub stages: usize,
    pub closed: usize,
    pub overdue: usize,
    pub planned: f64,
    pub paid: f64,
    /// Muddati o'tgan qarz.
    pub debt: f64,
    /// Eng katta kechikish, kunlarda.
    pub max_delay: i64,
    /// O'rtacha kechikish yopilgan bosqichlar bo'yicha.
    pub avg_delay: f64,
    /// Yaqin o'ttiz kunda to'lanishi kerak.
    pub due_soon: f64,
}

/// To'lov jadvali bo'yicha intizomni hisoblaydi.
///
/// Kechikish faqat haqiqatda to'langan bosqichlardan o'lchanadi: hali
/// to'lanmagan bosqichda «qancha kechikdi» degan savol boshqa (u qarz).
pub fn payment_discipline(stages: &[PaymentStage], today: NaiveDate) -> PaymentDiscipline {
    let soon = today + chrono::Duration::days(30);
    let paid_delays: Vec<i64> = stages
        .iter()
        .filter_map(|x| x.paid_at.map(|p| (p - x.due).num_days().max(0)))
        .collect();

    PaymentDiscipline {
        stages: stages.len(),
        closed: stages.iter().filter(|x| x.closed()).count(),
        overdue: stages.iter().filter(|x| x.overdue(today)).count(),
        planned: stages.iter().map(|x| x.amount).sum(),
        paid: stages.iter().map(|x| x.paid).sum(),
        debt: stages
            .iter()
            .filter(|x| x.overdue(today))
            .map(|x| x.left())
            .sum(),
        max_delay: stages
            .iter()
            .map(|x| x.delay_days(today))
            .max()
            .unwrap_or(0),
        avg_delay: if paid_delays.is_empty() {
            0.0
        } else {
            paid_delays.iter().sum::<i64>() as f64 / paid_delays.len() as f64
        },
        due_soon: stages
            .iter()
            .filter(|x| !x.closed() && x.due >= today && x.due <= soon)
            .map(|x| x.left())
            .sum(),
    }
}

/// Buyurtmachining haftalik hisoboti (TZ VIII.33).
///
/// Hafta — bugundan orqaga yetti kun. Barcha sonlar bazadagi yozuvlardan,
/// shuning uchun hisobot obyekt ekranlaridagi sonlar bilan zid kelmaydi.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WeekReport {
    pub from: NaiveDate,
    pub to: NaiveDate,
    /// Hafta boshida va oxirida bajarilish, foizda.
    pub progress_start: f64,
    pub progress_end: f64,
    /// Haftada tugallangan ishlar.
    pub tasks_done: usize,
    /// Haftada boshlangan ishlar.
    pub tasks_started: usize,
    /// Jurnalga yozilgan kunlar.
    pub journal_days: usize,
    /// O'tkazilgan tekshiruvlar va ulardan salbiylari.
    pub inspections: usize,
    pub inspections_failed: usize,
    /// Yangi nomuvofiqliklar.
    pub issues_opened: usize,
    /// Imzolangan ijro hujjatlari.
    pub docs_signed: usize,
    /// Haftada to'langan summa.
    pub paid: f64,
    /// Qaror kutayotgan o'zgarishlar soni.
    pub changes_pending: usize,
    /// Qaror kutayotgan qabul hujjatlari.
    pub acceptances_pending: usize,
}

/// Haftalik hisobotni yig'adi.
#[allow(clippy::too_many_arguments)]
pub fn week_report(
    today: NaiveDate,
    tasks: &[Task],
    journal: &[JournalEntry],
    inspections: &[Inspection],
    issues: &[Issue],
    docs: &[ExecDoc],
    stages: &[PaymentStage],
    changes: &[ContractChange],
    acceptances: &[WorkAcceptance],
) -> WeekReport {
    let from = today - chrono::Duration::days(7);
    let in_week = |d: NaiveDate| d > from && d <= today;

    // Bajarilish: hajmga qarab vaznlangan o'rtacha — GPR ekranidagi bilan
    // bir xil qoida.
    let done_pct = |at: NaiveDate| -> f64 {
        let total: f64 = tasks.iter().map(|t| t.duration.max(1) as f64).sum();
        if total <= 0.0 {
            return 0.0;
        }
        tasks
            .iter()
            .map(|t| {
                let w = t.duration.max(1) as f64;
                // Hafta boshidagi holat: shu sanadan keyin tugagan ish hali
                // tugamagan hisoblanadi.
                let p = match t.fact_end {
                    Some(e) if e <= at => 100.0,
                    Some(_) => 0.0,
                    None => t.progress,
                };
                w * p
            })
            .sum::<f64>()
            / total
    };

    WeekReport {
        from,
        to: today,
        progress_start: done_pct(from),
        progress_end: done_pct(today),
        tasks_done: tasks
            .iter()
            .filter(|t| t.fact_end.is_some_and(in_week))
            .count(),
        tasks_started: tasks
            .iter()
            .filter(|t| t.fact_start.is_some_and(in_week))
            .count(),
        journal_days: journal.iter().filter(|j| in_week(j.date)).count(),
        inspections: inspections
            .iter()
            .filter(|x| x.done.is_some_and(in_week))
            .count(),
        inspections_failed: inspections
            .iter()
            .filter(|x| x.done.is_some_and(in_week) && x.result != InspectionResult::Pass)
            .count(),
        issues_opened: issues
            .iter()
            .filter(|i| {
                i.created_at
                    .get(..10)
                    .and_then(|d| d.parse::<NaiveDate>().ok())
                    .is_some_and(in_week)
            })
            .count(),
        docs_signed: docs
            .iter()
            .filter(|d| in_week(d.date) && d.status == ExecDocStatus::Signed)
            .count(),
        paid: stages
            .iter()
            .filter(|x| x.paid_at.is_some_and(in_week))
            .map(|x| x.paid)
            .sum(),
        changes_pending: changes.iter().filter(|x| x.pending()).count(),
        acceptances_pending: acceptances.iter().filter(|x| x.pending()).count(),
    }
}

// ================================================================ X. Xaridlar

/// Xarid rejasidagi bitta pozitsiya (TZ X.4-6).
#[derive(Debug, Clone, PartialEq)]
pub struct PurchasePlanLine {
    pub material_id: i64,
    /// Erkin qoldiq (rezervsiz).
    pub available: f64,
    /// Yo'lda: buyurtma qilingan, lekin hali kelmagan.
    pub ordered: f64,
    /// Minimal zaxira.
    pub min_stock: f64,
    /// Normativ bo'yicha yaqin ishlar uchun kerak bo'ladigan miqdor.
    pub needed_for_tasks: f64,
    /// Sotib olish kerak bo'lgan miqdor.
    pub to_buy: f64,
    /// Taxminiy summa katalog narxi bo'yicha.
    pub cost: f64,
    /// Qachongacha kerak — eng erta ishning boshlanishi.
    pub need_by: Option<NaiveDate>,
    /// Ariza allaqachon berilganmi.
    pub has_request: bool,
}

impl PurchasePlanLine {
    /// Muddat siqilgan: kerak bo'lgan sanagacha ikki haftadan kam qoldi.
    pub fn tight(&self, today: NaiveDate) -> bool {
        self.need_by.is_some_and(|d| (d - today).num_days() <= 14)
    }
}

/// Nima sotib olish kerakligini hisoblaydi (TZ X.4-6).
///
/// Ikki manba qo'shiladi: minimal zaxirani tiklash va yaqin ishlar uchun
/// normativ ehtiyoj. Yo'ldagi buyurtma va erkin qoldiq ayriladi — shuning
/// uchun ro'yxatda faqat haqiqatan yetishmayotgani qoladi.
#[allow(clippy::too_many_arguments)]
pub fn purchase_plan(
    materials: &[Material],
    stock: &[StockLine],
    purchases: &[Purchase],
    requests: &[Request],
    norms: &[MaterialNorm],
    tasks: &[Task],
    today: NaiveDate,
    horizon_days: i64,
) -> Vec<PurchasePlanLine> {
    let until = today + chrono::Duration::days(horizon_days);
    // Yaqin ufqdagi ishlar: hali tugamagan va shu davrda boshlanadigan.
    let soon: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.fact_end.is_none() && t.progress < 99.999)
        .filter(|t| t.plan_start <= until)
        .collect();

    let mut out: Vec<PurchasePlanLine> = Vec::new();
    for m in materials {
        let line = stock.iter().find(|l| l.material_id == m.id);
        let available = line.map(|l| l.available).unwrap_or(0.0);
        // Yo'ldagi miqdor: buyurtma qilingan, lekin yetkazilmagan qism.
        let ordered: f64 = purchases
            .iter()
            .filter(|p| !p.fully_delivered())
            .filter(|p| p.title.trim() == m.name.trim())
            .map(|p| (p.qty - p.delivered_qty).max(0.0))
            .sum();

        // Normativ ehtiyoj: har bir yaqin ish uchun qolgan hajm × norma.
        let mut needed = 0.0;
        let mut need_by: Option<NaiveDate> = None;
        for t in &soon {
            let Some(n) = norms
                .iter()
                .find(|n| n.material_id == m.id && n.task_id == t.id)
            else {
                continue;
            };
            let left = t.volume * (1.0 - t.progress / 100.0).clamp(0.0, 1.0);
            needed += left * n.per_unit;
            need_by = Some(match need_by {
                Some(d) if d < t.plan_start => d,
                _ => t.plan_start,
            });
        }

        // Sotib olish kerak: ehtiyoj va minimal zaxiradan kattasi.
        let target = needed.max(m.min_stock);
        let to_buy = target - available - ordered;
        if to_buy <= 0.001 {
            continue;
        }

        out.push(PurchasePlanLine {
            material_id: m.id,
            available,
            ordered,
            min_stock: m.min_stock,
            needed_for_tasks: needed,
            to_buy,
            cost: to_buy * m.price,
            need_by,
            has_request: requests
                .iter()
                .filter(|r| !matches!(r.status, RequestStatus::Closed | RequestStatus::Rejected))
                .any(|r| r.material_id == Some(m.id)),
        });
    }
    // Muddati yaqinlari oldinda, keyin summasi kattalari.
    out.sort_by(|a, b| {
        a.need_by
            .unwrap_or(NaiveDate::MAX)
            .cmp(&b.need_by.unwrap_or(NaiveDate::MAX))
            .then(b.cost.total_cmp(&a.cost))
    });
    out
}

/// Xaridlardagi risk belgisi (TZ X.41).
#[derive(Debug, Clone, PartialEq)]
pub enum ProcurementRisk {
    /// Bitta yetkazib beruvchining ulushi juda katta.
    SupplierShare { supplier: String, pct: f64 },
    /// Taklifsiz, to'g'ridan-to'g'ri xarid.
    NoQuotes { number: String, amount: f64 },
    /// Narx katalogdan sezilarli yuqori.
    HighPrice { number: String, over_pct: f64 },
    /// Shoshilinch xaridlar ulushi katta.
    TooManyUrgent { count: usize, pct: f64 },
}

/// Bitta yetkazib beruvchining shundan katta ulushi — savol tug'diradi.
pub const SUPPLIER_SHARE_LIMIT: f64 = 45.0;
/// Katalog narxidan shuncha foiz yuqorisi tekshirishga arziydi.
pub const PRICE_OVER_LIMIT: f64 = 20.0;
/// Shoshilinch xaridlarning maqbul ulushi.
pub const URGENT_SHARE_LIMIT: f64 = 25.0;

/// Xarid jarayonidagi shubhali joylarni topadi (TZ X.41).
///
/// Bu **ayblov emas**: har bir belgi — tekshirib ko'rish uchun sabab.
/// Shuning uchun har birida son va dalil bor, xulosa esa odamniki.
pub fn procurement_risks(
    purchases: &[Purchase],
    quotes: &[Quote],
    materials: &[Material],
) -> Vec<ProcurementRisk> {
    let mut out = Vec::new();
    let total: f64 = purchases.iter().map(|p| p.amount()).sum();
    if total <= 0.0 {
        return out;
    }

    // 1. Yetkazib beruvchilar ulushi.
    let mut by_supplier: Vec<(String, f64)> = Vec::new();
    for p in purchases {
        let key = p.supplier.trim().to_string();
        if key.is_empty() {
            continue;
        }
        match by_supplier.iter_mut().find(|(s, _)| *s == key) {
            Some((_, sum)) => *sum += p.amount(),
            None => by_supplier.push((key, p.amount())),
        }
    }
    for (supplier, sum) in &by_supplier {
        let pct = sum / total * 100.0;
        if pct > SUPPLIER_SHARE_LIMIT {
            out.push(ProcurementRisk::SupplierShare {
                supplier: supplier.clone(),
                pct,
            });
        }
    }

    // 2. Taklifsiz xaridlar: arizasi bo'lsa ham taklif solishtirilmagan.
    for p in purchases {
        let has_quote = p
            .request_id
            .is_some_and(|rid| quotes.iter().any(|q| q.request_id == Some(rid)));
        if !has_quote && p.amount() > 0.0 {
            out.push(ProcurementRisk::NoQuotes {
                number: p.number.clone(),
                amount: p.amount(),
            });
        }
    }

    // 3. Katalog narxidan yuqori.
    for p in purchases {
        let Some(m) = materials.iter().find(|m| m.name.trim() == p.title.trim()) else {
            continue;
        };
        if m.price <= 0.0 {
            continue;
        }
        let over = (p.price - m.price) / m.price * 100.0;
        if over > PRICE_OVER_LIMIT {
            out.push(ProcurementRisk::HighPrice {
                number: p.number.clone(),
                over_pct: over,
            });
        }
    }

    // 4. Shoshilinch xaridlar ulushi.
    let urgent = purchases.iter().filter(|p| p.urgent).count();
    if !purchases.is_empty() {
        let pct = urgent as f64 / purchases.len() as f64 * 100.0;
        if pct > URGENT_SHARE_LIMIT {
            out.push(ProcurementRisk::TooManyUrgent { count: urgent, pct });
        }
    }
    out
}

/// Xaridchi bo'yicha yakun (TZ X.46).
#[derive(Debug, Clone, PartialEq)]
pub struct BuyerStats {
    pub buyer: String,
    pub purchases: usize,
    pub amount: f64,
    /// Muddatida yetkazilganlar ulushi, foizda.
    pub on_time_pct: f64,
    /// Shoshilinch xaridlar soni.
    pub urgent: usize,
    /// Taklif solishtirilgan xaridlar ulushi.
    pub with_quotes_pct: f64,
}

/// Xaridchilar kesimida yakun.
pub fn buyer_stats(purchases: &[Purchase], quotes: &[Quote], today: NaiveDate) -> Vec<BuyerStats> {
    let mut names: Vec<String> = purchases
        .iter()
        .map(|p| p.buyer.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    names.sort();
    names.dedup();

    let mut out: Vec<BuyerStats> = names
        .into_iter()
        .map(|buyer| {
            let mine: Vec<&Purchase> = purchases
                .iter()
                .filter(|p| p.buyer.trim() == buyer)
                .collect();
            // Muddatida: yetkazilgan va yetkazish sanasi o'tmagan.
            let closed: Vec<&&Purchase> = mine.iter().filter(|p| p.fully_delivered()).collect();
            let on_time = closed
                .iter()
                .filter(|p| p.delivery_date >= today || p.fully_delivered())
                .count();
            let with_quotes = mine
                .iter()
                .filter(|p| {
                    p.request_id
                        .is_some_and(|rid| quotes.iter().any(|q| q.request_id == Some(rid)))
                })
                .count();
            BuyerStats {
                purchases: mine.len(),
                amount: mine.iter().map(|p| p.amount()).sum(),
                on_time_pct: if closed.is_empty() {
                    0.0
                } else {
                    on_time as f64 / closed.len() as f64 * 100.0
                },
                urgent: mine.iter().filter(|p| p.urgent).count(),
                with_quotes_pct: if mine.is_empty() {
                    0.0
                } else {
                    with_quotes as f64 / mine.len() as f64 * 100.0
                },
                buyer,
            }
        })
        .collect();
    out.sort_by(|a, b| b.amount.total_cmp(&a.amount));
    out
}

// ================= XVII.14, 32-34. Moliyaviy prognoz =================

/// Obyekt bo'yicha moliyaviy prognoz (TZ XVII.13-14, 32-34).
///
/// Prognoz bitta oddiy taxminga tayanadi: bugungi bajarilish darajasidagi
/// tannarx oxirigacha shu tezlikda o'sadi. Boshqa taxmin kiritilmaydi —
/// aks holda son «qayerdan chiqdi» degan savolga javob bo'lmaydi.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FinanceForecast {
    /// Shartnoma summasi va tasdiqlangan o'zgarishlar bilan amaldagi summa.
    pub contract: f64,
    /// Bugungi kunga bajarilgan ish qiymati.
    pub earned: f64,
    /// Bugungi kunga to'plangan tannarx.
    pub cost_now: f64,
    /// Yakuniy tannarx prognozi.
    pub cost_forecast: f64,
    /// Yakuniy foyda prognozi.
    pub profit_forecast: f64,
    /// Foyda ulushi, foizda.
    pub margin_pct: f64,
    /// Buyurtmachidan olinishi kerak bo'lgan qarz (debitorlik).
    pub receivable: f64,
    /// Shundan muddati o'tgani.
    pub overdue: f64,
    /// Yaqin 90 kunda kutilayotgan tushum.
    pub revenue_90: f64,
    /// Bajarilish foizi — prognoz shundan chiqarilgan.
    pub progress_pct: f64,
}

/// Moliyaviy prognozni yig'adi.
pub fn finance_forecast(
    contract: f64,
    changes: &[ContractChange],
    stages: &[PaymentStage],
    cost_now: f64,
    progress_pct: f64,
    today: NaiveDate,
) -> FinanceForecast {
    let approved: f64 = changes
        .iter()
        .filter(|c| c.counts())
        .map(|c| c.amount)
        .sum();
    let current = contract + approved;
    let p = (progress_pct / 100.0).clamp(0.0, 1.0);

    // Yakuniy tannarx: bugungi tannarxni bajarilish ulushiga bo'lamiz.
    // Bajarilish juda kichik bo'lsa prognoz ishonchsiz — nol qoldiramiz.
    let cost_forecast = if p >= 0.05 { cost_now / p } else { 0.0 };
    let soon = today + chrono::Duration::days(90);

    FinanceForecast {
        contract: current,
        earned: current * p,
        cost_now,
        cost_forecast,
        profit_forecast: if cost_forecast > 0.0 {
            current - cost_forecast
        } else {
            0.0
        },
        margin_pct: if current > 0.0 && cost_forecast > 0.0 {
            (current - cost_forecast) / current * 100.0
        } else {
            0.0
        },
        receivable: stages.iter().map(|s| s.left()).sum(),
        overdue: stages
            .iter()
            .filter(|s| s.overdue(today))
            .map(|s| s.left())
            .sum(),
        revenue_90: stages
            .iter()
            .filter(|s| !s.closed() && s.due >= today && s.due <= soon)
            .map(|s| s.left())
            .sum(),
        progress_pct,
    }
}

// ================= XVII.26-27. Unumdorlik va benchmarking =================

/// Bitta ish bo'yicha unumdorlik (TZ XVII.26).
#[derive(Debug, Clone, PartialEq)]
pub struct Productivity {
    pub task_id: i64,
    /// Bajarilgan hajm.
    pub done_volume: f64,
    pub unit: String,
    /// Shu ishga sarflangan soat.
    pub hours: f64,
    /// Bir birlik uchun soat.
    pub hours_per_unit: f64,
    /// Bir birlik uchun ish haqi.
    pub cost_per_unit: f64,
}

/// Ishlar bo'yicha unumdorlikni hisoblaydi.
///
/// Faqat hajmi va sarflangan soati bor ishlar kiradi: qolganida
/// «bir birlik qancha turadi» degan savolga javob yo'q.
pub fn productivity(
    tasks: &[Task],
    timesheet: &[TimesheetEntry],
    workers: &[Worker],
) -> Vec<Productivity> {
    let mut out = Vec::new();
    for t in tasks {
        if t.volume <= 0.0 || t.progress <= 0.0 {
            continue;
        }
        let rows: Vec<&TimesheetEntry> = timesheet
            .iter()
            .filter(|e| e.task_id == Some(t.id))
            .collect();
        let hours: f64 = rows.iter().map(|e| e.hours).sum();
        if hours <= 0.0 {
            continue;
        }
        let cost: f64 = rows
            .iter()
            .map(|e| {
                let rate = workers
                    .iter()
                    .find(|w| w.id == e.worker_id)
                    .map(|w| w.hourly_rate)
                    .unwrap_or(0.0);
                e.hours * rate * e.shift.rate()
            })
            .sum();
        let done = t.volume * (t.progress / 100.0);
        if done <= 0.0 {
            continue;
        }
        out.push(Productivity {
            task_id: t.id,
            done_volume: done,
            unit: t.unit.clone(),
            hours,
            hours_per_unit: hours / done,
            cost_per_unit: cost / done,
        });
    }
    out.sort_by(|a, b| b.hours_per_unit.total_cmp(&a.hours_per_unit));
    out
}

// ================= XVII.36, 43. Ssenariy =================

/// «Nima bo'ladi, agar?» ssenariysi (TZ XVII.36, 43).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scenario {
    /// Muddat necha kunga suriladi.
    pub delay_days: i64,
    /// Material narxi necha foizga o'zgaradi.
    pub price_pct: f64,
    /// Ish haqi necha foizga o'zgaradi.
    pub wage_pct: f64,
}

/// Ssenariy natijasi.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScenarioResult {
    /// Tugash sanasi ssenariysiz va ssenariy bilan.
    pub finish: Option<NaiveDate>,
    pub finish_after: Option<NaiveDate>,
    /// Yakuniy tannarx ssenariysiz va ssenariy bilan.
    pub cost: f64,
    pub cost_after: f64,
    /// Foyda ssenariysiz va ssenariy bilan.
    pub profit: f64,
    pub profit_after: f64,
    /// Shartnoma muddatidan chiqib ketadimi.
    pub over_deadline: bool,
}

/// Ssenariyni hisoblaydi (TZ XVII.36).
///
/// Bu bashorat emas, **arifmetika**: berilgan taxminlar bugungi sonlarga
/// qo'llanadi. Shuning uchun natija tushunarli va tekshirib bo'ladigan.
pub fn scenario(
    base: &FinanceForecast,
    costs: &[TaskCost],
    forecast_end: Option<NaiveDate>,
    planned_end: NaiveDate,
    s: &Scenario,
) -> ScenarioResult {
    // Material va ish haqi ulushlari bugungi tannarx tarkibidan olinadi va
    // yakuniy tannarxga ko'chiriladi.
    let total: f64 = costs.iter().map(|c| c.total).sum();
    let share = |part: f64| if total > 0.0 { part / total } else { 0.0 };
    let mat_share = share(costs.iter().map(|c| c.material).sum());
    let wage_share = share(costs.iter().map(|c| c.labour).sum());

    let delta = base.cost_forecast * mat_share * (s.price_pct / 100.0)
        + base.cost_forecast * wage_share * (s.wage_pct / 100.0);
    let cost_after = base.cost_forecast + delta;
    let finish_after = forecast_end.map(|d| d + chrono::Duration::days(s.delay_days));

    ScenarioResult {
        finish: forecast_end,
        finish_after,
        cost: base.cost_forecast,
        cost_after,
        profit: base.contract - base.cost_forecast,
        profit_after: base.contract - cost_after,
        over_deadline: finish_after.is_some_and(|d| d > planned_end),
    }
}

// ================= XVII.41, 44. Yo'qotish va imkoniyatlar =================

/// Topilgan yo'qotish yoki imkoniyat (TZ XVII.41, 44).
#[derive(Debug, Clone, PartialEq)]
pub struct Opportunity {
    pub code: &'static str,
    /// Musbat — tejash imkoniyati, manfiy — yo'qotish.
    pub amount: f64,
    pub title: String,
    pub detail: String,
    pub screen: crate::app::Screen,
}

/// Ko'zga tashlanmaydigan yo'qotishlar va tejash imkoniyatlari.
///
/// Har biri **pulda** o'lchanadi: «diqqat qiling» degan xabar emas, aniq
/// summa bo'lsa, unga qarab qaror qabul qilinadi.
pub fn opportunities(
    stock: &[StockLine],
    materials: &[Material],
    machines: &[MachineLine],
    quotes: &[Quote],
    purchases: &[Purchase],
    usage: &[ConsumptionLine],
) -> Vec<Opportunity> {
    let mut out = Vec::new();

    // 1. Harakatsiz zaxira: pul omborda turibdi.
    let idle: f64 = stock
        .iter()
        .filter(|l| l.balance > 0.0)
        .filter(|l| {
            materials
                .iter()
                .find(|m| m.id == l.material_id)
                .is_some_and(|m| l.balance > m.min_stock * 3.0 && m.min_stock > 0.0)
        })
        .map(|l| l.value)
        .sum();
    if idle > 0.0 {
        out.push(Opportunity {
            code: "OP-1",
            amount: idle,
            title: crate::i18n::t("op_idle_stock").to_string(),
            detail: crate::i18n::t("op_idle_stock_hint").to_string(),
            screen: crate::app::Screen::Warehouse,
        });
    }

    // 2. Bo'sh turgan texnika: ijarasi to'lanadi, ishlamaydi.
    let idle_machines: Vec<&MachineLine> = machines
        .iter()
        .filter(|m| m.work_days == 0 && m.cost > 0.0)
        .collect();
    if !idle_machines.is_empty() {
        out.push(Opportunity {
            code: "OP-2",
            amount: idle_machines.iter().map(|m| m.cost).sum(),
            title: format!(
                "{} {}",
                idle_machines.len(),
                crate::i18n::t("op_idle_machines")
            ),
            detail: crate::i18n::t("op_idle_machines_hint").to_string(),
            screen: crate::app::Screen::Machines,
        });
    }

    // 3. Normadan ortiq sarf: material yo'qolgan yoki isrof bo'lgan.
    let over: f64 = usage
        .iter()
        .filter(|u| u.over)
        .map(|u| {
            let price = materials
                .iter()
                .find(|m| m.id == u.material_id)
                .map(|m| m.price)
                .unwrap_or(0.0);
            let _ = price;
            u.over_cost
        })
        .sum();
    if over > 0.0 {
        out.push(Opportunity {
            code: "OP-3",
            amount: -over,
            title: crate::i18n::t("op_over_usage").to_string(),
            detail: crate::i18n::t("op_over_usage_hint").to_string(),
            screen: crate::app::Screen::Materials,
        });
    }

    // 4. Arzonroq taklif tanlanmagan: farq — yo'qotilgan tejash.
    let mut missed = 0.0;
    for p in purchases {
        let Some(rid) = p.request_id else { continue };
        let best = quotes
            .iter()
            .filter(|q| q.request_id == Some(rid))
            .map(|q| q.price)
            .fold(f64::INFINITY, f64::min);
        if best.is_finite() && p.price > best {
            missed += (p.price - best) * p.qty;
        }
    }
    if missed > 0.0 {
        out.push(Opportunity {
            code: "OP-4",
            amount: -missed,
            title: crate::i18n::t("op_missed_quote").to_string(),
            detail: crate::i18n::t("op_missed_quote_hint").to_string(),
            screen: crate::app::Screen::Purchases,
        });
    }

    out.sort_by(|a, b| b.amount.abs().total_cmp(&a.amount.abs()));
    out
}

// ================================================================ XI.32-34. Asboblar

/// Bitta asbob bo'yicha holat (TZ XI.34).
#[derive(Debug, Clone, PartialEq)]
pub struct ToolStatus {
    pub tool_id: i64,
    /// Hozir kimda. Bo'sh — omborda.
    pub holder: Option<i64>,
    pub issued: Option<NaiveDate>,
    pub due: Option<NaiveDate>,
    /// Necha kundan beri ishchida.
    pub days: i64,
    /// Qaytarish muddati o'tgan.
    pub overdue: bool,
    /// Necha marta berilgan.
    pub issues: usize,
}

/// Asboblarning hozirgi holati.
///
/// Asbob sarflanmaydi — u qaytariladi, shuning uchun asosiy savol
/// «qoldiq qancha» emas, «kimda va qachondan beri».
pub fn tool_status(tools: &[Tool], issues: &[ToolIssue], today: NaiveDate) -> Vec<ToolStatus> {
    tools
        .iter()
        .map(|t| {
            let mine: Vec<&ToolIssue> = issues.iter().filter(|x| x.tool_id == t.id).collect();
            // Ochiq berish — eng oxirgisi; ikkitasi bo'lsa oxirgisi haqiqiy.
            let open = mine
                .iter()
                .filter(|x| x.open())
                .max_by_key(|x| x.issued)
                .copied();
            ToolStatus {
                tool_id: t.id,
                holder: open.map(|x| x.worker_id),
                issued: open.map(|x| x.issued),
                due: open.and_then(|x| x.due),
                days: open.map(|x| x.days(today)).unwrap_or(0),
                overdue: open.is_some_and(|x| x.overdue(today)),
                issues: mine.len(),
            }
        })
        .collect()
}

/// Asboblar bo'yicha yakun.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ToolSummary {
    pub total: usize,
    /// Omborda turganlari.
    pub in_store: usize,
    /// Ishchilarda.
    pub issued: usize,
    /// Qaytarish muddati o'tganlari.
    pub overdue: usize,
    /// Ta'mirda yoki hisobdan chiqarilgan.
    pub out_of_service: usize,
    /// Tekshiruv muddati o'tganlari.
    pub check_overdue: usize,
    /// Ishchilardagi asboblarning qiymati.
    pub issued_value: f64,
}

/// Asboblar bo'yicha yakunni yig'adi.
pub fn tool_summary(tools: &[Tool], status: &[ToolStatus], today: NaiveDate) -> ToolSummary {
    let held = |t: &Tool| {
        status
            .iter()
            .find(|s| s.tool_id == t.id)
            .is_some_and(|s| s.holder.is_some())
    };
    ToolSummary {
        total: tools.len(),
        in_store: tools.iter().filter(|t| !held(t)).count(),
        issued: tools.iter().filter(|t| held(t)).count(),
        overdue: status.iter().filter(|s| s.overdue).count(),
        out_of_service: tools.iter().filter(|t| t.out_of_service()).count(),
        check_overdue: tools.iter().filter(|t| t.check_overdue(today)).count(),
        issued_value: tools.iter().filter(|t| held(t)).map(|t| t.price).sum(),
    }
}

// ================================================================ XI.26. Kamomad

/// Inventarizatsiya farqi bo'yicha xulosa (TZ XI.26).
#[derive(Debug, Clone, PartialEq)]
pub struct ShortageLine {
    pub material_id: i64,
    /// Necha marta inventarizatsiyada farq chiqqan.
    pub times: usize,
    /// Jami kamomad (manfiy farqlar yig'indisi), miqdorda.
    pub shortage: f64,
    /// Jami ortiqcha (musbat farqlar).
    pub surplus: f64,
    /// Kamomadning puldagi qiymati.
    pub cost: f64,
    /// Kamomad hisob bo'yicha qoldiqning necha foizi.
    pub pct: f64,
}

/// Kamomad qaysi materiallarda takrorlanayotganini ko'rsatadi.
///
/// Bitta farq — xato bo'lishi mumkin; **takrorlangan** farq esa tizimli
/// sabab: o'lchov, saqlash yoki hisob tartibida.
pub fn shortages(
    inventories: &[Inventory],
    lines: &[InventoryLine],
    materials: &[Material],
) -> Vec<ShortageLine> {
    // Faqat yopilgan inventarizatsiyalar: ochig'i hali to'ldirilmoqda.
    let closed: Vec<i64> = inventories
        .iter()
        .filter(|i| i.closed)
        .map(|i| i.id)
        .collect();

    let mut out: Vec<ShortageLine> = Vec::new();
    for l in lines.iter().filter(|l| closed.contains(&l.inventory_id)) {
        let diff = l.fact - l.book;
        if diff.abs() < 0.0001 {
            continue;
        }
        let price = materials
            .iter()
            .find(|m| m.id == l.material_id)
            .map(|m| m.price)
            .unwrap_or(0.0);
        match out.iter_mut().find(|s| s.material_id == l.material_id) {
            Some(s) => {
                s.times += 1;
                if diff < 0.0 {
                    s.shortage += -diff;
                    s.cost += -diff * price;
                } else {
                    s.surplus += diff;
                }
                s.pct += if l.book > 0.0 {
                    (-diff).max(0.0) / l.book * 100.0
                } else {
                    0.0
                };
            }
            None => out.push(ShortageLine {
                material_id: l.material_id,
                times: 1,
                shortage: (-diff).max(0.0),
                surplus: diff.max(0.0),
                cost: (-diff).max(0.0) * price,
                pct: if l.book > 0.0 {
                    (-diff).max(0.0) / l.book * 100.0
                } else {
                    0.0
                },
            }),
        }
    }
    // Puldagi zarari kattalari oldinda.
    out.sort_by(|a, b| b.cost.total_cmp(&a.cost));
    out
}

// ================================================================ XI.42. Qayta taqsimlash

/// Obyektlar orasida material ko'chirish taklifi (TZ XI.42).
#[derive(Debug, Clone, PartialEq)]
pub struct Redistribution {
    /// Qayerdan — ortiqcha turgan obyekt.
    pub from_project: i64,
    /// Qayerga — yetishmayotgan obyekt.
    pub to_project: i64,
    pub material_name: String,
    pub unit: String,
    /// Ko'chirish mumkin bo'lgan miqdor.
    pub qty: f64,
    /// Shuncha pul sotib olishga sarflanmaydi.
    pub saving: f64,
}

/// Bir obyektda ortiqcha, boshqasida yetishmayotgan materiallarni topadi.
///
/// Solishtirish **nom bo'yicha**: obyektlarning kataloglari alohida va
/// bir xil material turli kodlar bilan yozilgan bo'lishi mumkin.
pub fn redistribution(
    surplus: &[(i64, Vec<StockLine>, Vec<Material>)],
    need: &[(i64, Vec<PurchasePlanLine>, Vec<Material>)],
) -> Vec<Redistribution> {
    let mut out = Vec::new();
    for (to_pid, plan, need_mats) in need {
        for line in plan {
            let Some(m) = need_mats.iter().find(|m| m.id == line.material_id) else {
                continue;
            };
            let name = m.name.trim().to_lowercase();
            for (from_pid, stock, mats) in surplus {
                if from_pid == to_pid {
                    continue;
                }
                let Some(src) = mats.iter().find(|x| x.name.trim().to_lowercase() == name) else {
                    continue;
                };
                let Some(sl) = stock.iter().find(|l| l.material_id == src.id) else {
                    continue;
                };
                // Ortiqcha deb faqat minimal zaxiradan yuqorisi hisoblanadi:
                // boshqa obyektni zaxirasiz qoldirib bo'lmaydi.
                let free = sl.available - src.min_stock;
                if free <= 0.0 {
                    continue;
                }
                let qty = free.min(line.to_buy);
                if qty <= 0.0001 {
                    continue;
                }
                out.push(Redistribution {
                    from_project: *from_pid,
                    to_project: *to_pid,
                    material_name: m.name.clone(),
                    unit: m.unit.clone(),
                    qty,
                    saving: qty * m.price,
                });
            }
        }
    }
    out.sort_by(|a, b| b.saving.total_cmp(&a.saving));
    out
}

// ================================================================ IX. Ariza tekshiruvi

/// Arizadagi kamchilik yoki savol (TZ IX.11-14, 38).
#[derive(Debug, Clone, PartialEq)]
pub enum RequestIssue {
    /// Xuddi shu material bo'yicha ochiq ariza allaqachon bor.
    Duplicate { number: String },
    /// Material smetada uchramaydi.
    NotInEstimate,
    /// Bo'lim byudjeti oshib ketadi.
    OverBudget { over: f64 },
    /// Arzonroq analog bor.
    Cheaper { name: String, saving: f64 },
    /// Ish ko'rsatilmagan: kechikish kimga ta'sir qilishi ko'rinmaydi.
    NoTask,
    /// Material taqiqlangan.
    Banned { reason: String },
    /// Loyiha spetsifikatsiyasiga havola yo'q — material loyihada
    /// ko'zda tutilganini tekshirib bo'lmaydi (TZ IX.13).
    NoSpecRef,
    /// So'ralgan miqdor ish uchun normadan sezilarli ko'p (TZ IX.13).
    OverNorm { need: f64, by_norm: f64 },
    /// Xodimga arizada kasb ko'rsatilmagan (TZ IX.27).
    NoProfession,
    /// Xodim ehtiyoji hisobda ko'rinmaydi: brigada yetarli (TZ IX.27).
    StaffEnough { have: usize, need: usize },
}

/// Arizani tasdiqlashdan oldin tekshiradi (TZ IX.11-14).
///
/// Har bir belgi — **savol**, taqiq emas: ariza baribir tasdiqlanishi
/// mumkin, lekin qaror ko'zi ochiq qabul qilinadi.
#[allow(clippy::too_many_arguments)]
pub fn request_issues(
    r: &Request,
    others: &[Request],
    materials: &[Material],
    alts: &[MaterialAlt],
    estimate_items: &[EstimateItem],
    budgets: &[PurchaseBudget],
    purchases: &[Purchase],
    norms: &[MaterialNorm],
    tasks: &[Task],
    staff: Option<&StaffForecast>,
) -> Vec<RequestIssue> {
    let mut out = Vec::new();

    // 1. Dublikat: shu material bo'yicha boshqa ochiq ariza.
    if let Some(mid) = r.material_id {
        if let Some(dup) = others
            .iter()
            .filter(|x| x.id != r.id)
            .filter(|x| x.material_id == Some(mid))
            .find(|x| !matches!(x.status, RequestStatus::Closed | RequestStatus::Rejected))
        {
            out.push(RequestIssue::Duplicate {
                number: dup.number.clone(),
            });
        }
    }

    let material = r
        .material_id
        .and_then(|id| materials.iter().find(|m| m.id == id));

    // 2. Taqiqlangan material.
    if let Some(m) = material {
        if m.banned {
            out.push(RequestIssue::Banned {
                reason: m.ban_reason.clone(),
            });
        }

        // 3. Smetada bormi: kod bo'yicha, bo'lmasa nom bo'yicha.
        if !estimate_items.is_empty() {
            let by_code = !m.estimate_code.trim().is_empty()
                && estimate_items
                    .iter()
                    .any(|i| i.code.trim() == m.estimate_code.trim());
            let by_name = estimate_items
                .iter()
                .any(|i| i.name.trim().eq_ignore_ascii_case(m.name.trim()));
            if !by_code && !by_name {
                out.push(RequestIssue::NotInEstimate);
            }
        }

        // 4. Arzonroq analog bor.
        let mut best: Option<(&Material, f64)> = None;
        for a in alts.iter().filter(|a| a.material_id == m.id) {
            let Some(other) = materials.iter().find(|x| x.id == a.alt_id) else {
                continue;
            };
            if other.banned || other.price <= 0.0 || other.price >= m.price {
                continue;
            }
            let saving = (m.price - other.price) * r.qty;
            if best.map(|(_, s)| saving > s).unwrap_or(true) {
                best = Some((other, saving));
            }
        }
        if let Some((other, saving)) = best {
            out.push(RequestIssue::Cheaper {
                name: other.name.clone(),
                saving,
            });
        }

        // 5. Bo'lim byudjeti: ariza summasi qoldiqdan oshadimi.
        if let Some(b) = budgets.iter().find(|b| b.section == m.section) {
            let spent: f64 = purchases
                .iter()
                .filter(|p| p.section == m.section)
                .map(|p| p.amount())
                .sum();
            let left = b.planned - spent;
            let need = r.qty * m.price;
            if need > left {
                out.push(RequestIssue::OverBudget { over: need - left });
            }
        }
    }

    // 6. Ish ko'rsatilmagan.
    if r.task_id.is_none() && r.kind == RequestKind::Material {
        out.push(RequestIssue::NoTask);
    }

    // 7. Loyihaga muvofiqlik (TZ IX.13).
    if let Some(m) = material {
        if m.spec_ref.trim().is_empty() {
            out.push(RequestIssue::NoSpecRef);
        }
        // Norma bo'yicha kerak bo'lgan miqdor bilan solishtirish: ish va
        // norma ma'lum bo'lsagina. Norma yo'q joyda «ko'p so'ragan» deb
        // aytish mumkin emas.
        if let (Some(tid), Some(task)) = (
            r.task_id,
            r.task_id.and_then(|id| tasks.iter().find(|t| t.id == id)),
        ) {
            if let Some(n) = norms
                .iter()
                .find(|n| n.task_id == tid && n.material_id == m.id)
            {
                let by_norm = n.per_unit * task.volume * (1.0 + n.tolerance / 100.0);
                if by_norm > 0.0 && r.qty > by_norm {
                    out.push(RequestIssue::OverNorm {
                        need: r.qty,
                        by_norm,
                    });
                }
            }
        }
    }

    // 8. Xodimga ariza (TZ IX.27).
    if r.kind == RequestKind::Labor {
        // Kasb nomi arizaning sarlavhasida bo'lishi kerak: «odam kerak»
        // degan ariza bo'yicha hech kimni topib bo'lmaydi.
        if r.title.trim().chars().count() < 4 {
            out.push(RequestIssue::NoProfession);
        }
        // Ehtiyoj hisobi kishilar yetarli deb ko'rsatsa — savol beriladi.
        if let Some(f) = staff {
            if f.gap <= 0 {
                out.push(RequestIssue::StaffEnough {
                    have: f.have,
                    need: f.needed_workers,
                });
            }
        }
    }

    out
}

// ================================================================ XIII.20-21. Tabel anomaliyalari

/// Tabeldagi g'ayrioddiy holat (TZ XIII.20-21).
#[derive(Debug, Clone, PartialEq)]
pub enum TimesheetAnomaly {
    /// Bir kunda haddan tashqari ko'p soat.
    TooManyHours {
        worker_id: i64,
        day: NaiveDate,
        hours: f64,
    },
    /// Dam olish kunida ish, naryadsiz.
    WeekendWork { worker_id: i64, day: NaiveDate },
    /// Dam olishsiz ketma-ket ishlangan kunlar.
    NoRest { worker_id: i64, days: usize },
    /// Butun davr davomida soat bir xil — tabel qo'lda «to'ldirilgan» bo'lishi mumkin.
    Identical {
        worker_id: i64,
        hours: f64,
        days: usize,
    },
}

/// Kunda shundan ko'p soat — tekshirishga arziydi.
pub const MAX_DAY_HOURS: f64 = 12.0;
/// Dam olishsiz shuncha kun ketma-ket ishlash normadan chetga chiqish.
pub const MAX_STREAK: usize = 7;
/// Bir xil soat shuncha kundan ko'p takrorlansa — savol.
pub const IDENTICAL_LIMIT: usize = 20;

/// Tabeldagi g'ayrioddiy holatlarni topadi.
///
/// Bu **ayblov emas**: har bir belgi tekshirishga sabab. Ko'pincha ular
/// haqiqiy — masalan avariya kuni yoki topshirish oldidan.
pub fn timesheet_anomalies(entries: &[TimesheetEntry]) -> Vec<TimesheetAnomaly> {
    let mut out = Vec::new();

    // Ishchilar bo'yicha guruhlash.
    let mut ids: Vec<i64> = entries.iter().map(|e| e.worker_id).collect();
    ids.sort_unstable();
    ids.dedup();

    for id in ids {
        let mut mine: Vec<&TimesheetEntry> = entries
            .iter()
            .filter(|e| e.worker_id == id && e.hours > 0.0)
            .collect();
        mine.sort_by_key(|e| e.date);

        // 1. Kunlik soat chegarasi.
        for e in &mine {
            if e.hours > MAX_DAY_HOURS {
                out.push(TimesheetAnomaly::TooManyHours {
                    worker_id: id,
                    day: e.date,
                    hours: e.hours,
                });
            }
            // 2. Yakshanba ishi.
            if e.date.weekday() == chrono::Weekday::Sun {
                out.push(TimesheetAnomaly::WeekendWork {
                    worker_id: id,
                    day: e.date,
                });
            }
        }

        // 3. Dam olishsiz ketma-ket kunlar.
        let mut streak = 0usize;
        let mut best = 0usize;
        let mut prev: Option<NaiveDate> = None;
        for e in &mine {
            streak = match prev {
                Some(p) if (e.date - p).num_days() == 1 => streak + 1,
                _ => 1,
            };
            best = best.max(streak);
            prev = Some(e.date);
        }
        if best > MAX_STREAK {
            out.push(TimesheetAnomaly::NoRest {
                worker_id: id,
                days: best,
            });
        }

        // 4. Bir xil soat: tabel «bir xil qilib» to'ldirilgan bo'lishi mumkin.
        if mine.len() >= IDENTICAL_LIMIT {
            let first = mine[0].hours;
            if mine.iter().all(|e| (e.hours - first).abs() < 0.001) {
                out.push(TimesheetAnomaly::Identical {
                    worker_id: id,
                    hours: first,
                    days: mine.len(),
                });
            }
        }
    }
    out
}

// ================================================================ XIII.26-27. Xodim ehtiyoji

/// Yaqin davr uchun xodim ehtiyoji (TZ XIII.26-27).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StaffForecast {
    /// Hozir faol ishchilar soni.
    pub have: usize,
    /// Yaqin davrda kerak bo'ladigan soat.
    pub needed_hours: f64,
    /// Shu soatni bajarish uchun kerakli ishchi soni.
    pub needed_workers: usize,
    /// Yetishmayapti (manfiy — ortiqcha).
    pub gap: i64,
    /// Hisob qaysi ishlardan chiqqani.
    pub tasks: usize,
}

/// Yaqin ishlar uchun necha ishchi kerakligini baholaydi.
///
/// Hisob bugungi unumdorlikka tayanadi: bajarilgan hajm qancha soatga
/// tushgan bo'lsa, qolgan hajm ham shuncha talab qiladi. Unumdorligi
/// noma'lum ish hisobga kirmaydi — taxmin qilib bo'lmaydi.
pub fn staff_forecast(
    tasks: &[Task],
    productivity: &[Productivity],
    workers: &[Worker],
    today: NaiveDate,
    horizon_days: i64,
) -> StaffForecast {
    let until = today + chrono::Duration::days(horizon_days);
    let mut hours = 0.0;
    let mut counted = 0usize;

    for t in tasks {
        if t.fact_end.is_some() || t.progress >= 99.999 {
            continue;
        }
        // Faqat shu davrda boradigan ishlar.
        if t.plan_start > until {
            continue;
        }
        let Some(p) = productivity.iter().find(|p| p.task_id == t.id) else {
            continue;
        };
        let left = t.volume * (1.0 - t.progress / 100.0).clamp(0.0, 1.0);
        hours += left * p.hours_per_unit;
        counted += 1;
    }

    // Davrdagi ish kunlari: yakshanbadan boshqasi.
    let mut work_days = 0usize;
    let mut d = today;
    while d <= until {
        if d.weekday() != chrono::Weekday::Sun {
            work_days += 1;
        }
        d += chrono::Duration::days(1);
    }
    let capacity = work_days as f64 * SHIFT_HOURS;
    let needed_workers = if capacity > 0.0 {
        (hours / capacity).ceil().max(0.0) as usize
    } else {
        0
    };
    let have = workers.iter().filter(|w| w.active).count();

    StaffForecast {
        have,
        needed_hours: hours,
        needed_workers,
        gap: needed_workers as i64 - have as i64,
        tasks: counted,
    }
}

// ================================================================ XIV.28-29. Pudratchi reytingi

/// Pudratchi yoki mas'ul bo'yicha sifat yakuni (TZ XIV.28-29).
#[derive(Debug, Clone, PartialEq)]
pub struct ContractorQuality {
    pub name: String,
    /// Shu mas'ul bo'yicha tekshiruvlar.
    pub checks: usize,
    pub passed: usize,
    pub failed: usize,
    /// Ochiq nuqsonlar.
    pub open_defects: usize,
    /// Bartaraf etish muddati o'tganlari.
    pub overdue: usize,
    /// Sifat balli, 0-100.
    pub score: f64,
}

/// Mas'ullar kesimida sifat reytingi.
///
/// Ball sifat modulidagi umumiy ball bilan **bir xil qoidada** hisoblanadi:
/// ikki joyda ikki xil formula bo'lsa, reytingga ishonib bo'lmaydi.
pub fn contractor_quality(quality: &[QualityCheck], today: NaiveDate) -> Vec<ContractorQuality> {
    let mut names: Vec<String> = quality
        .iter()
        .map(|q| q.inspector.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    names.sort();
    names.dedup();

    let mut out: Vec<ContractorQuality> = names
        .into_iter()
        .map(|name| {
            let mine: Vec<QualityCheck> = quality
                .iter()
                .filter(|q| q.inspector.trim() == name)
                .cloned()
                .collect();
            let s = quality_score(&mine, today);
            ContractorQuality {
                name,
                checks: s.checks,
                passed: s.passed,
                failed: s.failed,
                open_defects: s.open,
                overdue: s.overdue,
                score: s.score,
            }
        })
        .collect();
    out.sort_by(|a, b| b.score.total_cmp(&a.score));
    out
}

// ================================================================ XIV.32. Prediktiv sifat

/// Nuqson ehtimoli yuqori ish (TZ XIV.32).
#[derive(Debug, Clone, PartialEq)]
pub struct QualityRisk {
    pub task_id: i64,
    /// Nima uchun xavfli deb belgilangani.
    pub reasons: Vec<QualityRiskReason>,
    /// Xavf darajasi: sabablar soni.
    pub level: usize,
}

/// Sifat xavfining sababi.
#[derive(Debug, Clone, PartialEq)]
pub enum QualityRiskReason {
    /// Shu ishda ilgari nuqson bo'lgan.
    PastDefects { count: usize },
    /// Ish kechikmoqda — shoshilinch bajarish sifatga ta'sir qiladi.
    Delayed { days: i64 },
    /// Normadan ortiq material sarfi — texnologiya buzilgan bo'lishi mumkin.
    OverUsage,
    /// Ish uchun tasdiqlangan texnologik karta yo'q.
    NoPpr,
    /// Yashirin ishlar tekshiruvi o'tkazilmagan.
    NoInspection,
}

/// Qaysi ishlarda nuqson ehtimoli yuqori.
///
/// Bashorat emas — **e'tibor ro'yxati**: har bir sabab bugungi
/// ma'lumotdan olingan va tekshirib ko'rish mumkin.
pub fn quality_risks(
    tasks: &[Task],
    quality: &[QualityCheck],
    consumption: &[ConsumptionLine],
    ppr: &[PprDoc],
    inspections: &[Inspection],
    progress_overdue: &[i64],
    schedule_late: impl Fn(i64) -> i64,
) -> Vec<QualityRisk> {
    let mut out = Vec::new();
    for t in tasks {
        // Tugagan ish uchun ogohlantirish kech.
        if t.fact_end.is_some() || t.progress >= 99.999 {
            continue;
        }
        let mut reasons = Vec::new();

        let past = quality
            .iter()
            .filter(|q| q.task_id == Some(t.id))
            .filter(|q| q.result != QualityResult::Pass)
            .count();
        if past > 0 {
            reasons.push(QualityRiskReason::PastDefects { count: past });
        }
        if progress_overdue.contains(&t.id) {
            reasons.push(QualityRiskReason::Delayed {
                days: schedule_late(t.id),
            });
        }
        if consumption.iter().any(|c| c.task_id == t.id && c.over) {
            reasons.push(QualityRiskReason::OverUsage);
        }
        // Boshlangan ishda tasdiqlangan karta bo'lishi kerak.
        if t.progress > 0.0 && !ppr.iter().any(|p| p.task_id == Some(t.id) && p.approved) {
            reasons.push(QualityRiskReason::NoPpr);
        }
        // Yarmidan ko'p bajarilgan ishda tekshiruv bo'lishi kerak.
        if t.progress >= 50.0
            && !inspections
                .iter()
                .any(|x| x.task_id == Some(t.id) && !x.open())
        {
            reasons.push(QualityRiskReason::NoInspection);
        }

        if reasons.len() >= 2 {
            out.push(QualityRisk {
                task_id: t.id,
                level: reasons.len(),
                reasons,
            });
        }
    }
    // Ko'proq sabab — ko'proq e'tibor.
    out.sort_by_key(|r| std::cmp::Reverse(r.level));
    out
}

// ================================================================ XIV.6. Brak taqiqi

/// Taqiqlangan materialning ishlatilishi (TZ XIV.6, XII.36).
#[derive(Debug, Clone, PartialEq)]
pub struct BannedUsage {
    pub material_id: i64,
    pub reason: String,
    /// Taqiqdan keyin chiqarilgan miqdor.
    pub qty: f64,
    pub moves: usize,
    pub last: Option<NaiveDate>,
}

/// Taqiqlangan material baribir ishlatilganini topadi.
///
/// Taqiq o'z-o'zidan harakatni to'xtatmaydi — bu yozuv qog'ozda qoladi.
/// Shuning uchun uni **ko'rsatish** kerak: kim, qachon va qancha.
pub fn banned_usage(materials: &[Material], moves: &[StockMove]) -> Vec<BannedUsage> {
    let mut out = Vec::new();
    for m in materials.iter().filter(|m| m.banned) {
        let used: Vec<&StockMove> = moves
            .iter()
            .filter(|x| x.material_id == m.id)
            .filter(|x| matches!(x.kind, MoveKind::Out))
            .collect();
        if used.is_empty() {
            continue;
        }
        out.push(BannedUsage {
            material_id: m.id,
            reason: m.ban_reason.clone(),
            qty: used.iter().map(|x| x.qty).sum(),
            moves: used.len(),
            last: used.iter().map(|x| x.date).max(),
        });
    }
    out.sort_by(|a, b| b.qty.total_cmp(&a.qty));
    out
}

// ================================================================ XVI.12, 42-43. Texnika

/// Texnika bandligidagi to'qnashuv (TZ XVI.12).
#[derive(Debug, Clone, PartialEq)]
pub struct BookingConflict {
    pub machine_id: i64,
    pub first: i64,
    pub second: i64,
    /// Kesishgan kunlar soni.
    pub days: i64,
}

/// Bir texnika bir vaqtda ikki ishga band qilinganini topadi.
pub fn booking_conflicts(bookings: &[MachineBooking]) -> Vec<BookingConflict> {
    let mut out = Vec::new();
    for (i, a) in bookings.iter().enumerate() {
        for b in bookings.iter().skip(i + 1) {
            if !a.overlaps(b) {
                continue;
            }
            let from = a.from.max(b.from);
            let to = a.to.min(b.to);
            out.push(BookingConflict {
                machine_id: a.machine_id,
                first: a.id,
                second: b.id,
                days: (to - from).num_days() + 1,
            });
        }
    }
    out
}

/// Texnika bo'yicha ta'mir yakuni (TZ XVI.26, 43).
#[derive(Debug, Clone, PartialEq)]
pub struct RepairSummary {
    pub machine_id: i64,
    pub repairs: usize,
    /// Nosozlik sababli ta'mirlar (rejali emas).
    pub faults: usize,
    pub cost: f64,
    /// Ta'mirda o'tgan kunlar.
    pub downtime: i64,
    /// Hozir ta'mirda.
    pub in_repair: bool,
    /// Ta'mir xarajati texnika qiymatining necha foizi.
    pub cost_pct: f64,
    /// Almashtirishni o'ylash kerak: ta'mir qiymati chegaradan oshgan.
    pub consider_replacing: bool,
}

/// Ta'mir xarajati texnika qiymatining shu ulushidan oshsa — savol tug'iladi.
pub const REPLACE_LIMIT_PCT: f64 = 40.0;

/// Texnikalar bo'yicha ta'mir yakunini yig'adi.
///
/// «Ta'mirlash yoki almashtirish» qarori uchun asosiy son — ta'mirga
/// ketgan pulning texnika qiymatiga nisbati.
pub fn repair_summary(
    machines: &[Machine],
    repairs: &[MachineRepair],
    today: NaiveDate,
) -> Vec<RepairSummary> {
    machines
        .iter()
        .map(|m| {
            let mine: Vec<&MachineRepair> =
                repairs.iter().filter(|r| r.machine_id == m.id).collect();
            let cost: f64 = mine.iter().map(|r| r.cost).sum();
            // Texnika qiymati noma'lum bo'lsa foizni hisoblab bo'lmaydi.
            let cost_pct = if m.price > 0.0 {
                cost / m.price * 100.0
            } else {
                0.0
            };
            RepairSummary {
                machine_id: m.id,
                repairs: mine.len(),
                faults: mine.iter().filter(|r| r.kind == RepairKind::Fault).count(),
                cost,
                downtime: mine.iter().map(|r| r.days(today)).sum(),
                in_repair: mine.iter().any(|r| r.open()),
                cost_pct,
                consider_replacing: cost_pct > REPLACE_LIMIT_PCT,
            }
        })
        .collect()
}

// ================================================================ XV.28, 34-36. Xavfsizlik tahlili

/// Ildiz sabab bo'yicha yakun (TZ XV.28).
#[derive(Debug, Clone, PartialEq)]
pub struct CauseLine {
    pub cause: RootCause,
    pub count: usize,
    /// Shundan jiddiy va kritiklari.
    pub serious: usize,
    /// Barcha hodisalarning necha foizi.
    pub pct: f64,
}

/// Hodisalar qaysi sabab bo'yicha takrorlanayotganini ko'rsatadi.
///
/// Chora **simptomga** emas, shu sababga qaratilishi kerak: bitta hodisa
/// tasodif bo'lishi mumkin, bir xil sababdagi uchtasi esa tizim nuqsoni.
pub fn root_causes(events: &[SafetyEvent]) -> Vec<CauseLine> {
    // Faqat haqiqiy hodisalar: instruktaj va tekshiruv sabab talab qilmaydi.
    let real: Vec<&SafetyEvent> = events
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                SafetyKind::Violation | SafetyKind::NearMiss | SafetyKind::Incident
            )
        })
        .collect();
    if real.is_empty() {
        return Vec::new();
    }

    let mut out: Vec<CauseLine> = Vec::new();
    for e in &real {
        match out.iter_mut().find(|c| c.cause == e.root_cause) {
            Some(c) => {
                c.count += 1;
                if matches!(e.severity, Severity::Critical | Severity::Major) {
                    c.serious += 1;
                }
            }
            None => out.push(CauseLine {
                cause: e.root_cause,
                count: 1,
                serious: usize::from(matches!(e.severity, Severity::Critical | Severity::Major)),
                pct: 0.0,
            }),
        }
    }
    let total = real.len() as f64;
    for c in &mut out {
        c.pct = c.count as f64 / total * 100.0;
    }
    // Ko'p uchraydigan sabab oldinda: chora shu yerdan boshlanadi.
    out.sort_by_key(|c| std::cmp::Reverse(c.count));
    out
}

/// Mas'ul bo'yicha xavfsizlik yakuni (TZ XV.35).
#[derive(Debug, Clone, PartialEq)]
pub struct SafetyRating {
    pub name: String,
    pub events: usize,
    /// Buzilish va baxtsiz hodisalar.
    pub violations: usize,
    pub incidents: usize,
    /// Yopilmagan yozuvlar.
    pub open: usize,
    /// Muddati o'tganlari.
    pub overdue: usize,
}

/// Mas'ullar kesimida xavfsizlik yozuvlari.
pub fn safety_rating(events: &[SafetyEvent], today: NaiveDate) -> Vec<SafetyRating> {
    let mut names: Vec<String> = events
        .iter()
        .map(|e| e.responsible.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    names.sort();
    names.dedup();

    let mut out: Vec<SafetyRating> = names
        .into_iter()
        .map(|name| {
            let mine: Vec<&SafetyEvent> = events
                .iter()
                .filter(|e| e.responsible.trim() == name)
                .collect();
            let open = mine
                .iter()
                .filter(|e| matches!(e.status, IssueStatus::Open | IssueStatus::InWork))
                .count();
            SafetyRating {
                events: mine.len(),
                violations: mine
                    .iter()
                    .filter(|e| e.kind == SafetyKind::Violation)
                    .count(),
                incidents: mine
                    .iter()
                    .filter(|e| e.kind == SafetyKind::Incident)
                    .count(),
                open,
                overdue: mine
                    .iter()
                    .filter(|e| {
                        matches!(e.status, IssueStatus::Open | IssueStatus::InWork)
                            && e.deadline.is_some_and(|d| d < today)
                    })
                    .count(),
                name,
            }
        })
        .collect();
    // Ko'p hodisali oldinda: e'tibor shu yerga kerak.
    out.sort_by_key(|r| std::cmp::Reverse(r.events));
    out
}

/// Xavfsizlik bo'yicha ogohlantirish (TZ XV.36).
#[derive(Debug, Clone, PartialEq)]
pub enum SafetyRisk {
    /// Xavfli zonada chora ko'rilmagan.
    ZoneNotReady { zone_id: i64 },
    /// Zona tekshiruvi muddati o'tgan.
    ZoneOverdue { zone_id: i64, days: i64 },
    /// Bir xil sabab uch va undan ko'p marta takrorlangan.
    RepeatedCause { cause: RootCause, count: usize },
    /// Ishga qo'yib bo'lmaydigan ishchi bor.
    BlockedWorkers { count: usize },
    /// Naryadi kamchilik bilan ochiq.
    BadPermits { count: usize },
}

/// Bir xil sabab shuncha marta takrorlansa — tizim nuqsoni.
pub const CAUSE_REPEAT_LIMIT: usize = 3;

/// Xavfsizlik bo'yicha nimalarga e'tibor berish kerakligini yig'adi.
///
/// Bashorat emas: har bir belgi bugungi yozuvlardan chiqadi va uni
/// tekshirib ko'rish mumkin.
pub fn safety_risks(
    zones: &[SafetyZone],
    events: &[SafetyEvent],
    workers: &[WorkerSafety],
    permits: &[WorkPermit],
    today: NaiveDate,
) -> Vec<SafetyRisk> {
    let mut out = Vec::new();

    for z in zones {
        if !z.ready {
            out.push(SafetyRisk::ZoneNotReady { zone_id: z.id });
        } else if z.overdue(today) {
            let days = z.check_due.map(|d| (today - d).num_days()).unwrap_or(0);
            out.push(SafetyRisk::ZoneOverdue {
                zone_id: z.id,
                days,
            });
        }
    }

    for c in root_causes(events) {
        if c.cause != RootCause::Unknown && c.count >= CAUSE_REPEAT_LIMIT {
            out.push(SafetyRisk::RepeatedCause {
                cause: c.cause,
                count: c.count,
            });
        }
    }

    let blocked = workers.iter().filter(|w| w.blocked()).count();
    if blocked > 0 {
        out.push(SafetyRisk::BlockedWorkers { count: blocked });
    }

    let bad = permits
        .iter()
        .filter(|p| p.status == PermitStatus::Open)
        .filter(|p| !permit_issues(p, workers, today).is_empty())
        .count();
    if bad > 0 {
        out.push(SafetyRisk::BadPermits { count: bad });
    }

    out
}

// ================================================================ VIII.28. Shartnoma monitoringi

/// Shartnoma bo'yicha ogohlantirish (TZ VIII.28).
#[derive(Debug, Clone, PartialEq)]
pub enum ContractAlert {
    /// Qiymat dastlabkidan sezilarli chetga chiqdi.
    SumDeviation { contract_id: i64, pct: f64 },
    /// Kelishuvda turgan o'zgarish uzoq qoldi.
    ChangePending { number: String, days: i64 },
    /// Muddati o'tgan to'lov.
    PaymentOverdue {
        number: String,
        days: i64,
        amount: f64,
    },
    /// Shartnoma muddati o'tgan, lekin yopilmagan.
    ContractOverdue { contract_id: i64, days: i64 },
    /// To'lov jadvali amaldagi summani qoplamaydi.
    ScheduleGap { contract_id: i64, gap: f64 },
    /// Qabul hujjati uzoq javobsiz turibdi.
    AcceptancePending { number: String, days: i64 },
}

/// Shartnoma summasi shundan ko'p chetga chiqsa — ogohlantiramiz.
pub const SUM_DEVIATION_LIMIT: f64 = 5.0;
/// Qaror shuncha kundan ko'p kutilsa — ogohlantiramiz.
pub const DECISION_LIMIT_DAYS: i64 = 10;

/// Shartnomalar bo'yicha nimalarga e'tibor berish kerakligini yig'adi.
///
/// Har bir ogohlantirishda **son** bor: foiz, kun yoki summa. «Diqqat
/// qiling» degan xabar buyurtmachiga hech narsa bermaydi.
pub fn contract_alerts(
    contracts: &[Contract],
    changes: &[ContractChange],
    stages: &[PaymentStage],
    acceptances: &[WorkAcceptance],
    today: NaiveDate,
) -> Vec<ContractAlert> {
    let mut out = Vec::new();

    for c in contracts {
        let st = contract_state(c, changes, stages, today);
        if st.change_pct().abs() > SUM_DEVIATION_LIMIT {
            out.push(ContractAlert::SumDeviation {
                contract_id: c.id,
                pct: st.change_pct(),
            });
        }
        if c.overdue(today) {
            out.push(ContractAlert::ContractOverdue {
                contract_id: c.id,
                days: (today - c.end).num_days(),
            });
        }
        // Jadval amaldagi summadan sezilarli kam bo'lsa — u eskirgan.
        if st.planned > 0.0 && st.schedule_gap() > st.current * 0.05 {
            out.push(ContractAlert::ScheduleGap {
                contract_id: c.id,
                gap: st.schedule_gap(),
            });
        }
    }

    for x in changes.iter().filter(|x| x.pending()) {
        let days = (today - x.date).num_days();
        if days > DECISION_LIMIT_DAYS {
            out.push(ContractAlert::ChangePending {
                number: x.number.clone(),
                days,
            });
        }
    }

    for s in stages.iter().filter(|s| s.overdue(today)) {
        out.push(ContractAlert::PaymentOverdue {
            number: s.number.clone(),
            days: s.delay_days(today),
            amount: s.left(),
        });
    }

    for a in acceptances.iter().filter(|a| a.pending()) {
        let days = (today - a.date).num_days();
        if days > DECISION_LIMIT_DAYS {
            out.push(ContractAlert::AcceptancePending {
                number: a.number.clone(),
                days,
            });
        }
    }

    out
}

// ================================================================ III.24-25, 33. Smeta zanjiri

/// Smeta pozitsiyasidan haqiqiy sarfgacha bo'lgan yo'l (TZ III.24-25, 27, 33).
#[derive(Debug, Clone, PartialEq)]
pub struct ChainLine {
    /// Qaysi ish. Smeta pozitsiyasi ishga bog'lanmagan bo'lsa — `None`.
    pub task_id: Option<i64>,
    /// Smetadagi reja qiymati.
    pub planned: f64,
    /// Ariza berilgan summa.
    pub requested: f64,
    /// Xarid qilingan summa.
    pub purchased: f64,
    /// Omborga kirim qilingan qiymat.
    pub received: f64,
    /// Ishga berilgan material qiymati.
    pub issued: f64,
    /// Haqiqiy tannarx: ish haqi, material va texnika.
    pub actual: f64,
    /// Bajarilish foizi.
    pub progress: f64,
    /// Bajarilgan ulushga to'g'ri keladigan reja.
    pub earned: f64,
    /// Farq: reja minus fakt. Manfiy — ortiqcha sarf.
    pub diff: f64,
}

impl ChainLine {
    /// Zanjirning har bosqichida qiymat kamaymasligi kerak: ariza
    /// xariddan kam bo'lsa — biror bosqich hujjatsiz o'tgan.
    pub fn has_gap(&self) -> bool {
        self.requested + 0.01 < self.purchased || self.received + 0.01 < self.issued
    }
}

/// Smetadan faktgacha bo'lgan zanjirni ishlar kesimida yig'adi.
///
/// Bu **yakuniy arxitektura** (TZ III.33): pul smetadan chiqib, ariza va
/// xarid orqali omborga, u yerdan ishga o'tadi va tannarxga aylanadi.
/// Har bosqich alohida modulda yozilgan — bu yerda ular bir qatorda.
#[allow(clippy::too_many_arguments)]
pub fn estimate_chain(
    items: &[EstimateItem],
    tasks: &[Task],
    requests: &[Request],
    purchases: &[Purchase],
    moves: &[StockMove],
    materials: &[Material],
    costs: &[TaskCost],
) -> Vec<ChainLine> {
    let mut out: Vec<ChainLine> = Vec::new();

    for t in tasks {
        // Smetada shu ishga bog'langan pozitsiyalar.
        let planned: f64 = items
            .iter()
            .filter(|i| i.task_id == Some(t.id))
            .map(|i| i.computed())
            .sum();
        // Ariza va xaridlar ishga bog'langanlari.
        let requested: f64 = requests
            .iter()
            .filter(|r| r.task_id == Some(t.id))
            .map(|r| {
                let price = r
                    .material_id
                    .and_then(|id| materials.iter().find(|m| m.id == id))
                    .map(|m| m.price)
                    .unwrap_or(0.0);
                r.qty * price
            })
            .sum();
        let purchased: f64 = purchases
            .iter()
            .filter(|p| p.task_id == Some(t.id))
            .map(|p| p.amount())
            .sum();
        // Omborga kirim va ishga chiqim.
        let received: f64 = moves
            .iter()
            .filter(|m| m.task_id == Some(t.id) && matches!(m.kind, MoveKind::In))
            .map(|m| m.qty * m.price)
            .sum();
        let issued: f64 = moves
            .iter()
            .filter(|m| m.task_id == Some(t.id) && matches!(m.kind, MoveKind::Out))
            .map(|m| {
                let price = if m.price > 0.0 {
                    m.price
                } else {
                    materials
                        .iter()
                        .find(|x| x.id == m.material_id)
                        .map(|x| x.price)
                        .unwrap_or(0.0)
                };
                m.qty * price
            })
            .sum();
        let actual = costs
            .iter()
            .find(|c| c.task_id == t.id)
            .map(|c| c.total)
            .unwrap_or(0.0);

        // Hech bir bosqichda yozuv bo'lmasa — qator ham kerak emas.
        if planned == 0.0 && requested == 0.0 && purchased == 0.0 && actual == 0.0 {
            continue;
        }

        let progress = t.progress.clamp(0.0, 100.0);
        let earned = planned * progress / 100.0;
        out.push(ChainLine {
            task_id: Some(t.id),
            planned,
            requested,
            purchased,
            received,
            issued,
            actual,
            progress,
            earned,
            diff: earned - actual,
        });
    }

    // Ortiqcha sarf kattalari oldinda: e'tibor shu yerga kerak.
    out.sort_by(|a, b| a.diff.total_cmp(&b.diff));
    out
}

/// Zanjir bo'yicha umumiy yakun.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChainTotals {
    pub planned: f64,
    pub requested: f64,
    pub purchased: f64,
    pub received: f64,
    pub issued: f64,
    pub actual: f64,
    pub earned: f64,
    pub diff: f64,
    /// Zanjirida uzilish bor qatorlar.
    pub gaps: usize,
}

/// Zanjir yakunini yig'adi.
pub fn chain_totals(lines: &[ChainLine]) -> ChainTotals {
    let sum = |f: fn(&ChainLine) -> f64| lines.iter().map(f).sum();
    ChainTotals {
        planned: sum(|l| l.planned),
        requested: sum(|l| l.requested),
        purchased: sum(|l| l.purchased),
        received: sum(|l| l.received),
        issued: sum(|l| l.issued),
        actual: sum(|l| l.actual),
        earned: sum(|l| l.earned),
        diff: sum(|l| l.diff),
        gaps: lines.iter().filter(|l| l.has_gap()).count(),
    }
}

// ================= IV.16, 19-20, 26. Hujjatni imzolashdan oldingi nazorat =================

/// Imzolashdan oldin topilgan bitta kamchilik (TZ IV.20).
#[derive(Debug, Clone, PartialEq)]
pub enum DocProblem {
    /// Ish hali tugallanmagan, hujjat esa imzoga qo'yilgan.
    WorkUnfinished { progress: f64 },
    /// Hujjat GPR ishiga bog'lanmagan — nimaga tegishli ekani noma'lum.
    NoTask,
    /// Yashirin ishlar dalolatnomasi, lekin texnik nazorat tekshiruvi yo'q.
    NoInspection,
    /// Tekshiruv o'tkazilgan, natijasi salbiy va bartaraf etilmagan.
    InspectionFailed { number: String },
    /// Sinov bayonnomasi, lekin laboratoriya sinovi yo'q.
    NoLabTest,
    /// Laboratoriya sinovi salbiy.
    LabFailed { number: String },
    /// Beton pasporti, lekin namuna natijasi yo'q yoki talabdan past.
    ConcreteWeak { sample: String },
    /// Hujjat sanasi ishning haqiqiy tugash sanasidan oldin.
    DatedBeforeWork { days: i64 },
    /// Shu turdagi imzolangan hujjat allaqachon bor.
    Duplicate { number: String },
    /// Raqam yoki mas'ul ko'rsatilmagan.
    Incomplete,
}

impl DocProblem {
    /// Kamchilik imzolashni to'sadimi. To'smaydiganlari — ogohlantirish.
    pub fn blocking(&self) -> bool {
        matches!(
            self,
            DocProblem::WorkUnfinished { .. }
                | DocProblem::InspectionFailed { .. }
                | DocProblem::LabFailed { .. }
                | DocProblem::ConcreteWeak { .. }
                | DocProblem::Incomplete
        )
    }
}

/// Bitta hujjat bo'yicha imzolashdan oldingi xulosa.
#[derive(Debug, Clone)]
pub struct DocCheck {
    pub doc_id: i64,
    pub number: String,
    pub problems: Vec<DocProblem>,
}

impl DocCheck {
    /// Imzolashga tayyormi — to'sadigan kamchilik yo'qmi.
    pub fn ready(&self) -> bool {
        !self.problems.iter().any(|p| p.blocking())
    }
}

/// Hujjatni imzolashdan oldingi tekshiruv uchun manba.
pub struct DocCtx<'a> {
    pub docs: &'a [ExecDoc],
    pub tasks: &'a [Task],
    pub inspections: &'a [Inspection],
    pub lab_tests: &'a [LabTest],
    pub concrete: &'a [ConcreteTest],
}

/// TZ IV.20: «hujjat imzolanishidan oldin AI uni tekshiradi».
///
/// Tekshiruv yangi hisob-kitob qilmaydi — mavjud modullardagi yozuvlarga
/// qaraydi: ish bajarilganmi (GPR), tekshiruv o'tganmi (texnik nazorat),
/// sinov natijasi bormi (laboratoriya). Shuning uchun bu yerdagi xulosa
/// tegishli modul ekranidagi bilan hech qachon ziddiyatga tushmaydi.
pub fn doc_readiness(ctx: &DocCtx) -> Vec<DocCheck> {
    let mut out = Vec::new();
    for d in ctx.docs {
        // Imzolangan va rad etilganlar tekshirilmaydi — qaror chiqib bo'lgan.
        if matches!(d.status, ExecDocStatus::Signed | ExecDocStatus::Rejected) {
            continue;
        }
        let mut problems = Vec::new();

        if d.number.trim().is_empty() || d.responsible.trim().is_empty() {
            problems.push(DocProblem::Incomplete);
        }

        match d
            .task_id
            .and_then(|id| ctx.tasks.iter().find(|t| t.id == id))
        {
            None => problems.push(DocProblem::NoTask),
            Some(task) => {
                if task.progress < 99.999 && task.fact_end.is_none() {
                    problems.push(DocProblem::WorkUnfinished {
                        progress: task.progress,
                    });
                }
                if let Some(end) = task.fact_end {
                    if d.date < end {
                        problems.push(DocProblem::DatedBeforeWork {
                            days: (end - d.date).num_days(),
                        });
                    }
                }
            }
        }

        // Tur bo'yicha maxsus talablar.
        let task_id = d.task_id;
        match d.kind {
            ExecDocKind::Hidden => {
                let insp: Vec<&Inspection> = ctx
                    .inspections
                    .iter()
                    .filter(|i| i.task_id == task_id && task_id.is_some() && i.done.is_some())
                    .collect();
                if insp.is_empty() {
                    problems.push(DocProblem::NoInspection);
                } else if let Some(bad) = insp
                    .iter()
                    .find(|i| i.result == InspectionResult::Fail && i.fixed_at.is_none())
                {
                    problems.push(DocProblem::InspectionFailed {
                        number: bad.number.clone(),
                    });
                }
            }
            ExecDocKind::Test => {
                let tests: Vec<&LabTest> = ctx
                    .lab_tests
                    .iter()
                    .filter(|l| l.task_id == task_id && task_id.is_some())
                    .collect();
                if tests.is_empty() {
                    problems.push(DocProblem::NoLabTest);
                } else if let Some(bad) = tests.iter().find(|l| l.result == LabTestResult::Fail) {
                    problems.push(DocProblem::LabFailed {
                        number: bad.number.clone(),
                    });
                }
            }
            ExecDocKind::Passport => {
                let samples: Vec<&ConcreteTest> = ctx
                    .concrete
                    .iter()
                    .filter(|c| c.task_id == task_id && task_id.is_some())
                    .collect();
                if let Some(bad) = samples
                    .iter()
                    .find(|c| c.actual.map(|a| a < c.required).unwrap_or(true))
                {
                    problems.push(DocProblem::ConcreteWeak {
                        sample: bad.sample.clone(),
                    });
                }
            }
            _ => {}
        }

        // Shu ish uchun shu turdagi imzolangan hujjat bormi.
        if let Some(dup) = ctx.docs.iter().find(|o| {
            o.id != d.id
                && o.kind == d.kind
                && o.task_id == d.task_id
                && d.task_id.is_some()
                && o.status == ExecDocStatus::Signed
                && o.id != d.replaces.unwrap_or(-1)
        }) {
            problems.push(DocProblem::Duplicate {
                number: dup.number.clone(),
            });
        }

        if !problems.is_empty() {
            out.push(DocCheck {
                doc_id: d.id,
                number: d.number.clone(),
                problems,
            });
        }
    }
    // To'sadiganlar oldinda.
    out.sort_by_key(|c| (c.ready(), c.doc_id));
    out
}

/// Yashirin ish yopilmagani uchun to'silgan ish (TZ IV.16).
#[derive(Debug, Clone)]
pub struct HiddenBlock {
    /// Boshlanmasligi kerak bo'lgan ish.
    pub task_id: i64,
    pub task_name: String,
    /// Yashirin ishi yopilmagan oldingi ish.
    pub pred_id: i64,
    pub pred_name: String,
    /// Dalolatnoma umuman yo'qmi (`false` — bor, lekin imzolanmagan).
    pub missing: bool,
    /// Keyingi ish allaqachon boshlanib ketganmi — shunda bu buzilish.
    pub already_started: bool,
}

/// TZ IV.16: yashirin ishlar dalolatnomasi imzolanmaguncha keyingi ish
/// boshlanmasligi kerak — beton quyilsa, armatura endi ko'rinmaydi.
///
/// Talab faqat yashirin ish hujjati kerak bo'lgan bo'limlarga qo'llanadi;
/// talab ro'yxatini [`required_docs`] belgilaydi, shu yerda qayta yozilmaydi.
pub fn hidden_blocks(
    tasks: &[Task],
    links: &[crate::model::Link],
    docs: &[ExecDoc],
) -> Vec<HiddenBlock> {
    let required = required_docs(tasks, docs, false);
    let needs_hidden = |id: i64| {
        required
            .iter()
            .any(|r| r.task_id == id && r.kind == ExecDocKind::Hidden)
    };

    let mut out = Vec::new();
    for l in links {
        let (Some(pred), Some(succ)) = (
            tasks.iter().find(|t| t.id == l.pred),
            tasks.iter().find(|t| t.id == l.succ),
        ) else {
            continue;
        };
        if !needs_hidden(pred.id) {
            continue;
        }
        let act = docs
            .iter()
            .filter(|d| d.task_id == Some(pred.id) && d.kind == ExecDocKind::Hidden)
            .max_by_key(|d| d.version);
        let signed = act
            .map(|d| d.status == ExecDocStatus::Signed)
            .unwrap_or(false);
        if signed {
            continue;
        }
        out.push(HiddenBlock {
            task_id: succ.id,
            task_name: succ.name.clone(),
            pred_id: pred.id,
            pred_name: pred.name.clone(),
            missing: act.is_none(),
            already_started: succ.progress > 0.0 || succ.fact_start.is_some(),
        });
    }
    // Buzilgan holatlar oldinda: ish boshlanib ketgan bo'lsa gap qattiqroq.
    out.sort_by_key(|b| (!b.already_started, b.task_id));
    out
}

// ================= XII.8, 15, 32-34. Material moslik, sertifikat va komplekt =================

/// Materialning loyihaga mosligi bo'yicha bitta e'tiroz (TZ XII.8, 32).
#[derive(Debug, Clone, PartialEq)]
pub enum FitProblem {
    /// Loyiha spetsifikatsiyasiga havola yo'q — nimaga asoslangani noma'lum.
    NoSpecRef,
    /// Smeta rasenkasi ko'rsatilmagan — pul bilan bog'lanmagan.
    NoEstimateCode,
    /// Texnik tavsif bo'sh: marka va GOST yozilmagan.
    NoSpec,
    /// Maxsus talab qo'yilgan, lekin tavsifda aks etmagan.
    SpecialNotInSpec { special: String },
    /// Sertifikat raqami yo'q.
    NoCertificate,
    /// Sertifikat muddati o'tgan.
    CertExpired { days: i64 },
    /// Sertifikat muddati tugayapti.
    CertExpiring { days: i64 },
    /// Taqiqlangan material, lekin sabab yozilmagan.
    BanWithoutReason,
    /// Bo'limi ko'rsatilmagan — qaysi loyiha qismiga tegishli ekani noma'lum.
    NoSection,
}

impl FitProblem {
    /// Material ishlatilgan bo'lsa, bu e'tiroz jiddiymi.
    pub fn severe(&self) -> bool {
        matches!(
            self,
            FitProblem::NoSpec
                | FitProblem::SpecialNotInSpec { .. }
                | FitProblem::NoCertificate
                | FitProblem::CertExpired { .. }
                | FitProblem::BanWithoutReason
        )
    }
}

/// Bitta material bo'yicha moslik xulosasi.
#[derive(Debug, Clone)]
pub struct MaterialFit {
    pub material_id: i64,
    pub name: String,
    /// Obyektda haqiqatan ishlatilganmi — ishlatilgani birinchi navbatda muhim.
    pub used: bool,
    pub problems: Vec<FitProblem>,
}

impl MaterialFit {
    /// Ishlatilgan materialda jiddiy e'tiroz bormi.
    pub fn critical(&self) -> bool {
        self.used && self.problems.iter().any(|p| p.severe())
    }
}

/// Sertifikat muddati tugashiga shuncha kun qolganda ogohlantiriladi.
pub const CERT_WARN_DAYS: i64 = 30;

/// TZ XII.8, 15, 32: material kartochkasi loyiha talabiga javob beradimi.
///
/// Tekshiruv kartochkadagi yozuvlarga qaraydi — spetsifikatsiya havolasi,
/// smeta rasenkasi, texnik tavsif va sertifikat. Ishlatilgan material
/// oldinda turadi: qog'ozdagi kamchilik bilan devordagi kamchilik bir xil
/// og'irlikda emas.
pub fn material_fit(
    materials: &[Material],
    moves: &[StockMove],
    today: NaiveDate,
) -> Vec<MaterialFit> {
    let mut out = Vec::new();
    for m in materials {
        let used = moves
            .iter()
            .any(|x| x.material_id == m.id && matches!(x.kind, MoveKind::Out));
        let mut problems = Vec::new();

        if m.spec_ref.trim().is_empty() {
            problems.push(FitProblem::NoSpecRef);
        }
        if m.estimate_code.trim().is_empty() {
            problems.push(FitProblem::NoEstimateCode);
        }
        if m.spec.trim().is_empty() {
            problems.push(FitProblem::NoSpec);
        } else if !m.special.trim().is_empty() {
            // Maxsus talab tavsifda aks etganmi: eng uzun so'z bo'yicha
            // qaraymiz, chunki qisqa so'zlar tasodifan mos kelib qoladi.
            let spec = m.spec.to_lowercase();
            let key = m
                .special
                .to_lowercase()
                .split_whitespace()
                .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
                .filter(|w| w.chars().count() >= 4)
                .max_by_key(|w| w.chars().count());
            if let Some(key) = key {
                if !spec.contains(&key) {
                    problems.push(FitProblem::SpecialNotInSpec {
                        special: m.special.clone(),
                    });
                }
            }
        }
        if m.section == Section::None {
            problems.push(FitProblem::NoSection);
        }
        if m.cert_no.trim().is_empty() {
            problems.push(FitProblem::NoCertificate);
        }
        if let Some(until) = m.cert_until {
            let days = (until - today).num_days();
            if days < 0 {
                problems.push(FitProblem::CertExpired { days: -days });
            } else if days <= CERT_WARN_DAYS {
                problems.push(FitProblem::CertExpiring { days });
            }
        }
        if m.banned && m.ban_reason.trim().is_empty() {
            problems.push(FitProblem::BanWithoutReason);
        }

        if !problems.is_empty() {
            out.push(MaterialFit {
                material_id: m.id,
                name: m.name.clone(),
                used,
                problems,
            });
        }
    }
    // Ishlatilgan va jiddiy e'tirozlilar oldinda.
    out.sort_by_key(|f| (!f.critical(), !f.used, f.material_id));
    out
}

/// Ish uchun material komplekti (TZ XII.33).
#[derive(Debug, Clone)]
pub struct MaterialKit {
    pub task_id: i64,
    /// Komplektdagi material turlari soni.
    pub total: usize,
    /// Yetishmayotgan turlar soni.
    pub missing: usize,
    /// Komplekt tayyorligi, foizda.
    pub ready_pct: f64,
    /// Ish shuncha kundan keyin boshlanadi. Manfiy — boshlangan.
    pub days_left: i64,
    /// Eng katta yetishmovchilik qaysi materialda.
    pub worst: Option<i64>,
}

impl MaterialKit {
    /// Komplekt to'liq yig'ilganmi.
    pub fn complete(&self) -> bool {
        self.missing == 0
    }
}

/// TZ XII.33: ish uchun material bittalab emas, komplekt bo'lib kerak.
///
/// Bitta material yetishmasa ham ish boshlanmaydi, shuning uchun tayyorlik
/// komplekt darajasida o'lchanadi. Yetishmovchilik [`readiness`] natijasidan
/// olinadi — shu sababli bu yerdagi son «Tayyorlik» jadvalidagi bilan
/// hech qachon ziddiyatga tushmaydi.
pub fn material_kits(
    norms: &[MaterialNorm],
    tasks: &[Task],
    ready: &[Readiness],
    today: NaiveDate,
    within_days: i64,
) -> Vec<MaterialKit> {
    let mut out = Vec::new();
    for task in tasks {
        if task.progress >= 100.0 {
            continue;
        }
        let start = task.fact_start.unwrap_or(task.plan_start);
        let days_left = (start - today).num_days();
        if days_left > within_days {
            continue;
        }
        let total = norms.iter().filter(|n| n.task_id == task.id).count();
        if total == 0 {
            continue;
        }
        let short: Vec<&Readiness> = ready.iter().filter(|r| r.task_id == task.id).collect();
        let missing = short.len();
        let worst = short
            .iter()
            .max_by(|a, b| a.short.total_cmp(&b.short))
            .map(|r| r.material_id);
        out.push(MaterialKit {
            task_id: task.id,
            total,
            missing,
            ready_pct: (total - missing.min(total)) as f64 * 100.0 / total as f64,
            days_left,
            worst,
        });
    }
    // Eng yaqin boshlanadigan va eng kam tayyor komplekt oldinda.
    out.sort_by(|a, b| {
        a.ready_pct
            .total_cmp(&b.ready_pct)
            .then(a.days_left.cmp(&b.days_left))
    });
    out
}

/// Bitta yetkazib beruvchi bo'yicha material taklifi (TZ XII.34).
#[derive(Debug, Clone)]
pub struct MakerOffer {
    pub supplier: String,
    pub price: f64,
    pub delivery_days: i64,
    /// Shu yetkazib beruvchidan olingan xaridlar soni.
    pub purchases: usize,
    /// Taklif tanlanganmi.
    pub chosen: bool,
    /// Eng arzon takliftan qancha qimmat, foizda.
    pub over_pct: f64,
}

/// Bitta material bo'yicha takliflar solishtiruvi.
#[derive(Debug, Clone)]
pub struct MakerComparison {
    pub title: String,
    pub unit: String,
    pub offers: Vec<MakerOffer>,
    /// Eng arzon va eng qimmat orasidagi farq, foizda.
    pub spread_pct: f64,
}

/// TZ XII.34: bir xil material bo'yicha yetkazib beruvchilarni solishtirish.
///
/// Narx yolg'iz o'zi yetarli emas: muddat va shu yetkazib beruvchi bilan
/// oldingi tajriba ham ustunda turadi. Solishtirish faqat **ikki va undan
/// ortiq** taklifi bor materiallar bo'yicha ko'rsatiladi — bitta taklif
/// solishtiruv emas.
pub fn maker_comparison(quotes: &[Quote], purchases: &[Purchase]) -> Vec<MakerComparison> {
    let key = |s: &str| s.trim().to_lowercase();
    let mut titles: Vec<String> = Vec::new();
    for q in quotes {
        if !titles.iter().any(|t| key(t) == key(&q.title)) {
            titles.push(q.title.clone());
        }
    }

    let mut out = Vec::new();
    for title in titles {
        let group: Vec<&Quote> = quotes
            .iter()
            .filter(|q| key(&q.title) == key(&title))
            .collect();
        if group.len() < 2 {
            continue;
        }
        let min = group.iter().map(|q| q.price).fold(f64::INFINITY, f64::min);
        let max = group.iter().map(|q| q.price).fold(0.0_f64, f64::max);
        if min <= 0.0 {
            continue;
        }
        let mut offers: Vec<MakerOffer> = group
            .iter()
            .map(|q| MakerOffer {
                supplier: q.supplier.clone(),
                price: q.price,
                delivery_days: q.delivery_days,
                purchases: purchases
                    .iter()
                    .filter(|p| key(&p.supplier) == key(&q.supplier))
                    .count(),
                chosen: q.chosen,
                over_pct: (q.price - min) * 100.0 / min,
            })
            .collect();
        offers.sort_by(|a, b| a.price.total_cmp(&b.price));
        out.push(MakerComparison {
            title,
            unit: group[0].unit.clone(),
            offers,
            spread_pct: (max - min) * 100.0 / min,
        });
    }
    // Narx tarqoqligi katta materiallar oldinda — tanlov shu yerda ko'proq
    // pul tejaydi.
    out.sort_by(|a, b| b.spread_pct.total_cmp(&a.spread_pct));
    out
}

// ================= VI.25, 33-34. Kunni yakunlash va ishni yopish =================

/// Kun yakunlanishidan oldin topilgan kamchilik (TZ VI.33-34).
#[derive(Debug, Clone, PartialEq)]
pub enum DayIssue {
    /// Kunlik jurnalga yozuv kiritilmagan.
    NoJournal,
    /// Jurnalda ob-havo yozilmagan — sovuqda beton ishlari uchun muhim.
    NoWeather,
    /// Foto biriktirilmagan.
    NoPhoto,
    /// Tabel to'ldirilmagan: bugun hech kim belgilanmagan.
    NoTimesheet,
    /// Jurnaldagi ishchi soni tabeldagidan farq qiladi.
    CrewMismatch { journal: i64, timesheet: i64 },
    /// Ketayotgan ish bo'yicha bugun hajm kiritilmagan.
    NoVolume { task_id: i64 },
    /// Bugun ochilgan xavfsizlik yoki sifat holati yopilmagan.
    OpenIssues { count: usize },
}

impl DayIssue {
    /// Kunni yopishga to'sadimi. To'smaydiganlari — eslatma.
    pub fn blocking(&self) -> bool {
        matches!(
            self,
            DayIssue::NoJournal | DayIssue::NoTimesheet | DayIssue::CrewMismatch { .. }
        )
    }
}

/// Kun yakunini tekshirish uchun manba.
pub struct DayCtx<'a> {
    pub journal: &'a [JournalEntry],
    pub timesheet: &'a [TimesheetEntry],
    pub tasks: &'a [Task],
    /// Bugun ketayotgan ishlar ro'yxati — ekran qaysi ishni ko'rsatsa, o'sha.
    pub running: &'a [i64],
    /// Bugun ochiq qolgan xavfsizlik va sifat holatlari soni.
    pub open_issues: usize,
    pub day: NaiveDate,
}

/// TZ VI.33-34: ish kuni yopilishidan va hisobot yuborilishidan oldingi
/// tekshiruv.
///
/// Tekshiruv **bugungi yozuvlarga** qaraydi: jurnal, tabel va bajarilgan
/// hajm. Maqsadi — kechqurun esdan chiqqan narsani ertaga emas, bugun
/// aytish: bir kun o'tsa, kim qancha ishlagani endi eslanmaydi.
pub fn day_close(ctx: &DayCtx) -> Vec<DayIssue> {
    let mut out = Vec::new();

    let today_entries: Vec<&JournalEntry> =
        ctx.journal.iter().filter(|j| j.date == ctx.day).collect();
    let today_hours: Vec<&TimesheetEntry> = ctx
        .timesheet
        .iter()
        .filter(|e| e.date == ctx.day && e.hours > 0.0)
        .collect();

    if today_entries.is_empty() {
        out.push(DayIssue::NoJournal);
    } else {
        if today_entries.iter().all(|j| j.weather.trim().is_empty()) {
            out.push(DayIssue::NoWeather);
        }
        if today_entries.iter().all(|j| j.photos.trim().is_empty()) {
            out.push(DayIssue::NoPhoto);
        }
    }

    if today_hours.is_empty() {
        out.push(DayIssue::NoTimesheet);
    } else if let Some(j) = today_entries.iter().find(|j| j.workers > 0) {
        let counted = today_hours.len() as i64;
        // Bitta odam farq — yaxlitlash emas, lekin ikkitadan ortiq farq
        // odatda kimdir tabelga tushmaganini bildiradi.
        if (j.workers - counted).abs() > 1 {
            out.push(DayIssue::CrewMismatch {
                journal: j.workers,
                timesheet: counted,
            });
        }
    }

    for id in ctx.running {
        let has_volume = today_entries
            .iter()
            .any(|j| j.task_id == Some(*id) && j.volume > 0.0);
        if !has_volume && ctx.tasks.iter().any(|t| t.id == *id) {
            out.push(DayIssue::NoVolume { task_id: *id });
        }
    }

    if ctx.open_issues > 0 {
        out.push(DayIssue::OpenIssues {
            count: ctx.open_issues,
        });
    }

    // To'sadiganlar oldinda.
    out.sort_by_key(|i| !i.blocking());
    out
}

/// Ishni yopishdan oldingi ogohlantirish (TZ VI.25).
#[derive(Debug, Clone, PartialEq)]
pub enum CloseWarning {
    /// Sifat bo'yicha to'siq: nuqson yoki qabul nazorati.
    Quality { defects: usize, points: usize },
    /// Talab qilinadigan ijro hujjati imzolanmagan.
    Docs { missing: usize },
    /// Ishga material chiqim qilinmagan — sarf yozilmagan.
    NoConsumption,
    /// Ishga bironta soat yozilmagan — kim bajarganini bilib bo'lmaydi.
    NoLabour,
    /// Jurnalda bu ish bo'yicha hajm yozuvi yo'q.
    NoJournalVolume,
}

impl CloseWarning {
    /// Jiddiy: yopilsa keyin tuzatib bo'lmaydigan narsa qoladi.
    pub fn severe(&self) -> bool {
        matches!(
            self,
            CloseWarning::Quality { .. } | CloseWarning::Docs { .. }
        )
    }
}

/// TZ VI.25: ishni «bajarildi» deb belgilashdan oldin nimalar yopilmagani.
///
/// Sifat to'sig'i [`task_blocks`] dan, hujjat talabi [`required_docs`] dan
/// olinadi — bu yerda qayta yozilmaydi, shuning uchun prorab ekranidagi
/// ogohlantirish sifat va hujjat ekranlaridagi bilan bir xil gapiradi.
pub fn close_warnings(
    task_id: i64,
    blocks: &[TaskBlock],
    required: &[RequiredDoc],
    moves: &[StockMove],
    timesheet: &[TimesheetEntry],
    journal: &[JournalEntry],
) -> Vec<CloseWarning> {
    let mut out = Vec::new();

    if let Some(b) = blocks.iter().find(|b| b.task_id == task_id) {
        if b.blocked() {
            out.push(CloseWarning::Quality {
                defects: b.open_defects,
                points: b.pending_points,
            });
        }
    }

    let missing = required
        .iter()
        .filter(|r| r.task_id == task_id && !r.signed)
        .count();
    if missing > 0 {
        out.push(CloseWarning::Docs { missing });
    }

    let consumed = moves
        .iter()
        .any(|m| m.task_id == Some(task_id) && matches!(m.kind, MoveKind::Out));
    if !consumed {
        out.push(CloseWarning::NoConsumption);
    }

    let worked = timesheet
        .iter()
        .any(|e| e.task_id == Some(task_id) && e.hours > 0.0);
    if !worked {
        out.push(CloseWarning::NoLabour);
    }

    let logged = journal
        .iter()
        .any(|j| j.task_id == Some(task_id) && j.volume > 0.0);
    if !logged {
        out.push(CloseWarning::NoJournalVolume);
    }

    out.sort_by_key(|w| !w.severe());
    out
}

// ================= V.10-11, 16, 24, 32. Kunlik jurnal tahlili =================

/// Bir kunlik hajmga sarflangan material (TZ V.10-11).
#[derive(Debug, Clone)]
pub struct DayMaterial {
    pub task_id: i64,
    pub material_id: i64,
    /// Jurnalga yozilgan hajm.
    pub volume: f64,
    /// Shu hajmga norma bo'yicha kerak bo'lgan miqdor.
    pub by_norm: f64,
    /// Shu kuni ishga haqiqatda berilgan miqdor.
    pub issued: f64,
    /// Farq: fakt minus norma. Musbat — ortiqcha sarf.
    pub diff: f64,
    /// Ruxsat etilgan chetlanish, foizda.
    pub tolerance: f64,
}

impl DayMaterial {
    /// Chetlanish ruxsat etilgan chegaradan chiqdimi.
    pub fn over(&self) -> bool {
        self.by_norm > 0.0 && self.diff * 100.0 / self.by_norm > self.tolerance
    }
}

/// TZ V.10-11: kunlik jurnaldagi hajm bilan o'sha kuni berilgan materialni
/// solishtiradi.
///
/// Solishtirish **kun ichida** bo'lishi muhim: oy oxirida jami bo'yicha
/// qaraganda ortiqcha sarf o'rtachada yo'qoladi va sababini topib bo'lmaydi.
pub fn day_material(
    journal: &[JournalEntry],
    norms: &[MaterialNorm],
    moves: &[StockMove],
    day: NaiveDate,
) -> Vec<DayMaterial> {
    let mut out = Vec::new();
    for j in journal.iter().filter(|j| j.date == day && j.volume > 0.0) {
        let Some(task_id) = j.task_id else { continue };
        for n in norms.iter().filter(|n| n.task_id == task_id) {
            let by_norm = n.per_unit * j.volume;
            let issued: f64 = moves
                .iter()
                .filter(|m| {
                    m.date == day
                        && m.task_id == Some(task_id)
                        && m.material_id == n.material_id
                        && matches!(m.kind, MoveKind::Out)
                })
                .map(|m| m.qty)
                .sum();
            // Na norma, na fakt bo'lmasa — qator ham kerak emas.
            if by_norm <= 0.0 && issued <= 0.0 {
                continue;
            }
            out.push(DayMaterial {
                task_id,
                material_id: n.material_id,
                volume: j.volume,
                by_norm,
                issued,
                diff: issued - by_norm,
                tolerance: n.tolerance,
            });
        }
    }
    // Ortiqcha sarf kattalari oldinda.
    out.sort_by(|a, b| b.diff.total_cmp(&a.diff));
    out
}

/// Ertangi kunga reja bo'yicha bitta ish (TZ V.16).
#[derive(Debug, Clone)]
pub struct TomorrowTask {
    pub task_id: i64,
    /// Ish ertaga boshlanadimi (`false` — davom etadi).
    pub starts: bool,
    /// Rejadan qolgan hajm.
    pub volume_left: f64,
    /// Material komplekti tayyorligi, foizda.
    pub kit_ready: f64,
    /// Yetishmayotgan material turlari.
    pub missing: usize,
    /// Bugun shu ishda ishlagan odamlar soni.
    pub crew_today: usize,
}

impl TomorrowTask {
    /// Ertaga to'siqsiz boshlash mumkinmi.
    pub fn ready(&self) -> bool {
        self.missing == 0 && self.crew_today > 0
    }
}

/// TZ V.16: ertangi kunga reja — nima ketadi, material yetadimi, kim bor.
///
/// Komplekt tayyorligi [`material_kits`] dan olinadi: ertangi rejadagi son
/// «Komplekt» jadvalidagi bilan bir xil bo'lishi kerak.
pub fn tomorrow_plan(
    tasks: &[Task],
    running_tomorrow: &[i64],
    running_today: &[i64],
    kits: &[MaterialKit],
    timesheet: &[TimesheetEntry],
    today: NaiveDate,
) -> Vec<TomorrowTask> {
    let mut out = Vec::new();
    for id in running_tomorrow {
        let Some(task) = tasks.iter().find(|t| t.id == *id) else {
            continue;
        };
        let kit = kits.iter().find(|k| k.task_id == *id);
        out.push(TomorrowTask {
            task_id: *id,
            starts: !running_today.contains(id),
            volume_left: task.volume * (100.0 - task.progress.clamp(0.0, 100.0)) / 100.0,
            kit_ready: kit.map(|k| k.ready_pct).unwrap_or(100.0),
            missing: kit.map(|k| k.missing).unwrap_or(0),
            crew_today: timesheet
                .iter()
                .filter(|e| e.date == today && e.task_id == Some(*id) && e.hours > 0.0)
                .count(),
        });
    }
    // Ertaga boshlanadigan va tayyor bo'lmaganlari oldinda: e'tibor shularga.
    out.sort_by(|a, b| {
        a.ready()
            .cmp(&b.ready())
            .then(b.starts.cmp(&a.starts))
            .then(a.kit_ready.total_cmp(&b.kit_ready))
    });
    out
}

/// Jurnal yozuvidagi shubhali holat (TZ V.32).
#[derive(Debug, Clone, PartialEq)]
pub enum JournalDoubt {
    /// Yozilgan hajm ishda qolgan hajmdan katta.
    VolumeOverPlan { entered: f64, left: f64 },
    /// Ishchi soni ko'rsatilgan, tabelda esa o'sha kuni hech kim yo'q.
    CrewWithoutTimesheet { workers: i64 },
    /// Bir xil hajm ketma-ket kunlarda takrorlangan.
    RepeatedVolume { days: usize, volume: f64 },
    /// Yozuv kelajak sanaga kiritilgan.
    FutureDate,
    /// Bir ishchiga to'g'ri keladigan hajm shu ish bo'yicha o'rtachadan
    /// keskin katta.
    ImplausibleRate { per_worker: f64, average: f64 },
}

impl JournalDoubt {
    /// Jiddiy: hajm yoki odam soni haqiqatga to'g'ri kelmaydi.
    pub fn severe(&self) -> bool {
        matches!(
            self,
            JournalDoubt::VolumeOverPlan { .. }
                | JournalDoubt::CrewWithoutTimesheet { .. }
                | JournalDoubt::FutureDate
        )
    }
}

/// Bitta jurnal yozuvi bo'yicha shubhalar.
#[derive(Debug, Clone)]
pub struct JournalCheck {
    pub entry_id: i64,
    pub date: NaiveDate,
    pub doubts: Vec<JournalDoubt>,
}

/// Bir xil hajm shuncha kun ketma-ket takrorlansa — shubha.
pub const REPEAT_DAYS: usize = 3;

/// Bir ishchiga to'g'ri keladigan hajm o'rtachadan shuncha barobar oshsa —
/// shubha.
pub const RATE_FACTOR: f64 = 2.5;

/// TZ V.32: soxta hisobotlardan himoya.
///
/// Bu ayblov emas — savol: dastur faqat **ichki ziddiyatni** ko'rsatadi
/// (yozilgan hajm rejadan katta, tabelda odam yo'q, bir xil raqam
/// takrorlanadi). Har qanday holatda oxirgi so'z odamniki.
pub fn journal_doubts(
    journal: &[JournalEntry],
    tasks: &[Task],
    timesheet: &[TimesheetEntry],
    today: NaiveDate,
) -> Vec<JournalCheck> {
    // Ish bo'yicha bir ishchiga to'g'ri keladigan o'rtacha hajm.
    let average = |task_id: i64| -> f64 {
        let rows: Vec<&JournalEntry> = journal
            .iter()
            .filter(|j| j.task_id == Some(task_id) && j.volume > 0.0 && j.workers > 0)
            .collect();
        if rows.is_empty() {
            return 0.0;
        }
        rows.iter()
            .map(|j| j.volume / j.workers as f64)
            .sum::<f64>()
            / rows.len() as f64
    };

    let mut out = Vec::new();
    for j in journal {
        let mut doubts = Vec::new();

        if j.date > today {
            doubts.push(JournalDoubt::FutureDate);
        }

        if let Some(task) = j.task_id.and_then(|id| tasks.iter().find(|t| t.id == id)) {
            let left = task.volume * (100.0 - task.progress.clamp(0.0, 100.0)) / 100.0;
            // Tugallangan ishga yozilgan hajm ham shu qoidaga tushadi.
            if j.volume > left + task.volume * 0.001 && task.volume > 0.0 {
                doubts.push(JournalDoubt::VolumeOverPlan {
                    entered: j.volume,
                    left,
                });
            }

            if j.workers > 0 {
                let avg = average(task.id);
                let rate = j.volume / j.workers as f64;
                if avg > 0.0 && rate > avg * RATE_FACTOR {
                    doubts.push(JournalDoubt::ImplausibleRate {
                        per_worker: rate,
                        average: avg,
                    });
                }
            }

            // Bir xil hajm ketma-ket kunlarda.
            let mut streak = 1;
            let mut day = j.date - chrono::Duration::days(1);
            while journal.iter().any(|o| {
                o.task_id == j.task_id && o.date == day && (o.volume - j.volume).abs() < 0.0001
            }) {
                streak += 1;
                day -= chrono::Duration::days(1);
            }
            if streak >= REPEAT_DAYS && j.volume > 0.0 {
                doubts.push(JournalDoubt::RepeatedVolume {
                    days: streak,
                    volume: j.volume,
                });
            }
        }

        if j.workers > 0 && !timesheet.iter().any(|e| e.date == j.date && e.hours > 0.0) {
            doubts.push(JournalDoubt::CrewWithoutTimesheet { workers: j.workers });
        }

        if !doubts.is_empty() {
            doubts.sort_by_key(|d| !d.severe());
            out.push(JournalCheck {
                entry_id: j.id,
                date: j.date,
                doubts,
            });
        }
    }
    // Yangi yozuvlar oldinda: savol tez berilsa, javob ham aniq bo'ladi.
    out.sort_by_key(|c| std::cmp::Reverse(c.date));
    out
}

// ================= VII.26, 35-36. Chek-list va yakuniy qabul =================

/// Tekshiruv chek-listining bitta bandi (TZ VII.26).
#[derive(Debug, Clone, PartialEq)]
pub struct ChecklistItem {
    /// Nimani tekshirish kerak — i18n kaliti.
    pub key: &'static str,
    /// Band majburiymi.
    pub required: bool,
}

/// TZ VII.26: tekshiruvga chiqishdan oldin chek-list.
///
/// Chek-list ish **bo'limi** va tekshiruv **turi** dan chiqadi: nimani
/// tekshirish kerakligi shu ikkisi bilan aniqlanadi. Ro'yxat qisqa —
/// uzun ro'yxat o'qilmaydi, o'qilmagan ro'yxat esa foydasiz.
pub fn inspection_checklist(kind: InspectionKind, section: Section) -> Vec<ChecklistItem> {
    let item = |key: &'static str, required: bool| ChecklistItem { key, required };
    let mut out = vec![
        item("cl_marks", true),
        item("cl_drawing", true),
        item("cl_docs", true),
    ];

    // Tekshiruv turiga xos bandlar.
    match kind {
        InspectionKind::Hidden => {
            out.push(item("cl_hidden_ready", true));
            out.push(item("cl_hidden_photo", true));
            out.push(item("cl_hidden_clean", false));
        }
        InspectionKind::Concrete => {
            out.push(item("cl_concrete_sample", true));
            out.push(item("cl_concrete_temp", true));
            out.push(item("cl_concrete_care", false));
        }
        InspectionKind::Geodesy => {
            out.push(item("cl_geo_base", true));
            out.push(item("cl_geo_tolerance", true));
        }
        InspectionKind::Material => {
            out.push(item("cl_mat_cert", true));
            out.push(item("cl_mat_batch", true));
            out.push(item("cl_mat_storage", false));
        }
        InspectionKind::Volume => {
            out.push(item("cl_vol_measure", true));
            out.push(item("cl_vol_journal", true));
        }
        InspectionKind::Final => {
            out.push(item("cl_final_defects", true));
            out.push(item("cl_final_docs", true));
            out.push(item("cl_final_tests", true));
        }
        InspectionKind::Physical => out.push(item("cl_phys_visual", true)),
    }

    // Bo'limga xos bandlar.
    match section {
        Section::Kj | Section::Km => {
            out.push(item("cl_sec_rebar", true));
            out.push(item("cl_sec_weld", section == Section::Km));
        }
        Section::Vk | Section::Ov => out.push(item("cl_sec_pressure", true)),
        Section::Eom | Section::Ss => out.push(item("cl_sec_insulation", true)),
        Section::Pb => out.push(item("cl_sec_fire", true)),
        Section::Ar | Section::None => {}
    }
    out
}

/// Yakuniy qabulga to'siq bo'layotgan holat (TZ VII.35-36).
#[derive(Debug, Clone, PartialEq)]
pub enum FinalBlock {
    /// Tugallanmagan ishlar.
    TasksOpen { count: usize },
    /// Imzolanmagan ijro hujjatlari.
    DocsUnsigned { count: usize },
    /// Bartaraf etilmagan nuqsonlar.
    DefectsOpen { count: usize },
    /// Salbiy laboratoriya sinovlari.
    LabFailed { count: usize },
    /// Yopilmagan texnik nazorat tekshiruvlari.
    InspectionsOpen { count: usize },
    /// Yopilmagan xavfsizlik holatlari.
    SafetyOpen { count: usize },
    /// Qabul qilinmagan ish topshiruvlari.
    AcceptancePending { count: usize },
}

impl FinalBlock {
    /// Nechta yozuv haqida gap ketyapti.
    pub fn count(&self) -> usize {
        match self {
            FinalBlock::TasksOpen { count }
            | FinalBlock::DocsUnsigned { count }
            | FinalBlock::DefectsOpen { count }
            | FinalBlock::LabFailed { count }
            | FinalBlock::InspectionsOpen { count }
            | FinalBlock::SafetyOpen { count }
            | FinalBlock::AcceptancePending { count } => *count,
        }
    }
}

/// Obyektning yakuniy qabulga tayyorligi.
#[derive(Debug, Clone, Default)]
pub struct FinalReadiness {
    /// Tayyorlik foizi: yopilgan shartlar ulushi.
    pub ready_pct: f64,
    /// Tekshirilgan shartlar soni.
    pub total_checks: usize,
    pub blocks: Vec<FinalBlock>,
}

impl FinalReadiness {
    /// Obyektni topshirish mumkinmi.
    pub fn ready(&self) -> bool {
        self.blocks.is_empty()
    }
}

/// Yakuniy qabul tekshiruvi uchun manba.
pub struct FinalCtx<'a> {
    pub tasks: &'a [Task],
    pub required: &'a [RequiredDoc],
    pub quality: &'a [QualityCheck],
    pub lab_tests: &'a [LabTest],
    pub inspections: &'a [Inspection],
    pub safety: &'a [SafetyEvent],
    pub acceptances: &'a [WorkAcceptance],
}

/// TZ VII.35-36: obyekt yakuniy qabulga tayyormi.
///
/// Tayyorlik yetti shart bo'yicha o'lchanadi va har biri **o'z modulidagi**
/// yozuvdan olinadi. Shuning uchun bu yerdagi son tegishli ekrandagi bilan
/// bir xil bo'ladi: yakuniy qabul alohida hisob-kitob emas, mavjud
/// holatlarning yig'indisi.
pub fn final_readiness(ctx: &FinalCtx) -> FinalReadiness {
    let mut blocks = Vec::new();

    let open_tasks = ctx
        .tasks
        .iter()
        .filter(|t| t.progress < 99.999 && t.fact_end.is_none())
        .count();
    if open_tasks > 0 {
        blocks.push(FinalBlock::TasksOpen { count: open_tasks });
    }

    let unsigned = ctx.required.iter().filter(|r| !r.signed).count();
    if unsigned > 0 {
        blocks.push(FinalBlock::DocsUnsigned { count: unsigned });
    }

    let defects = ctx
        .quality
        .iter()
        .filter(|q| !q.defect.trim().is_empty() && q.fixed_at.is_none())
        .count();
    if defects > 0 {
        blocks.push(FinalBlock::DefectsOpen { count: defects });
    }

    let lab = ctx
        .lab_tests
        .iter()
        .filter(|l| l.result == LabTestResult::Fail)
        .count();
    if lab > 0 {
        blocks.push(FinalBlock::LabFailed { count: lab });
    }

    let insp = ctx
        .inspections
        .iter()
        .filter(|i| {
            i.done.is_none() || (i.result == InspectionResult::Fail && i.fixed_at.is_none())
        })
        .count();
    if insp > 0 {
        blocks.push(FinalBlock::InspectionsOpen { count: insp });
    }

    let safety = ctx
        .safety
        .iter()
        .filter(|s| matches!(s.status, IssueStatus::Open | IssueStatus::InWork))
        .count();
    if safety > 0 {
        blocks.push(FinalBlock::SafetyOpen { count: safety });
    }

    let accept = ctx
        .acceptances
        .iter()
        .filter(|a| matches!(a.state, AcceptState::Submitted | AcceptState::Rejected))
        .count();
    if accept > 0 {
        blocks.push(FinalBlock::AcceptancePending { count: accept });
    }

    // Eng ko'p yozuvli to'siq oldinda: ish shu yerdan boshlanadi.
    blocks.sort_by_key(|b| std::cmp::Reverse(b.count()));

    let total_checks = 7;
    FinalReadiness {
        ready_pct: (total_checks - blocks.len()) as f64 * 100.0 / total_checks as f64,
        total_checks,
        blocks,
    }
}

// ================= X.33, 37-38, 42. Obyektlar bo'yicha xaridlar =================

/// Bitta obyektning bitta material bo'yicha xaridi.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectBuy {
    pub project_id: i64,
    pub qty: f64,
    /// Vaznlangan o'rtacha narx: jami summa / jami miqdor.
    pub price: f64,
    pub amount: f64,
    /// Nechta xarid yozuvi.
    pub deals: usize,
}

/// Bir material bo'yicha obyektlar kesimidagi manzara (TZ X.33, 38).
#[derive(Debug, Clone)]
pub struct CentralLine {
    /// Material nomi — kod bo'yicha emas, nom bo'yicha guruhlanadi, chunki
    /// har obyektning o'z katalogi bor va kodlar mos kelmasligi mumkin.
    pub title: String,
    pub unit: String,
    pub objects: Vec<ObjectBuy>,
    pub total_qty: f64,
    pub total_amount: f64,
    /// Eng arzon obyektdagi narx.
    pub best_price: f64,
    /// Eng qimmat va eng arzon orasidagi farq, foizda.
    pub spread_pct: f64,
    /// Hammasi eng arzon narxda olinganda tejaladigan summa (TZ X.37).
    pub saving: f64,
}

impl CentralLine {
    /// Markazlashtirish ma'noli bo'ladigan holat: material bir nechta
    /// obyektda olinadi va narxlar farq qiladi.
    pub fn worth_central(&self) -> bool {
        self.objects.len() > 1 && self.spread_pct > CENTRAL_SPREAD
    }
}

/// Narx tarqoqligi shu foizdan oshsa markazlashtirish taklif qilinadi.
pub const CENTRAL_SPREAD: f64 = 10.0;

/// TZ X.33, 37-38: obyektlar bo'yicha xaridlarni bir jadvalda solishtiradi.
///
/// Guruhlash **nom bo'yicha**: har obyektning o'z material katalogi bor va
/// kodlar mos kelmasligi mumkin. Bu qo'pol, lekin halol: dastur mos
/// kelmagan kodlarni o'zicha bir xil deb e'lon qilmaydi.
///
/// Tejash — hammasi eng arzon narxda olinganda chiqadigan farq. Bu **yuqori
/// chegara**, kafolat emas: hajm va yetkazish sharti har xil bo'lishi mumkin.
pub fn central_purchases(rows: &[(i64, Vec<Purchase>)]) -> Vec<CentralLine> {
    let key = |s: &str| s.trim().to_lowercase();

    // Nom bo'yicha yig'amiz: (nom, birlik, obyektlar bo'yicha yig'indi).
    /// Bitta obyekt bo'yicha oraliq yig'indi: id, miqdor, summa, yozuvlar soni.
    type ObjectSum = (i64, f64, f64, usize);
    let mut by_title: Vec<(String, String, Vec<ObjectSum>)> = Vec::new();
    for (pid, purchases) in rows {
        for p in purchases {
            if p.title.trim().is_empty() || p.qty <= 0.0 {
                continue;
            }
            let idx = match by_title
                .iter()
                .position(|(t, _, _)| key(t) == key(&p.title))
            {
                Some(i) => i,
                None => {
                    by_title.push((p.title.clone(), p.unit.clone(), Vec::new()));
                    by_title.len() - 1
                }
            };
            let slot = &mut by_title[idx].2;
            match slot.iter_mut().find(|(id, _, _, _)| id == pid) {
                Some(e) => {
                    e.1 += p.qty;
                    e.2 += p.amount();
                    e.3 += 1;
                }
                None => slot.push((*pid, p.qty, p.amount(), 1)),
            }
        }
    }

    let mut out = Vec::new();
    for (title, unit, entries) in by_title {
        let mut objects: Vec<ObjectBuy> = entries
            .into_iter()
            .map(|(project_id, qty, amount, deals)| ObjectBuy {
                project_id,
                qty,
                price: if qty > 0.0 { amount / qty } else { 0.0 },
                amount,
                deals,
            })
            .filter(|o| o.price > 0.0)
            .collect();
        if objects.is_empty() {
            continue;
        }
        objects.sort_by(|a, b| a.price.total_cmp(&b.price));

        let best_price = objects[0].price;
        let worst_price = objects.last().map(|o| o.price).unwrap_or(best_price);
        let total_qty: f64 = objects.iter().map(|o| o.qty).sum();
        let total_amount: f64 = objects.iter().map(|o| o.amount).sum();
        out.push(CentralLine {
            title,
            unit,
            total_qty,
            total_amount,
            best_price,
            spread_pct: if best_price > 0.0 {
                (worst_price - best_price) * 100.0 / best_price
            } else {
                0.0
            },
            saving: total_amount - total_qty * best_price,
            objects,
        });
    }
    // Eng katta tejash imkoniyati oldinda.
    out.sort_by(|a, b| b.saving.total_cmp(&a.saving));
    out
}

/// Markazlashtirilgan buyurtmani obyektlar bo'yicha bo'lish (TZ X.42).
#[derive(Debug, Clone, PartialEq)]
pub struct SplitLine {
    pub project_id: i64,
    pub qty: f64,
    /// Umumiy hajmdagi ulushi, foizda.
    pub share_pct: f64,
    /// Shu ulushga to'g'ri keladigan summa.
    pub amount: f64,
}

/// TZ X.42: bitta katta buyurtmani obyektlar ehtiyojiga qarab bo'ladi.
///
/// Bo'lish **ehtiyoj ulushiga** qarab: kim ko'p so'ragan bo'lsa, unga ko'p
/// tegadi. Yaxlitlash oxirgi qatorga yig'iladi, shunda bo'laklar yig'indisi
/// har doim jamiga teng bo'ladi — aks holda omborda hisob buzilardi.
pub fn split_order(need: &[(i64, f64)], total_qty: f64, unit_price: f64) -> Vec<SplitLine> {
    let sum: f64 = need.iter().map(|(_, q)| q.max(0.0)).sum();
    if sum <= 0.0 || total_qty <= 0.0 {
        return Vec::new();
    }
    let mut out: Vec<SplitLine> = Vec::new();
    let mut given = 0.0;
    for (i, (pid, q)) in need.iter().enumerate() {
        let q = q.max(0.0);
        if q <= 0.0 {
            continue;
        }
        let share = q / sum;
        // Oxirgi qatorda qoldiqni to'liq beramiz.
        let qty = if i + 1 == need.len() {
            total_qty - given
        } else {
            (total_qty * share * 1000.0).round() / 1000.0
        };
        given += qty;
        out.push(SplitLine {
            project_id: *pid,
            qty,
            share_pct: share * 100.0,
            amount: qty * unit_price,
        });
    }
    // Yaxlitlash tufayli oxirgi qator manfiy bo'lib qolmasin.
    if let Some(last) = out.last_mut() {
        if last.qty < 0.0 {
            last.qty = 0.0;
            last.amount = 0.0;
        }
    }
    out
}

// ================= X.18-19, 21-22. Kelishuv, almashtirish, shartnoma =================

/// Xarid bo'yicha nazorat e'tirozi (TZ X.18-19, 22).
#[derive(Debug, Clone, PartialEq)]
pub enum SupplyIssue {
    /// Texnik kelishuvsiz buyurtma berilgan.
    NoTechApproval { number: String, amount: f64 },
    /// Almashtirish tasdiqlangan analoglar ro'yxatida yo'q.
    UnapprovedSubstitute { number: String, material: String },
    /// Almashtiruvchi tasdiqlangan, lekin texnik kelishuv olinmagan.
    SubstituteWithoutTech { number: String },
    /// Shartnoma summasidan oshib ketildi.
    ContractOverrun {
        contract: String,
        over: f64,
        pct: f64,
    },
    /// Shartnoma muddati tugagan, xarid esa davom etyapti.
    ContractExpired { contract: String, days: i64 },
    /// Yirik xarid shartnomasiz rasmiylashtirilgan.
    NoContract { number: String, amount: f64 },
}

impl SupplyIssue {
    /// To'lovni to'xtatishga arziydigan darajadagi e'tirozmi.
    pub fn severe(&self) -> bool {
        matches!(
            self,
            SupplyIssue::UnapprovedSubstitute { .. }
                | SupplyIssue::ContractOverrun { .. }
                | SupplyIssue::ContractExpired { .. }
        )
    }
}

/// Shartnomasiz rasmiylashtirilishi mumkin bo'lgan xarid chegarasi.
///
/// Chegara **shartli**: har tashkilotning o'z tartibi bor. Shuning uchun u
/// bitta joyda turadi va ekranda ochiq aytiladi.
pub const CONTRACT_LIMIT: f64 = 50_000_000.0;

/// TZ X.18-19, 22: xaridlar tartibini tekshiradi.
///
/// Uch savol: buyurtma texnik kelishuvdan o'tganmi, almashtirish
/// tasdiqlanganmi va shartnoma sharti buzilmayaptimi. Almashtirishning
/// tasdig'i [`MaterialAlt`] katalogidan olinadi — bu yerda qayta
/// belgilanmaydi.
pub fn supply_control(
    purchases: &[Purchase],
    contracts: &[Contract],
    alts: &[MaterialAlt],
    materials: &[Material],
    today: NaiveDate,
) -> Vec<SupplyIssue> {
    let mut out = Vec::new();
    let name_of = |id: i64| {
        materials
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.name.clone())
            .unwrap_or_default()
    };

    for p in purchases {
        // Qoralama hali buyurtma emas — undan talab qilinmaydi.
        if p.status == PurchaseStatus::Draft {
            continue;
        }

        if let Some(orig) = p.substitute_for {
            let approved = alts
                .iter()
                .any(|a| a.material_id == orig && Some(a.alt_id) == p.material_id && a.approved());
            if !approved {
                out.push(SupplyIssue::UnapprovedSubstitute {
                    number: p.number.clone(),
                    material: name_of(orig),
                });
            } else if !p.tech_ok {
                // Analog tasdiqlangan bo'lsa ham, aynan shu ishga mosligini
                // muhandis ko'rishi kerak.
                out.push(SupplyIssue::SubstituteWithoutTech {
                    number: p.number.clone(),
                });
            }
        } else if !p.tech_ok {
            out.push(SupplyIssue::NoTechApproval {
                number: p.number.clone(),
                amount: p.amount(),
            });
        }

        if p.contract_id.is_none() && p.amount() > CONTRACT_LIMIT {
            out.push(SupplyIssue::NoContract {
                number: p.number.clone(),
                amount: p.amount(),
            });
        }
    }

    // Shartnoma bo'yicha yakun: summa va muddat.
    for c in contracts.iter().filter(|c| c.kind == ContractKind::Supply) {
        let under: Vec<&Purchase> = purchases
            .iter()
            .filter(|p| p.contract_id == Some(c.id) && p.status != PurchaseStatus::Draft)
            .collect();
        if under.is_empty() {
            continue;
        }
        let spent: f64 = under.iter().map(|p| p.amount()).sum();
        if c.sum > 0.0 && spent > c.sum {
            out.push(SupplyIssue::ContractOverrun {
                contract: c.number.clone(),
                over: spent - c.sum,
                pct: (spent - c.sum) * 100.0 / c.sum,
            });
        }
        // Muddati tugagach berilgan buyurtma — shartnoma tashqarisida.
        if let Some(late) = under
            .iter()
            .filter(|p| p.date > c.end)
            .max_by_key(|p| p.date)
        {
            out.push(SupplyIssue::ContractExpired {
                contract: c.number.clone(),
                days: (late.date - c.end).num_days(),
            });
        }
        let _ = today;
    }

    // Jiddiylari oldinda.
    out.sort_by_key(|i| !i.severe());
    out
}

// ================= XVII.23-25, 35, 49. Kesimlar bo'yicha tahlil =================

/// Mas'ul (pudratchi) bo'yicha yakun (TZ XVII.23).
#[derive(Debug, Clone)]
pub struct ContractorReport {
    pub name: String,
    /// Shu mas'ulga biriktirilgan ishlar.
    pub tasks: usize,
    pub done: usize,
    /// Muddati o'tgan va tugallanmagan ishlar.
    pub overdue: usize,
    /// Muddatida tugatilgan ishlar ulushi, foizda.
    pub on_time_pct: f64,
    /// O'rtacha kechikish, kunlarda (faqat kechikkanlari bo'yicha).
    pub avg_delay: f64,
    /// Sifat balli — [`contractor_quality`] dan olinadi, qayta hisoblanmaydi.
    pub quality: f64,
    /// Ochiq nuqsonlar.
    pub defects: usize,
    /// Yopilmagan xavfsizlik holatlari.
    pub safety: usize,
}

impl ContractorReport {
    /// E'tibor talab qiladigan mas'ul.
    pub fn attention(&self) -> bool {
        self.overdue > 0 || self.defects > 0 || self.safety > 0
    }
}

/// TZ XVII.23: mas'ullar kesimida ish, sifat va xavfsizlik bir jadvalda.
///
/// Sifat balli [`contractor_quality`] dan olinadi — bu yerda qayta
/// hisoblanmaydi, shuning uchun sifat ekranidagi ball bilan bir xil bo'ladi.
pub fn contractor_report(
    tasks: &[Task],
    quality: &[QualityCheck],
    safety: &[SafetyEvent],
    today: NaiveDate,
) -> Vec<ContractorReport> {
    let key = |s: &str| s.trim().to_lowercase();
    let by_quality = contractor_quality(quality, today);

    let mut names: Vec<String> = Vec::new();
    for t in tasks {
        if t.responsible.trim().is_empty() {
            continue;
        }
        if !names.iter().any(|n| key(n) == key(&t.responsible)) {
            names.push(t.responsible.clone());
        }
    }

    let mut out = Vec::new();
    for name in names {
        let mine: Vec<&Task> = tasks
            .iter()
            .filter(|t| key(&t.responsible) == key(&name))
            .collect();
        let done = mine
            .iter()
            .filter(|t| t.progress >= 99.999 || t.fact_end.is_some())
            .count();
        let overdue = mine
            .iter()
            .filter(|t| {
                t.progress < 99.999
                    && t.fact_end.is_none()
                    && t.plan_start + chrono::Duration::days(t.duration.max(0)) < today
            })
            .count();

        // Muddatida tugatilganlar: haqiqiy tugash sanasi rejadan keyin emas.
        let finished: Vec<&&Task> = mine.iter().filter(|t| t.fact_end.is_some()).collect();
        let mut on_time = 0usize;
        let mut delays: Vec<i64> = Vec::new();
        for t in &finished {
            let plan_end = t.plan_start + chrono::Duration::days(t.duration.max(0));
            let end = t.fact_end.unwrap();
            if end <= plan_end {
                on_time += 1;
            } else {
                delays.push((end - plan_end).num_days());
            }
        }

        let q = by_quality.iter().find(|c| key(&c.name) == key(&name));
        out.push(ContractorReport {
            tasks: mine.len(),
            done,
            overdue,
            on_time_pct: if finished.is_empty() {
                // Tugatilgan ishi yo'q — ulush ham yo'q, nolga tenglashtirish
                // yolg'on baho berardi.
                100.0
            } else {
                on_time as f64 * 100.0 / finished.len() as f64
            },
            avg_delay: if delays.is_empty() {
                0.0
            } else {
                delays.iter().sum::<i64>() as f64 / delays.len() as f64
            },
            quality: q.map(|c| c.score).unwrap_or(100.0),
            defects: q.map(|c| c.open_defects).unwrap_or(0),
            safety: safety
                .iter()
                .filter(|s| {
                    key(&s.responsible) == key(&name)
                        && matches!(s.status, IssueStatus::Open | IssueStatus::InWork)
                })
                .count(),
            name,
        });
    }
    // E'tibor talab qiladiganlar oldinda, keyin sifat balli bo'yicha.
    out.sort_by(|a, b| {
        b.attention()
            .cmp(&a.attention())
            .then(a.quality.total_cmp(&b.quality))
    });
    out
}

/// Yetkazib beruvchi bo'yicha yakun (TZ XVII.24).
#[derive(Debug, Clone)]
pub struct SupplierReport {
    pub name: String,
    pub deals: usize,
    pub amount: f64,
    /// To'liq yetkazilgan buyurtmalar ulushi, foizda.
    pub complete_pct: f64,
    /// Muddatida yetkazilganlar ulushi, foizda.
    pub on_time_pct: f64,
    /// O'rtacha kechikish, kunlarda.
    pub avg_delay: f64,
    /// Shu ta'minotchining takliflari eng arzonidan qancha qimmat, foizda.
    pub price_over_pct: f64,
    /// Kirish nazoratida rad etilgan partiyalar.
    pub rejected: usize,
}

impl SupplierReport {
    /// Ishonchlilik balli, 0-100.
    ///
    /// Uch ulush teng vaznda: to'liq yetkazish, muddat va sifat. Narx
    /// **ballga kirmaydi** — arzon lekin kechikadigan ta'minotchini yaxshi
    /// deb ko'rsatib qo'ymaslik uchun; narx alohida ustunda turadi.
    pub fn score(&self) -> f64 {
        let quality = if self.deals == 0 {
            100.0
        } else {
            (1.0 - self.rejected as f64 / self.deals as f64) * 100.0
        };
        ((self.complete_pct + self.on_time_pct + quality) / 3.0).clamp(0.0, 100.0)
    }
}

/// TZ XVII.24: yetkazib beruvchilarni bir jadvalda solishtiradi.
///
/// Narx ballga kiritilmaydi: arzon, lekin kechikadigan ta'minotchi yaxshi
/// ko'rinib qolardi. Narx alohida ustunda turadi va qaror odamniki.
pub fn supplier_report(
    purchases: &[Purchase],
    quotes: &[Quote],
    quality: &[QualityCheck],
    today: NaiveDate,
) -> Vec<SupplierReport> {
    let key = |s: &str| s.trim().to_lowercase();
    let mut names: Vec<String> = Vec::new();
    for p in purchases {
        if p.supplier.trim().is_empty() {
            continue;
        }
        if !names.iter().any(|n| key(n) == key(&p.supplier)) {
            names.push(p.supplier.clone());
        }
    }

    // Har material bo'yicha eng arzon taklif — narx ustunini hisoblash uchun.
    let best_for = |title: &str| -> f64 {
        quotes
            .iter()
            .filter(|q| key(&q.title) == key(title) && q.price > 0.0)
            .map(|q| q.price)
            .fold(f64::INFINITY, f64::min)
    };

    let mut out = Vec::new();
    for name in names {
        let mine: Vec<&Purchase> = purchases
            .iter()
            .filter(|p| key(&p.supplier) == key(&name) && p.status != PurchaseStatus::Draft)
            .collect();
        if mine.is_empty() {
            continue;
        }
        let complete = mine
            .iter()
            .filter(|p| p.delivered_qty + 0.0001 >= p.qty)
            .count();

        // Muddat: yetkazilganlar bo'yicha rejadagi sana bilan solishtiriladi.
        let mut on_time = 0usize;
        let mut late = 0usize;
        let mut delays: Vec<i64> = Vec::new();
        for p in &mine {
            let arrived = p.delivered_qty > 0.0;
            if arrived && p.delivery_date >= today {
                on_time += 1;
            } else if !arrived && p.delivery_date < today {
                late += 1;
                delays.push((today - p.delivery_date).num_days());
            } else {
                on_time += 1;
            }
        }

        // Narx: shu ta'minotchining takliflari eng arzonidan qancha qimmat.
        let mut over: Vec<f64> = Vec::new();
        for q in quotes.iter().filter(|q| key(&q.supplier) == key(&name)) {
            let best = best_for(&q.title);
            if best.is_finite() && best > 0.0 {
                over.push((q.price - best) * 100.0 / best);
            }
        }

        out.push(SupplierReport {
            deals: mine.len(),
            amount: mine.iter().map(|p| p.amount()).sum(),
            complete_pct: complete as f64 * 100.0 / mine.len() as f64,
            on_time_pct: on_time as f64 * 100.0 / mine.len() as f64,
            avg_delay: if delays.is_empty() {
                0.0
            } else {
                delays.iter().sum::<i64>() as f64 / delays.len() as f64
            },
            price_over_pct: if over.is_empty() {
                0.0
            } else {
                over.iter().sum::<f64>() / over.len() as f64
            },
            // Kirish nazorati: shu ta'minotchidan kelgan material bo'yicha
            // salbiy tekshiruv. Bog'lanish tekshiruv mavzusi bilan xarid
            // nomi ustma-ust tushishiga qarab topiladi — sifat yozuvida
            // ta'minotchi maydoni yo'q, shuning uchun bu eng ishonchli
            // mavjud bog'lanish.
            rejected: quality
                .iter()
                .filter(|q| {
                    q.kind == QualityKind::Input
                        && q.result == QualityResult::Fail
                        && mine.iter().any(|p| {
                            !p.title.trim().is_empty() && key(&q.subject).contains(&key(&p.title))
                        })
                })
                .count(),
            name,
        });
        let _ = late;
    }
    // Ishonchsizlari oldinda.
    out.sort_by(|a, b| a.score().total_cmp(&b.score()));
    out
}

/// Ikki ko'rsatkich orasidagi bog'liqlik (TZ XVII.25).
#[derive(Debug, Clone)]
pub struct Correlation {
    /// Nima bilan nima solishtirilgani — i18n kaliti.
    pub key: &'static str,
    /// Pirson koeffitsiyenti, -1 dan 1 gacha.
    pub r: f64,
    /// Nechta juftlik bo'yicha hisoblangan.
    pub points: usize,
}

impl Correlation {
    /// Bog'liqlik e'tiborga arziydimi.
    ///
    /// Ikki shart birga: koeffitsiyent yetarlicha katta **va** juftliklar
    /// soni yetarli. Uch nuqtadan chiqqan «kuchli bog'liqlik» — tasodif.
    pub fn meaningful(&self) -> bool {
        self.r.abs() >= CORR_LIMIT && self.points >= CORR_MIN_POINTS
    }
}

/// Bog'liqlik shu qiymatdan kuchli bo'lsa ko'rsatiladi.
pub const CORR_LIMIT: f64 = 0.5;

/// Bog'liqlik shuncha juftlikdan kam bo'lsa ko'rsatilmaydi.
pub const CORR_MIN_POINTS: usize = 8;

/// Pirson korrelyatsiya koeffitsiyenti.
///
/// Qiymatlar bir xil bo'lsa (dispersiya nol) koeffitsiyent aniqlanmaydi —
/// bunda nol qaytariladi, chunki «bog'liqlik yo'q» halolroq javob.
pub fn pearson(xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len().min(ys.len());
    if n < 2 {
        return 0.0;
    }
    let mx = xs.iter().take(n).sum::<f64>() / n as f64;
    let my = ys.iter().take(n).sum::<f64>() / n as f64;
    let mut num = 0.0;
    let mut dx = 0.0;
    let mut dy = 0.0;
    for i in 0..n {
        let a = xs[i] - mx;
        let b = ys[i] - my;
        num += a * b;
        dx += a * a;
        dy += b * b;
    }
    if dx <= f64::EPSILON || dy <= f64::EPSILON {
        return 0.0;
    }
    (num / (dx * dy).sqrt()).clamp(-1.0, 1.0)
}

/// TZ XVII.25: oldindan tanlangan juftliklar bo'yicha bog'liqlikni o'lchaydi.
///
/// Bu **sabab emas, birgalikda o'zgarish**: ikki son birga o'zgargani
/// birinchisi ikkinchisini keltirib chiqargan degani emas. Ekranda buni
/// ochiq aytish shart, aks holda tasodifiy bog'liqlik qaror asosiga
/// aylanib qoladi.
pub fn correlations(
    tasks: &[Task],
    journal: &[JournalEntry],
    timesheet: &[TimesheetEntry],
    costs: &[TaskCost],
    today: NaiveDate,
) -> Vec<Correlation> {
    let mut out = Vec::new();

    // 1. Brigada kattaligi va kunlik hajm.
    let mut crew = Vec::new();
    let mut volume = Vec::new();
    for j in journal.iter().filter(|j| j.volume > 0.0 && j.workers > 0) {
        crew.push(j.workers as f64);
        volume.push(j.volume);
    }
    out.push(Correlation {
        key: "corr_crew_volume",
        r: pearson(&crew, &volume),
        points: crew.len(),
    });

    // 2. Ish hajmi va tannarx.
    let mut vol = Vec::new();
    let mut cost = Vec::new();
    for c in costs {
        if let Some(t) = tasks.iter().find(|t| t.id == c.task_id) {
            if t.volume > 0.0 && c.total > 0.0 {
                vol.push(t.volume);
                cost.push(c.total);
            }
        }
    }
    out.push(Correlation {
        key: "corr_volume_cost",
        r: pearson(&vol, &cost),
        points: vol.len(),
    });

    // 3. Kunlik soat va kunlik hajm.
    let mut hours = Vec::new();
    let mut day_volume = Vec::new();
    let mut days: Vec<NaiveDate> = journal
        .iter()
        .filter(|j| j.date <= today && j.volume > 0.0)
        .map(|j| j.date)
        .collect();
    days.sort_unstable();
    days.dedup();
    for d in days {
        let h: f64 = timesheet
            .iter()
            .filter(|e| e.date == d)
            .map(|e| e.hours)
            .sum();
        let v: f64 = journal
            .iter()
            .filter(|j| j.date == d)
            .map(|j| j.volume)
            .sum();
        if h > 0.0 && v > 0.0 {
            hours.push(h);
            day_volume.push(v);
        }
    }
    out.push(Correlation {
        key: "corr_hours_volume",
        r: pearson(&hours, &day_volume),
        points: hours.len(),
    });

    // Kuchlilari oldinda.
    out.sort_by(|a, b| b.r.abs().total_cmp(&a.r.abs()));
    out
}

/// Loyiha o'zgarishlari bo'yicha yakun (TZ XVII.35).
#[derive(Debug, Clone, Default)]
pub struct ChangeReport {
    pub total: usize,
    pub approved: usize,
    pub pending: usize,
    pub rejected: usize,
    /// Tasdiqlangan o'zgarishlarning summasi.
    pub approved_sum: f64,
    /// Qaror kutayotganlarning summasi — bu hali pul emas.
    pub pending_sum: f64,
    /// Tasdiqlangan o'zgarishlar qo'shgan kunlar.
    pub added_days: i64,
    /// Qaror qabul qilishgacha o'rtacha kun.
    pub avg_decision_days: f64,
    /// Turlar bo'yicha soni.
    pub by_kind: Vec<(ChangeKind, usize, f64)>,
}

/// TZ XVII.35: shartnoma o'zgarishlarini bir joyda ko'rsatadi.
///
/// Tasdiqlangan va qaror kutayotgan summalar **alohida** turadi: qaror
/// kutayotgani hali pul emas va uni jamiga qo'shish byudjetni yolg'on
/// ko'rsatardi.
pub fn change_report(changes: &[ContractChange]) -> ChangeReport {
    let mut r = ChangeReport {
        total: changes.len(),
        ..Default::default()
    };
    let mut decision_days: Vec<i64> = Vec::new();

    for c in changes {
        match c.status {
            ChangeStatus::Approved => {
                r.approved += 1;
                r.approved_sum += c.amount;
                r.added_days += c.days;
            }
            ChangeStatus::Rejected => r.rejected += 1,
            _ => {
                r.pending += 1;
                r.pending_sum += c.amount;
            }
        }
        if let Some(at) = c.decided_at {
            decision_days.push((at - c.date).num_days().max(0));
        }

        match r.by_kind.iter_mut().find(|(k, _, _)| *k == c.kind) {
            Some(e) => {
                e.1 += 1;
                e.2 += c.amount;
            }
            None => r.by_kind.push((c.kind, 1, c.amount)),
        }
    }

    r.avg_decision_days = if decision_days.is_empty() {
        0.0
    } else {
        decision_days.iter().sum::<i64>() as f64 / decision_days.len() as f64
    };
    // Eng katta summali tur oldinda.
    r.by_kind.sort_by(|a, b| b.2.total_cmp(&a.2));
    r
}

/// Rahbar uchun umumiy ball (TZ XVII.49).
#[derive(Debug, Clone, Default)]
pub struct ExecutiveScore {
    /// Muddat: rejadan orqada qolish va muddati o'tgan ishlar.
    pub schedule: f64,
    /// Pul: smeta va bajarilgan ish qiymati farqi.
    pub money: f64,
    /// Sifat balli — sifat modulidan.
    pub quality: f64,
    /// Xavfsizlik balli — xavfsizlik modulidan.
    pub safety: f64,
    /// Ta'minot: kechikkan arizalar ulushi.
    pub supply: f64,
    /// Hujjatlar: imzolangan ijro hujjatlari ulushi.
    pub docs: f64,
    /// Umumiy ball, 0-100.
    pub total: f64,
}

/// Umumiy ball uchun manba.
pub struct ExecCtx<'a> {
    /// Rejadan orqada qolish, kunlarda.
    pub delay_days: i64,
    /// Muddati o'tgan ishlar soni va jami ishlar.
    pub overdue: usize,
    pub tasks: usize,
    /// Bajarilgan ish qiymati va shu ishga ketgan haqiqiy sarf.
    pub earned: f64,
    pub actual: f64,
    /// Sifat va xavfsizlik ballari — o'z modullaridan.
    pub quality_score: f64,
    pub safety_score: f64,
    /// Ta'minot qatorlari.
    pub supply: &'a [SupplyLine],
    /// Talab qilinadigan hujjatlar.
    pub required_docs: &'a [RequiredDoc],
}

/// TZ XVII.49: rahbar uchun bitta ko'rsatkich.
///
/// Ball **yangi hisob qilmaydi**: sifat va xavfsizlik ballari o'z
/// modullaridan olinadi, qolgan uchtasi esa shu ekranlardagi sonlardan
/// chiqadi. Shuning uchun umumiy ball pasayganda sababini har doim aniq
/// bo'limdan topish mumkin.
///
/// Vaznlar teng emas: xavfsizlik va sifat og'irroq, chunki ularni keyin
/// tuzatib bo'lmaydi — muddat va pulni esa qayta rejalashtirish mumkin.
pub fn executive_score(ctx: &ExecCtx) -> ExecutiveScore {
    // Muddat: har kun kechikish uchun bir ball, muddati o'tgan ish ulushi
    // esa yarim vazn bilan.
    let overdue_share = if ctx.tasks == 0 {
        0.0
    } else {
        ctx.overdue as f64 * 100.0 / ctx.tasks as f64
    };
    let schedule = (100.0 - ctx.delay_days.max(0) as f64 - overdue_share * 0.5).clamp(0.0, 100.0);

    // Pul: bajarilgan ish qiymatidan qancha oshib ketilgani.
    let money = if ctx.earned <= 0.0 {
        100.0
    } else {
        let over = (ctx.actual - ctx.earned) * 100.0 / ctx.earned;
        (100.0 - over.max(0.0)).clamp(0.0, 100.0)
    };

    // Ta'minot: kechikkan arizalar ulushi.
    let supply = if ctx.supply.is_empty() {
        100.0
    } else {
        let late = ctx.supply.iter().filter(|s| s.late).count();
        (100.0 - late as f64 * 100.0 / ctx.supply.len() as f64).clamp(0.0, 100.0)
    };

    // Hujjatlar: imzolanganlari ulushi.
    let docs = if ctx.required_docs.is_empty() {
        100.0
    } else {
        let signed = ctx.required_docs.iter().filter(|d| d.signed).count();
        signed as f64 * 100.0 / ctx.required_docs.len() as f64
    };

    let quality = ctx.quality_score.clamp(0.0, 100.0);
    let safety = ctx.safety_score.clamp(0.0, 100.0);

    // Vaznlar: xavfsizlik 25, sifat 25, muddat 20, pul 15, ta'minot 10,
    // hujjat 5. Yig'indisi 100.
    let total = safety * 0.25
        + quality * 0.25
        + schedule * 0.20
        + money * 0.15
        + supply * 0.10
        + docs * 0.05;

    ExecutiveScore {
        schedule,
        money,
        quality,
        safety,
        supply,
        docs,
        total: total.clamp(0.0, 100.0),
    }
}

// ================= XI.8, 13, 29, 31. Ombor nazorati =================

/// Ombor bo'yicha bitta e'tiroz (TZ XI.8, 13, 29, 31).
#[derive(Debug, Clone, PartialEq)]
pub enum StockIssue {
    /// Kirim hujjatsiz qilingan.
    IntakeNoDocument { material: String, qty: f64 },
    /// Sertifikatli material partiyasiz kirim qilingan — sertifikat
    /// partiyaga bog'lanadi, partiya bo'lmasa u qaysi materialga tegishli
    /// ekani noma'lum qoladi.
    IntakeNoBatch { material: String },
    /// Kirim buyurtmadagi miqdordan oshgan.
    IntakeOverOrder { material: String, over: f64 },
    /// Ishga berilgan material smeta rasenkasiga bog'lanmagan — sarf
    /// tannarxga tushmaydi.
    NoEstimateLink { material: String, amount: f64 },
    /// Harorat talabi bor material issiq mavsumda ochiq omborda.
    TemperatureRisk { material: String, place: String },
    /// Omborga kirgan va texnikaga berilgan yoqilg'i mos kelmaydi.
    FuelGap { issued: f64, used: f64, diff: f64 },
}

impl StockIssue {
    /// Hisobni buzadigan darajadagi e'tirozmi.
    pub fn severe(&self) -> bool {
        matches!(
            self,
            StockIssue::IntakeOverOrder { .. }
                | StockIssue::FuelGap { .. }
                | StockIssue::TemperatureRisk { .. }
        )
    }
}

/// Yoqilg'i hisobidagi farq shu foizdan oshsa e'tiroz beriladi.
///
/// Nol chegara qo'yish mumkin emas: bakdagi qoldiq va o'lchov aniqligi
/// tufayli kichik farq har doim bo'ladi.
pub const FUEL_GAP_PCT: f64 = 10.0;

/// Harorat talabi tekshiriladigan oy oralig'i (yozgi mavsum).
pub const HOT_MONTHS: (u32, u32) = (6, 8);

/// Ombor nazorati uchun manba.
pub struct StockCtx<'a> {
    pub moves: &'a [StockMove],
    pub materials: &'a [Material],
    pub purchases: &'a [Purchase],
    pub warehouses: &'a [Warehouse],
    pub machine_logs: &'a [MachineLog],
    pub today: NaiveDate,
}

/// TZ XI.8, 13, 29, 31: ombor yozuvlarini tekshiradi.
///
/// To'rt savol: kirim hujjatlanganmi, sarf smetaga bog'langanmi, harorat
/// talabi buzilmayaptimi va yoqilg'i hisobi to'g'ri kelayaptimi.
pub fn stock_control(ctx: &StockCtx) -> Vec<StockIssue> {
    let mut out = Vec::new();
    let name_of = |id: i64| {
        ctx.materials
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.name.clone())
            .unwrap_or_default()
    };

    // ---------- Kirim nazorati (TZ XI.8) ----------
    for m in ctx.moves.iter().filter(|m| matches!(m.kind, MoveKind::In)) {
        let material = ctx.materials.iter().find(|x| x.id == m.material_id);
        if m.document.trim().is_empty() {
            out.push(StockIssue::IntakeNoDocument {
                material: name_of(m.material_id),
                qty: m.qty,
            });
        }
        // Sertifikat muddati yuritiladigan material partiyasiz kelmasligi kerak.
        if m.batch_id.is_none() && material.is_some_and(|x| x.cert_until.is_some()) {
            out.push(StockIssue::IntakeNoBatch {
                material: name_of(m.material_id),
            });
        }
    }

    // Buyurtmadan oshgan kirim: hujjat raqami bo'yicha bog'lanadi.
    for p in ctx.purchases.iter().filter(|p| p.qty > 0.0) {
        let got: f64 = ctx
            .moves
            .iter()
            .filter(|m| {
                matches!(m.kind, MoveKind::In)
                    && !p.number.trim().is_empty()
                    && m.document.contains(p.number.trim())
            })
            .map(|m| m.qty)
            .sum();
        if got > p.qty * 1.001 {
            out.push(StockIssue::IntakeOverOrder {
                material: p.title.clone(),
                over: got - p.qty,
            });
        }
    }

    // ---------- Smeta bilan bog'lanish (TZ XI.13) ----------
    for material in ctx.materials {
        if !material.estimate_code.trim().is_empty() {
            continue;
        }
        let issued: f64 = ctx
            .moves
            .iter()
            .filter(|m| m.material_id == material.id && matches!(m.kind, MoveKind::Out))
            .map(|m| {
                m.qty
                    * if m.price > 0.0 {
                        m.price
                    } else {
                        material.price
                    }
            })
            .sum();
        if issued > 0.0 {
            out.push(StockIssue::NoEstimateLink {
                material: material.name.clone(),
                amount: issued,
            });
        }
    }

    // ---------- Harorat nazorati (TZ XI.29) ----------
    let month = ctx.today.month();
    if month >= HOT_MONTHS.0 && month <= HOT_MONTHS.1 {
        for material in ctx.materials {
            let special = material.special.to_lowercase();
            let needs = ["harorat", "sovuq", "темпер", "холод", "muzlash"]
                .iter()
                .any(|k| special.contains(k));
            if !needs {
                continue;
            }
            // Qaysi omborda turgani oxirgi kirimdan aniqlanadi.
            let place = ctx
                .moves
                .iter()
                .filter(|m| m.material_id == material.id && matches!(m.kind, MoveKind::In))
                .max_by_key(|m| m.date)
                .and_then(|m| m.warehouse_id)
                .and_then(|id| ctx.warehouses.iter().find(|w| w.id == id))
                .filter(|w| w.kind == WarehouseKind::Open);
            if let Some(w) = place {
                out.push(StockIssue::TemperatureRisk {
                    material: material.name.clone(),
                    place: w.name.clone(),
                });
            }
        }
    }

    // ---------- Yoqilg'i hisobi (TZ XI.31) ----------
    // Omborga kirgan yoqilg'i va texnikaga yozilgani solishtiriladi.
    let fuel_ids: Vec<i64> = ctx
        .materials
        .iter()
        .filter(|m| {
            let n = m.name.to_lowercase();
            ["dizel", "benzin", "yoqilg", "дизел", "бензин", "топлив"]
                .iter()
                .any(|k| n.contains(k))
        })
        .map(|m| m.id)
        .collect();
    if !fuel_ids.is_empty() {
        let issued: f64 = ctx
            .moves
            .iter()
            .filter(|m| fuel_ids.contains(&m.material_id) && matches!(m.kind, MoveKind::Out))
            .map(|m| m.qty)
            .sum();
        let used: f64 = ctx.machine_logs.iter().map(|l| l.fuel).sum();
        let base = issued.max(used);
        if base > 0.0 && (issued - used).abs() * 100.0 / base > FUEL_GAP_PCT {
            out.push(StockIssue::FuelGap {
                issued,
                used,
                diff: issued - used,
            });
        }
    }

    // Jiddiylari oldinda.
    out.sort_by_key(|i| !i.severe());
    out
}

// ================= XVI.28, 31, 39, 46. Kunlik ko'rik, operator, park =================

/// Texnika bo'yicha mexanik e'tirozi (TZ XVI.28, 31, 46, XV.18).
#[derive(Debug, Clone, PartialEq)]
pub enum MechIssue {
    /// Bugun ishlagan texnika ko'rikdan o'tmagan.
    NoDailyCheck { machine: String },
    /// Ko'rikda nosozlik topilgan, texnika esa ishlashda.
    FaultButWorking { machine: String, fault: String },
    /// Ko'rikda ruxsat berilmagan, texnika esa ishlagan.
    NotAllowedButUsed { machine: String },
    /// Texnik ko'rik muddati o'tgan.
    InspectionExpired { machine: String, days: i64 },
    /// Rejali texnik xizmat muddati o'tgan (motosoat bo'yicha).
    ServiceOverdue { machine: String, over_hours: f64 },
    /// Operator ko'rsatilmagan — kim boshqargani noma'lum.
    NoOperator { machine: String },
    /// Operatorning ko'targich ishlariga ruxsati yo'q yoki muddati o'tgan.
    OperatorNoPermit { machine: String, operator: String },
}

impl MechIssue {
    /// Texnikani to'xtatishga arziydigan darajadagi e'tirozmi.
    pub fn stop(&self) -> bool {
        matches!(
            self,
            MechIssue::FaultButWorking { .. }
                | MechIssue::NotAllowedButUsed { .. }
                | MechIssue::InspectionExpired { .. }
                | MechIssue::OperatorNoPermit { .. }
        )
    }
}

/// Mexanik kabineti uchun manba.
pub struct MechCtx<'a> {
    pub machines: &'a [Machine],
    pub logs: &'a [MachineLog],
    pub checks: &'a [MachineCheck],
    pub workers: &'a [Worker],
    pub permits: &'a [WorkerPermit],
    pub today: NaiveDate,
}

/// TZ XVI.28, 31, 46: mexanik kabineti — bugungi holat bo'yicha e'tirozlar.
///
/// Ko'rik **yozib qoldirilishi** kerak: og'zaki «hammasi joyida» hodisadan
/// keyin hech narsani isbotlamaydi. Shuning uchun bugun ishlagan, lekin
/// ko'rik yozuvi yo'q texnika alohida ko'rsatiladi.
pub fn mech_issues(ctx: &MechCtx) -> Vec<MechIssue> {
    let key = |s: &str| s.trim().to_lowercase();
    let mut out = Vec::new();

    for m in ctx.machines {
        let worked_today = ctx
            .logs
            .iter()
            .any(|l| l.machine_id == m.id && l.date == ctx.today && l.hours > 0.0);
        let today_check = ctx
            .checks
            .iter()
            .find(|c| c.machine_id == m.id && c.date == ctx.today);

        if worked_today {
            match today_check {
                None => out.push(MechIssue::NoDailyCheck {
                    machine: m.name.clone(),
                }),
                Some(c) => {
                    if !c.allowed {
                        out.push(MechIssue::NotAllowedButUsed {
                            machine: m.name.clone(),
                        });
                    } else if !c.fault.trim().is_empty() {
                        out.push(MechIssue::FaultButWorking {
                            machine: m.name.clone(),
                            fault: c.fault.clone(),
                        });
                    }
                }
            }
        }

        // Texnik ko'rik muddati.
        if let Some(until) = m.inspection_until {
            if until < ctx.today {
                out.push(MechIssue::InspectionExpired {
                    machine: m.name.clone(),
                    days: (ctx.today - until).num_days(),
                });
            }
        }

        // Rejali TX: umumiy motosoat oxirgi TX dan oralig'idan oshdimi.
        if m.service_hours > 0.0 {
            let total: f64 = ctx
                .logs
                .iter()
                .filter(|l| l.machine_id == m.id)
                .map(|l| l.hours)
                .sum();
            let since = total - m.service_done;
            if since > m.service_hours {
                out.push(MechIssue::ServiceOverdue {
                    machine: m.name.clone(),
                    over_hours: since - m.service_hours,
                });
            }
        }

        // Operator (TZ XVI.31): ko'targich texnikasi ruxsat talab qiladi.
        if m.operator.trim().is_empty() {
            if worked_today {
                out.push(MechIssue::NoOperator {
                    machine: m.name.clone(),
                });
            }
        } else if matches!(m.kind, MachineKind::Crane | MachineKind::Lift) {
            let worker = ctx
                .workers
                .iter()
                .find(|w| key(&w.name) == key(&m.operator));
            let ok = worker.is_some_and(|w| {
                ctx.permits.iter().any(|p| {
                    p.worker_id == w.id && p.kind == PermitKind::Lifting && !p.expired(ctx.today)
                })
            });
            if !ok {
                out.push(MechIssue::OperatorNoPermit {
                    machine: m.name.clone(),
                    operator: m.operator.clone(),
                });
            }
        }
    }

    // To'xtatishga arziydiganlari oldinda.
    out.sort_by_key(|i| !i.stop());
    out
}

/// Park bo'yicha bitta texnika holati (TZ XVI.39).
#[derive(Debug, Clone)]
pub struct ParkLine {
    pub machine_id: i64,
    pub name: String,
    pub rented: bool,
    /// Kunlik o'rtacha ishlangan soat (oxirgi davr bo'yicha).
    pub hours_per_day: f64,
    /// Foydalanish koeffitsiyenti, foizda: soat / (kunlar × smena).
    pub usage_pct: f64,
    /// Bo'sh turgan kunlar.
    pub idle_days: i64,
    /// Davr ichida texnikaga ketgan pul: ishlangan soat × soat narxi.
    pub period_cost: f64,
}

/// Park bo'yicha tavsiya.
#[derive(Debug, Clone, PartialEq)]
pub enum ParkAdvice {
    /// O'z texnikasi kam ishlatilyapti — ijaraga berish yoki sotish.
    OwnIdle { name: String, usage_pct: f64 },
    /// Ijara texnikasi kam ishlatilyapti — qaytarish.
    RentedIdle { name: String, usage_pct: f64 },
    /// Ijara texnikasi doim ishlayapti — o'zini olish arzonroq bo'lishi
    /// mumkin.
    RentedBusy { name: String, usage_pct: f64 },
}

/// Foydalanish koeffitsiyenti shu foizdan past bo'lsa — kam ishlatilgan.
pub const PARK_IDLE_PCT: f64 = 35.0;

/// Foydalanish koeffitsiyenti shu foizdan yuqori bo'lsa — doim ishda.
pub const PARK_BUSY_PCT: f64 = 80.0;

/// TZ XVI.39: parkni ko'rib chiqish.
///
/// Tavsiya **qaror emas**: ijaraga olishmi yoki sotib olishmi — bu pul va
/// muddat bo'yicha qaror, dastur esa faqat koeffitsiyentni ko'rsatadi.
/// Shuning uchun tavsiya bitta gapdan oshmaydi.
pub fn park_review(
    machines: &[Machine],
    logs: &[MachineLog],
    today: NaiveDate,
    days: i64,
) -> (Vec<ParkLine>, Vec<ParkAdvice>) {
    let from = today - chrono::Duration::days(days.max(1));
    let mut lines = Vec::new();

    for m in machines {
        let mine: Vec<&MachineLog> = logs
            .iter()
            .filter(|l| l.machine_id == m.id && l.date > from && l.date <= today)
            .collect();
        let hours: f64 = mine.iter().map(|l| l.hours).sum();
        let worked_days = {
            let mut d: Vec<NaiveDate> = mine
                .iter()
                .filter(|l| l.hours > 0.0)
                .map(|l| l.date)
                .collect();
            d.sort_unstable();
            d.dedup();
            d.len() as i64
        };
        let period = days.max(1);
        lines.push(ParkLine {
            machine_id: m.id,
            name: m.name.clone(),
            rented: m.rented,
            hours_per_day: hours / period as f64,
            usage_pct: (hours / (period as f64 * SHIFT_HOURS) * 100.0).clamp(0.0, 100.0),
            idle_days: period - worked_days,
            period_cost: hours * m.hour_rate,
        });
    }

    let mut advice = Vec::new();
    for l in &lines {
        if l.usage_pct < PARK_IDLE_PCT {
            advice.push(if l.rented {
                ParkAdvice::RentedIdle {
                    name: l.name.clone(),
                    usage_pct: l.usage_pct,
                }
            } else {
                ParkAdvice::OwnIdle {
                    name: l.name.clone(),
                    usage_pct: l.usage_pct,
                }
            });
        } else if l.rented && l.usage_pct > PARK_BUSY_PCT {
            advice.push(ParkAdvice::RentedBusy {
                name: l.name.clone(),
                usage_pct: l.usage_pct,
            });
        }
    }

    // Eng kam ishlatilgani oldinda.
    lines.sort_by(|a, b| a.usage_pct.total_cmp(&b.usage_pct));
    (lines, advice)
}

// ================= III.9, 11, 13, 16. Smetaning chuqur tekshiruvi =================

/// Smeta bo'yicha chuqur tekshiruv e'tirozi (TZ III.9, 11, 13, 16).
#[derive(Debug, Clone, PartialEq)]
pub enum DeepIssue {
    /// Kompleks rasenka ichidagi ish alohida ham hisoblangan — ikki marta
    /// to'lash xavfi.
    DoubleCount { pos: i64, inside: i64, amount: f64 },
    /// Bir xil rasenka kodi turli narxda.
    SamePriceCode { code: String, low: f64, high: f64 },
    /// Texnologik ketma-ketlik: bu ish smetada bor, undan oldin
    /// bajarilishi kerak bo'lgan ish esa yo'q.
    MissingPredecessor { pos: i64, task: String },
    /// Marka yoki standart ko'rsatilmagan.
    NoMark { pos: i64, name: String },
    /// Smetadagi narx tijorat taklifidan sezilarli farq qiladi.
    QuoteGap {
        pos: i64,
        estimate: f64,
        quote: f64,
        pct: f64,
    },
}

impl DeepIssue {
    /// Pulga bevosita ta'sir qiladigan e'tirozmi.
    pub fn money(&self) -> bool {
        matches!(
            self,
            DeepIssue::DoubleCount { .. }
                | DeepIssue::SamePriceCode { .. }
                | DeepIssue::QuoteGap { .. }
        )
    }
}

/// Smeta narxi tijorat taklifidan shu foizdan ko'p farq qilsa ko'rsatiladi.
///
/// Chegara kichik bo'lsa har pozitsiya ogohlantirishga aylanadi: narxlar
/// hech qachon aynan bir xil bo'lmaydi.
pub const QUOTE_GAP_PCT: f64 = 15.0;

/// TZ III.9, 11, 13, 16: smetani chuqur tekshiradi.
///
/// To'rt savol: kompleks rasenka ichidagi ish alohida hisoblanmadimi,
/// texnologik ketma-ketlik buzilmadimi, marka ko'rsatilganmi va narx
/// tijorat taklifidan uzoqlashib ketmadimi.
pub fn estimate_deep(
    items: &[EstimateItem],
    tasks: &[Task],
    links: &[crate::model::Link],
    quotes: &[Quote],
) -> Vec<DeepIssue> {
    let norm = |s: &str| s.trim().to_lowercase();
    let mut out = Vec::new();

    // ---------- III.9: kompleks rasenka ichidagi takror ----------
    // Agar bir pozitsiya nomi ikkinchisining nomini to'liq o'z ichiga olsa
    // va ikkalasi bir bo'limda bo'lsa — kichigi kattasi ichida hisoblangan
    // bo'lishi mumkin. Bu **savol**, hukm emas: nomlar tasodifan ham
    // ustma-ust tushadi, shuning uchun uzunligi yetarli nomlar olinadi.
    for big in items {
        let bn = norm(&big.name);
        if bn.chars().count() < 12 {
            continue;
        }
        for small in items {
            if small.id == big.id || small.section != big.section {
                continue;
            }
            let sn = norm(&small.name);
            if sn.chars().count() < 8 || sn.chars().count() >= bn.chars().count() {
                continue;
            }
            if bn.contains(&sn) {
                out.push(DeepIssue::DoubleCount {
                    pos: big.pos,
                    inside: small.pos,
                    amount: small.computed(),
                });
            }
        }
    }

    // Bir xil kod turli narxda: rasenka bitta bo'lsa narx ham bitta bo'ladi.
    let mut codes: Vec<(String, f64, f64)> = Vec::new();
    for i in items
        .iter()
        .filter(|i| !i.code.trim().is_empty() && i.price > 0.0)
    {
        match codes.iter_mut().find(|(c, _, _)| norm(c) == norm(&i.code)) {
            Some(e) => {
                e.1 = e.1.min(i.price);
                e.2 = e.2.max(i.price);
            }
            None => codes.push((i.code.clone(), i.price, i.price)),
        }
    }
    for (code, low, high) in codes {
        if low > 0.0 && (high - low) / low > 0.001 {
            out.push(DeepIssue::SamePriceCode { code, low, high });
        }
    }

    // ---------- III.11: texnologik ketma-ketlik ----------
    // Ish smetada bor, undan oldin turishi kerak bo'lgan ish esa yo'q.
    let priced = |task_id: i64| items.iter().any(|i| i.task_id == Some(task_id));
    for i in items {
        let Some(tid) = i.task_id else { continue };
        for l in links.iter().filter(|l| l.succ == tid) {
            if priced(l.pred) {
                continue;
            }
            // Oldingi ish umuman GPR da bo'lmasa — bu boshqa muammo va u
            // grafik tekshiruvida ko'rinadi.
            let Some(pred) = tasks.iter().find(|t| t.id == l.pred) else {
                continue;
            };
            out.push(DeepIssue::MissingPredecessor {
                pos: i.pos,
                task: format!("{} {}", pred.wbs, pred.name),
            });
        }
    }

    // ---------- III.13: marka va xarakteristika ----------
    // Konstruksiya va tarmoq ishlarida marka yoki standart bo'lishi shart:
    // «beton quyish» degan pozitsiyaga narx qo'yib bo'lmaydi, «B25 beton
    // quyish» ga esa bo'ladi.
    for i in items {
        let needs_mark = matches!(
            i.section,
            Section::Kj | Section::Km | Section::Vk | Section::Ov | Section::Eom | Section::Ss
        );
        if !needs_mark {
            continue;
        }
        let n = i.name.clone();
        let has_digit = n.chars().any(|c| c.is_ascii_digit());
        let lower = norm(&n);
        let has_standard = ["gost", "гост", "shnq", "шнк", "sn", "din", "iso"]
            .iter()
            .any(|k| lower.contains(k));
        if !has_digit && !has_standard {
            out.push(DeepIssue::NoMark {
                pos: i.pos,
                name: n,
            });
        }
    }

    // ---------- III.16: tijorat taklifi bilan solishtirish ----------
    for i in items.iter().filter(|i| i.price > 0.0) {
        let best = quotes
            .iter()
            .filter(|q| q.price > 0.0 && norm(&q.title) == norm(&i.name))
            .map(|q| q.price)
            .fold(f64::INFINITY, f64::min);
        if !best.is_finite() || best <= 0.0 {
            continue;
        }
        let pct = (i.price - best) * 100.0 / best;
        if pct.abs() > QUOTE_GAP_PCT {
            out.push(DeepIssue::QuoteGap {
                pos: i.pos,
                estimate: i.price,
                quote: best,
                pct,
            });
        }
    }

    // Pulga tegishlilari oldinda.
    out.sort_by_key(|i| !i.money());
    out
}

// ================= VII.30, 32, VIII.24, IV.7. Loyiha versiyalari =================

/// Loyiha hujjati versiyasi bo'yicha e'tiroz (TZ VII.30, 32, VIII.24).
#[derive(Debug, Clone, PartialEq)]
pub enum VersionIssue {
    /// Yangi versiya kelgan, lekin qurilishga topshirilmagan — obyektda
    /// hali eski chizma bo'yicha ishlanyapti.
    NotIssued { name: String, revision: String },
    /// Yangi versiya topshirilgach, unga tegishli ish allaqachon
    /// bajarilgan: qurilishdagi holat yangi chizmaga mos kelmasligi mumkin.
    WorkDoneBefore {
        name: String,
        revision: String,
        tasks: usize,
    },
    /// O'zgartirish izohi yozilmagan — nima o'zgargani noma'lum.
    NoChangeNote { name: String, revision: String },
    /// O'zgartirish belgisi ko'rsatilmagan.
    NoRevision { name: String },
    /// Eski versiya arxivga tushmagan: bir bo'limda ikkita amaldagi
    /// hujjat turibdi va qaysi biri to'g'ri ekani noma'lum.
    TwoActive { name: String, count: usize },
}

impl VersionIssue {
    /// Obyektda noto'g'ri chizma bo'yicha ishlash xavfi bormi.
    pub fn risky(&self) -> bool {
        matches!(
            self,
            VersionIssue::NotIssued { .. }
                | VersionIssue::WorkDoneBefore { .. }
                | VersionIssue::TwoActive { .. }
        )
    }
}

/// TZ VII.30, 32: loyiha hujjatlari versiyalarini nazorat qiladi.
///
/// Asosiy xavf bitta: **obyektda eski chizma bo'yicha ishlash**. Shuning
/// uchun tekshiruv ikki tomonga qaraydi — yangi versiya ishga
/// berilganmi va berilgan bo'lsa, unga tegishli ish undan **oldin**
/// bajarilib ketmaganmi.
pub fn version_issues(docs: &[Document], tasks: &[Task], today: NaiveDate) -> Vec<VersionIssue> {
    let mut out = Vec::new();

    for d in docs {
        // Arxivga tushgan versiyadan hech narsa talab qilinmaydi.
        if d.superseded(docs) {
            continue;
        }

        if d.version > 1 {
            if d.issued.is_none() {
                out.push(VersionIssue::NotIssued {
                    name: d.name.clone(),
                    revision: d.label(),
                });
            } else if d.change_note.trim().is_empty() {
                out.push(VersionIssue::NoChangeNote {
                    name: d.name.clone(),
                    revision: d.label(),
                });
            }
            if d.revision.trim().is_empty() {
                out.push(VersionIssue::NoRevision {
                    name: d.name.clone(),
                });
            }

            // TZ VII.32: yangi chizma topshirilgach, shu bo'limdagi qaysi
            // ishlar undan oldin tugatilgan. Ular qayta ko'rilishi kerak
            // bo'lishi mumkin — bu ogohlantirish, taqiq emas.
            if let Some(issued) = d.issued {
                let done_before = tasks
                    .iter()
                    .filter(|t| t.section == d.section)
                    .filter(|t| t.fact_end.is_some_and(|e| e < issued))
                    .count();
                if done_before > 0 && issued <= today {
                    out.push(VersionIssue::WorkDoneBefore {
                        name: d.name.clone(),
                        revision: d.label(),
                        tasks: done_before,
                    });
                }
            }
        }
    }

    // Bir nomdagi ikkita amaldagi hujjat: eskisi arxivga tushmagan.
    let key = |s: &str| s.trim().to_lowercase();
    let mut names: Vec<String> = Vec::new();
    for d in docs {
        if !names.iter().any(|n| key(n) == key(&d.name)) {
            names.push(d.name.clone());
        }
    }
    for name in names {
        let active = docs
            .iter()
            .filter(|d| key(&d.name) == key(&name) && !d.superseded(docs))
            .count();
        if active > 1 {
            out.push(VersionIssue::TwoActive {
                name,
                count: active,
            });
        }
    }

    // Xavflilari oldinda.
    out.sort_by_key(|i| !i.risky());
    out
}

/// Ikki versiya orasidagi farq (TZ VIII.24).
#[derive(Debug, Clone, Default)]
pub struct VersionDiff {
    pub from_label: String,
    pub to_label: String,
    /// Varaq soni o'zgarishi.
    pub sheets_from: i64,
    pub sheets_to: i64,
    /// Loyihachi yozgan o'zgartirish izohi.
    pub note: String,
    /// Yangi versiya topshirilgan sana.
    pub issued: Option<NaiveDate>,
    /// Shu bo'limda yangi versiyagacha tugatilgan ishlar.
    pub tasks_before: Vec<i64>,
}

/// TZ VIII.24: ikki versiyani solishtiradi.
///
/// Dastur chizmaning **ichini** o'qimaydi: PDF va DWG ni tanish tashqi
/// kutubxonani talab qiladi. Shuning uchun solishtirish hujjat
/// kartochkasidagi ma'lumotga tayanadi — varaq soni, o'zgartirish belgisi
/// va loyihachi yozgan izoh. Bu kam, lekin **haqiqiy**: o'ylab topilgan
/// «farqlar ro'yxati» dan ko'ra foydali.
pub fn version_diff(old: &Document, new: &Document, tasks: &[Task]) -> VersionDiff {
    VersionDiff {
        from_label: old.label(),
        to_label: new.label(),
        sheets_from: old.sheets,
        sheets_to: new.sheets,
        note: new.change_note.clone(),
        issued: new.issued,
        tasks_before: match new.issued {
            None => Vec::new(),
            Some(issued) => tasks
                .iter()
                .filter(|t| t.section == new.section)
                .filter(|t| t.fact_end.is_some_and(|e| e < issued))
                .map(|t| t.id)
                .collect(),
        },
    }
}

// ================= XIV.37, V.17, V.24. Haftalik sifat, kunlik xulosa, ariza =================

/// Haftalik sifat hisoboti (TZ XIV.37).
#[derive(Debug, Clone, Default)]
pub struct QualityWeek {
    pub from: NaiveDate,
    pub to: NaiveDate,
    /// Haftada o'tkazilgan tekshiruvlar.
    pub checks: usize,
    pub passed: usize,
    pub failed: usize,
    /// Haftada ochilgan va yopilgan nuqsonlar.
    pub defects_opened: usize,
    pub defects_closed: usize,
    /// Hafta oxiriga ochiq qolgan nuqsonlar (butun loyiha bo'yicha).
    pub defects_open: usize,
    /// Muddati o'tgan nuqsonlar.
    pub overdue: usize,
    /// Eng ko'p takrorlangan nuqson sabablari.
    pub top_defects: Vec<(String, usize)>,
    /// Hafta oxiridagi sifat balli — sifat modulidagi bilan bir xil qoida.
    pub score: f64,
}

/// TZ XIV.37: hafta bo'yicha sifat xulosasi.
///
/// Ball [`quality_score`] dan olinadi — bu yerda qayta hisoblanmaydi,
/// shuning uchun hisobotdagi son sifat ekranidagi bilan bir xil bo'ladi.
pub fn quality_week(quality: &[QualityCheck], today: NaiveDate) -> QualityWeek {
    let from = today - chrono::Duration::days(7);
    let in_week = |d: NaiveDate| d > from && d <= today;

    let week: Vec<&QualityCheck> = quality.iter().filter(|q| in_week(q.date)).collect();
    let has_defect = |q: &QualityCheck| !q.defect.trim().is_empty();

    // Eng ko'p takrorlangan nuqson matnlari.
    let mut causes: Vec<(String, usize)> = Vec::new();
    for q in quality.iter().filter(|q| has_defect(q)) {
        let key = q.defect.trim().to_lowercase();
        match causes.iter_mut().find(|(c, _)| *c == key) {
            Some(e) => e.1 += 1,
            None => causes.push((q.defect.trim().to_string(), 1)),
        }
    }
    causes.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    causes.truncate(3);

    QualityWeek {
        from,
        to: today,
        checks: week.len(),
        passed: week
            .iter()
            .filter(|q| q.result == QualityResult::Pass)
            .count(),
        failed: week
            .iter()
            .filter(|q| q.result == QualityResult::Fail)
            .count(),
        defects_opened: week.iter().filter(|q| has_defect(q)).count(),
        defects_closed: quality
            .iter()
            .filter(|q| q.fixed_at.is_some_and(in_week))
            .count(),
        defects_open: quality
            .iter()
            .filter(|q| has_defect(q) && q.fixed_at.is_none())
            .count(),
        overdue: quality
            .iter()
            .filter(|q| {
                has_defect(q) && q.fixed_at.is_none() && q.deadline.is_some_and(|d| d < today)
            })
            .count(),
        top_defects: causes,
        score: quality_score(quality, today).score,
    }
}

/// Direktor uchun kunlik xulosa (TZ V.24).
#[derive(Debug, Clone, Default)]
pub struct DayReport {
    pub day: NaiveDate,
    /// Bugun ketayotgan ishlar.
    pub running: usize,
    /// Jurnalga yozilgan hajm bo'yicha ishlar soni.
    pub logged: usize,
    /// Tabelda belgilangan odamlar va soatlar.
    pub workers: usize,
    pub hours: f64,
    /// Bugun ishlagan texnika.
    pub machines: usize,
    /// Bugungi material chiqimi qiymati.
    pub material_cost: f64,
    /// Bugun ochilgan xavfsizlik va sifat holatlari.
    pub safety_new: usize,
    pub quality_new: usize,
    /// Bugun imzolangan ijro hujjatlari.
    pub docs_signed: usize,
    /// Kun yakuni tekshiruvidagi to'siqlar soni.
    pub blockers: usize,
}

/// TZ V.24: kun bo'yicha bir ekranli xulosa.
///
/// Hisobot **yangi son o'ylab topmaydi**: har qatori bugungi yozuvlardan
/// sanaladi. To'siqlar soni [`day_close`] dan olinadi — prorab ekranidagi
/// bilan bir xil.
#[allow(clippy::too_many_arguments)]
pub fn day_report(
    day: NaiveDate,
    running: usize,
    journal: &[JournalEntry],
    timesheet: &[TimesheetEntry],
    machine_logs: &[MachineLog],
    moves: &[StockMove],
    materials: &[Material],
    safety: &[SafetyEvent],
    quality: &[QualityCheck],
    docs: &[ExecDoc],
    blockers: usize,
) -> DayReport {
    let today_hours: Vec<&TimesheetEntry> = timesheet
        .iter()
        .filter(|e| e.date == day && e.hours > 0.0)
        .collect();
    DayReport {
        day,
        running,
        logged: journal
            .iter()
            .filter(|j| j.date == day && j.volume > 0.0)
            .count(),
        workers: today_hours.len(),
        hours: today_hours.iter().map(|e| e.hours).sum(),
        machines: machine_logs
            .iter()
            .filter(|l| l.date == day && l.hours > 0.0)
            .count(),
        material_cost: moves
            .iter()
            .filter(|m| m.date == day && matches!(m.kind, MoveKind::Out))
            .map(|m| {
                let price = if m.price > 0.0 {
                    m.price
                } else {
                    materials
                        .iter()
                        .find(|x| x.id == m.material_id)
                        .map(|x| x.price)
                        .unwrap_or(0.0)
                };
                m.qty * price
            })
            .sum(),
        safety_new: safety.iter().filter(|s| s.date == day).count(),
        quality_new: quality.iter().filter(|q| q.date == day).count(),
        docs_signed: docs
            .iter()
            .filter(|d| d.date == day && d.status == ExecDocStatus::Signed)
            .count(),
        blockers,
    }
}

/// Jurnaldan chiqadigan ariza taklifi (TZ V.17).
#[derive(Debug, Clone, PartialEq)]
pub struct JournalRequest {
    pub task_id: i64,
    pub material_id: i64,
    /// Qolgan hajmga norma bo'yicha kerak bo'lgan miqdor.
    pub need: f64,
    /// Omborda erkin qoldiq.
    pub available: f64,
    /// So'raladigan miqdor: kerak minus qoldiq.
    pub qty: f64,
}

/// TZ V.17: bugungi jurnal yozuvidan ariza taklif qiladi.
///
/// Mantiq oddiy va tekshiriladigan: jurnalda hajm yozilgan ish bo'yicha
/// **qolgan** hajmga norma qo'llanadi, natijadan ombordagi erkin qoldiq
/// ayriladi. Qoldiq yetsa — ariza taklif qilinmaydi, chunki keraksiz
/// ariza tartibni buzadi.
pub fn journal_requests(
    journal: &[JournalEntry],
    norms: &[MaterialNorm],
    tasks: &[Task],
    stock: &[StockLine],
    day: NaiveDate,
) -> Vec<JournalRequest> {
    let mut out: Vec<JournalRequest> = Vec::new();
    for j in journal.iter().filter(|j| j.date == day && j.volume > 0.0) {
        let Some(task_id) = j.task_id else { continue };
        let Some(task) = tasks.iter().find(|t| t.id == task_id) else {
            continue;
        };
        let left = task.volume * (100.0 - task.progress.clamp(0.0, 100.0)) / 100.0;
        if left <= 0.0 {
            continue;
        }
        for n in norms.iter().filter(|n| n.task_id == task_id) {
            let need = n.per_unit * left;
            let available = stock
                .iter()
                .find(|l| l.material_id == n.material_id)
                .map_or(0.0, |l| l.available);
            let qty = need - available;
            if qty <= 0.0001 {
                continue;
            }
            // Bir material bo'yicha ikki marta taklif qilmaymiz.
            if out
                .iter()
                .any(|r| r.task_id == task_id && r.material_id == n.material_id)
            {
                continue;
            }
            out.push(JournalRequest {
                task_id,
                material_id: n.material_id,
                need,
                available,
                qty,
            });
        }
    }
    // Eng katta ehtiyoj oldinda.
    out.sort_by(|a, b| b.qty.total_cmp(&a.qty));
    out
}

// ================= XIII.3, 12, 28. Obyektlar, grafik, ko'chirish =================

/// Bitta obyekt bo'yicha xodim yakuni (TZ XIII.3).
#[derive(Debug, Clone)]
pub struct ObjectStaff {
    pub project_id: i64,
    /// Faol ishchilar soni.
    pub workers: usize,
    /// Davr ichida yozilgan soat.
    pub hours: f64,
    /// Bo'sh turish soatlari.
    pub idle_hours: f64,
    /// Ish haqi fondi.
    pub payroll: f64,
    /// Bir ishchiga to'g'ri keladigan o'rtacha soat.
    pub hours_per_worker: f64,
}

/// TZ XIII.3: obyektlar kesimida xodim va soat.
///
/// Ishchi obyektga biriktirilgan, shuning uchun «obyektlar bo'yicha»
/// ko'rinish shunchaki guruhlash emas: bitta ishchi bir vaqtda ikki
/// obyektda bo'la olmaydi va bu hisobda ochiq ko'rinadi.
pub fn object_staff(
    rows: &[(i64, Vec<Worker>, Vec<TimesheetEntry>)],
    from: NaiveDate,
    to: NaiveDate,
) -> Vec<ObjectStaff> {
    let mut out = Vec::new();
    for (pid, workers, timesheet) in rows {
        let active: Vec<&Worker> = workers.iter().filter(|w| w.active).collect();
        let period: Vec<&TimesheetEntry> = timesheet
            .iter()
            .filter(|e| e.date >= from && e.date <= to)
            .collect();
        let hours: f64 = period
            .iter()
            .filter(|e| e.kind == DayKind::Work)
            .map(|e| e.hours)
            .sum();
        let idle: f64 = period
            .iter()
            .filter(|e| e.kind == DayKind::Downtime)
            .map(|e| e.hours)
            .sum();
        let payroll: f64 = period
            .iter()
            .filter(|e| e.kind.paid())
            .map(|e| {
                let rate = active
                    .iter()
                    .find(|w| w.id == e.worker_id)
                    .map(|w| w.hourly_rate)
                    .unwrap_or(0.0);
                e.hours * rate * e.shift.rate()
            })
            .sum();
        out.push(ObjectStaff {
            project_id: *pid,
            workers: active.len(),
            hours,
            idle_hours: idle,
            payroll,
            hours_per_worker: if active.is_empty() {
                0.0
            } else {
                hours / active.len() as f64
            },
        });
    }
    // Eng ko'p odam turgan obyekt oldinda.
    out.sort_by_key(|o| std::cmp::Reverse(o.workers));
    out
}

/// Ish grafigi: qaysi kunlar ish kuni (TZ XIII.12).
#[derive(Debug, Clone, PartialEq)]
pub struct WorkSchedule {
    /// Haftaning ish kunlari: 1 — dushanba, 7 — yakshanba.
    pub work_days: Vec<u32>,
    /// Kunlik smena soati.
    pub shift_hours: f64,
}

impl Default for WorkSchedule {
    fn default() -> Self {
        // Qurilishda odatiy grafik — olti kunlik ish haftasi.
        WorkSchedule {
            work_days: vec![1, 2, 3, 4, 5, 6],
            shift_hours: 8.0,
        }
    }
}

impl WorkSchedule {
    /// Shu kun grafik bo'yicha ish kunimi.
    pub fn is_work_day(&self, day: NaiveDate) -> bool {
        use chrono::Datelike;
        self.work_days
            .contains(&(day.weekday().number_from_monday()))
    }

    /// Davr ichidagi ish kunlari soni.
    pub fn work_days_in(&self, from: NaiveDate, to: NaiveDate) -> i64 {
        let mut n = 0;
        let mut d = from;
        while d <= to {
            if self.is_work_day(d) {
                n += 1;
            }
            d += chrono::Duration::days(1);
        }
        n
    }
}

/// Grafik bo'yicha e'tiroz (TZ XIII.12).
#[derive(Debug, Clone, PartialEq)]
pub enum ScheduleIssue {
    /// Dam olish kunida ish yozilgan.
    WorkOnRestDay { day: NaiveDate, workers: usize },
    /// Ish kunida hech kim belgilanmagan.
    EmptyWorkDay { day: NaiveDate },
    /// Smena soati grafikdagidan sezilarli ko'p.
    OverShift { day: NaiveDate, hours: f64 },
}

/// TZ XIII.12: tabelni ish grafigi bilan solishtiradi.
///
/// Dam olish kunidagi ish **taqiq emas**: qurilishda bu bo'ladi. Lekin u
/// ko'rinib turishi kerak, chunki bunday kunga haq boshqacha to'lanadi.
pub fn schedule_issues(
    timesheet: &[TimesheetEntry],
    schedule: &WorkSchedule,
    from: NaiveDate,
    to: NaiveDate,
) -> Vec<ScheduleIssue> {
    let mut out = Vec::new();
    let mut day = from;
    while day <= to {
        let rows: Vec<&TimesheetEntry> = timesheet
            .iter()
            .filter(|e| e.date == day && e.hours > 0.0)
            .collect();
        if schedule.is_work_day(day) {
            if rows.is_empty() {
                out.push(ScheduleIssue::EmptyWorkDay { day });
            } else {
                let max = rows.iter().map(|e| e.hours).fold(0.0_f64, f64::max);
                if max > schedule.shift_hours + 2.0 {
                    out.push(ScheduleIssue::OverShift { day, hours: max });
                }
            }
        } else if !rows.is_empty() {
            out.push(ScheduleIssue::WorkOnRestDay {
                day,
                workers: rows.len(),
            });
        }
        day += chrono::Duration::days(1);
    }
    out
}

// ================= IV.7, 24. Ijro sxemalari va mualliflik nazorati =================

/// Ijro sxemasi holati (TZ IV.7).
#[derive(Debug, Clone)]
pub struct SchemeStatus {
    pub doc_id: i64,
    pub number: String,
    pub task_id: Option<i64>,
    /// O'lchangan nuqtalar soni.
    pub points: usize,
    /// Dopuskdan chiqqan nuqtalar.
    pub out_of_tolerance: usize,
    /// Eng katta chetlanish.
    pub max_deviation: f64,
    pub unit: String,
}

impl SchemeStatus {
    /// Sxema imzolashga tayyormi: o'lchov bor va hammasi dopusk ichida.
    pub fn ready(&self) -> bool {
        self.points > 0 && self.out_of_tolerance == 0
    }
}

/// TZ IV.7: ijro sxemasi geodezik o'lchovga tayanadi.
///
/// Sxema — bu «qanday qurildi» degan hujjat, shuning uchun uning ortida
/// **o'lchangan nuqtalar** turishi kerak. O'lchovsiz sxema imzolanmaydi:
/// unda tasdiqlanadigan narsa yo'q.
pub fn scheme_status(
    docs: &[ExecDoc],
    points: &[GeodesyPoint],
    inspections: &[Inspection],
) -> Vec<SchemeStatus> {
    let mut out = Vec::new();
    for d in docs.iter().filter(|d| d.kind == ExecDocKind::Scheme) {
        // Nuqtalar ishga to'g'ridan-to'g'ri yoki tekshiruv orqali bog'lanadi.
        let mine: Vec<&GeodesyPoint> = points
            .iter()
            .filter(|p| {
                d.task_id.is_some()
                    && p.inspection_id.is_some_and(|iid| {
                        inspections
                            .iter()
                            .any(|i| i.id == iid && i.task_id == d.task_id)
                    })
            })
            .collect();

        let worst = mine
            .iter()
            .map(|p| p.deviation().abs())
            .fold(0.0_f64, f64::max);
        out.push(SchemeStatus {
            doc_id: d.id,
            number: d.number.clone(),
            task_id: d.task_id,
            points: mine.len(),
            out_of_tolerance: mine.iter().filter(|p| !p.within()).count(),
            max_deviation: worst,
            unit: mine.first().map(|p| p.unit.clone()).unwrap_or_default(),
        });
    }
    // Tayyor bo'lmaganlari oldinda.
    out.sort_by_key(|s| (s.ready(), s.doc_id));
    out
}

/// Mualliflik nazorati kabinetidagi bitta ish (TZ IV.24).
#[derive(Debug, Clone, PartialEq)]
pub enum AuthorTask {
    /// Loyihachiga yo'naltirilgan nomuvofiqlik.
    Issue { code: String, title: String },
    /// Qaror kutayotgan loyiha o'zgarishi.
    Change { number: String, days: i64 },
    /// Yangi versiya chiqarilgan, lekin qurilishga topshirilmagan.
    Version { name: String, revision: String },
    /// Texnik nazorat tekshiruvi salbiy va loyiha yechimiga tegishli.
    Inspection { number: String, days: i64 },
}

impl AuthorTask {
    /// Muddati o'tib ketgan ish.
    pub fn late(&self) -> bool {
        match self {
            AuthorTask::Change { days, .. } | AuthorTask::Inspection { days, .. } => {
                *days > DECISION_LIMIT_DAYS
            }
            _ => false,
        }
    }
}

/// TZ IV.24: loyihachi javob berishi kerak bo'lgan ishlar.
///
/// Kabinet **yangi ma'lumot yaratmaydi**: u boshqa modullardagi
/// yozuvlardan loyihachiga tegishlilarini yig'adi. Shuning uchun bu
/// yerdagi har qator o'z ekranida ham turadi.
pub fn author_supervision(
    issues: &[Issue],
    changes: &[ContractChange],
    docs: &[Document],
    inspections: &[Inspection],
    today: NaiveDate,
) -> Vec<AuthorTask> {
    let mut out = Vec::new();
    let designer = crate::i18n::t("role_designer").to_lowercase();

    for i in issues.iter().filter(|i| {
        matches!(i.status, IssueStatus::Open | IssueStatus::InWork)
            && i.responsible.to_lowercase().contains(&designer)
    }) {
        out.push(AuthorTask::Issue {
            code: i.code.clone(),
            title: i.title.clone(),
        });
    }

    for c in changes
        .iter()
        .filter(|c| matches!(c.status, ChangeStatus::Draft | ChangeStatus::Sent))
    {
        out.push(AuthorTask::Change {
            number: c.number.clone(),
            days: (today - c.date).num_days().max(0),
        });
    }

    for d in docs.iter().filter(|d| d.version > 1 && d.issued.is_none()) {
        if d.superseded(docs) {
            continue;
        }
        out.push(AuthorTask::Version {
            name: d.name.clone(),
            revision: d.label(),
        });
    }

    for i in inspections
        .iter()
        .filter(|i| i.result == InspectionResult::Fail && i.fixed_at.is_none())
    {
        // Faqat loyiha yechimiga tegishli tekshiruvlar: geodeziya va
        // yashirin ishlar odatda loyihachining javobini talab qiladi.
        if !matches!(i.kind, InspectionKind::Geodesy | InspectionKind::Hidden) {
            continue;
        }
        let day = i.done.unwrap_or(i.planned);
        out.push(AuthorTask::Inspection {
            number: i.number.clone(),
            days: (today - day).num_days().max(0),
        });
    }

    // Muddati o'tganlari oldinda.
    out.sort_by_key(|a| !a.late());
    out
}

// ================= XVII.3, 7, 10. Sabab va prognoz =================

/// Ishning kechikish sababi (TZ XVII.7).
///
/// Sabab **taxmin qilinmaydi**: u faqat bazadagi yozuvdan chiqadi. Hech
/// bir yozuv sababni ko'rsatmasa — `Unknown`, va ekranda shu ochiq
/// yoziladi. O'ylab topilgan sabab noto'g'ri qarorga olib keladi.
#[derive(Debug, Clone, PartialEq)]
pub enum DelayCause {
    /// Material yetishmaydi.
    MaterialShort { material: String, short: f64 },
    /// Ishga hech kim yozilmagan.
    NoCrew,
    /// Sifat to'sig'i: bartaraf etilmagan nuqson.
    QualityBlock { defects: usize },
    /// Talab qilinadigan hujjat imzolanmagan.
    WaitingDocs { missing: usize },
    /// Texnika ta'mirda.
    MachineDown { machine: String },
    /// Oldingi ish kechikkan.
    PredecessorLate { task: String, days: i64 },
    /// Yozuvlardan sabab topilmadi.
    Unknown,
}

impl DelayCause {
    /// Sabab bartaraf etilishi mumkinmi — ya'ni aniq amal bormi.
    pub fn actionable(&self) -> bool {
        !matches!(self, DelayCause::Unknown)
    }

    /// Sabab qaysi ekranda yopiladi.
    pub fn screen(&self) -> crate::app::Screen {
        use crate::app::Screen as S;
        match self {
            DelayCause::MaterialShort { .. } => S::Warehouse,
            DelayCause::NoCrew => S::Timesheet,
            DelayCause::QualityBlock { .. } => S::Quality,
            DelayCause::WaitingDocs { .. } => S::ExecDocs,
            DelayCause::MachineDown { .. } => S::Machines,
            DelayCause::PredecessorLate { .. } | DelayCause::Unknown => S::Gantt,
        }
    }
}

/// Kechikkan ish va uning sababi.
#[derive(Debug, Clone)]
pub struct TaskDelay {
    pub task_id: i64,
    /// Necha kun kechikkan.
    pub days: i64,
    pub cause: DelayCause,
    /// Kechikishning kunlik narxi: shu ishga yozilgan brigadaning
    /// bir kunlik ish haqi. Brigada yozilmagan bo'lsa — nol va bu
    /// «narx noma'lum» degani, «nol» degani emas.
    pub daily_cost: f64,
}

impl TaskDelay {
    /// Butun kechikishning taxminiy narxi.
    pub fn cost(&self) -> f64 {
        self.daily_cost * self.days.max(0) as f64
    }
}

/// Kechikish sababini topish uchun manba.
pub struct DelayCtx<'a> {
    pub tasks: &'a [Task],
    pub links: &'a [crate::model::Link],
    pub overdue: &'a [i64],
    pub readiness: &'a [Readiness],
    pub materials: &'a [Material],
    pub blocks: &'a [TaskBlock],
    pub required: &'a [RequiredDoc],
    pub timesheet: &'a [TimesheetEntry],
    pub workers: &'a [Worker],
    pub machines: &'a [Machine],
    pub machine_logs: &'a [MachineLog],
    pub today: NaiveDate,
}

/// TZ XVII.7: nega kechikdi degan savolga javob.
///
/// Sabablar **tartib bilan** tekshiriladi: avval eng aniqlari (material
/// yetishmasligi, sifat to'sig'i), keyin umumiylari. Birinchi mos kelgani
/// olinadi — bir ishga beshta sabab yozib qo'yish javob emas.
pub fn delay_causes(ctx: &DelayCtx) -> Vec<TaskDelay> {
    let mut out = Vec::new();

    for id in ctx.overdue {
        let Some(task) = ctx.tasks.iter().find(|t| t.id == *id) else {
            continue;
        };
        let plan_end = task.plan_start + chrono::Duration::days(task.duration.max(0));
        let days = (ctx.today - plan_end).num_days().max(0);

        // 1. Material yetishmaydi — eng aniq va eng tez tuzatiladigan sabab.
        let cause = if let Some(r) = ctx.readiness.iter().find(|r| r.task_id == *id) {
            DelayCause::MaterialShort {
                material: ctx
                    .materials
                    .iter()
                    .find(|m| m.id == r.material_id)
                    .map(|m| m.name.clone())
                    .unwrap_or_default(),
                short: r.short,
            }
        }
        // 2. Sifat to'sig'i.
        else if let Some(b) = ctx
            .blocks
            .iter()
            .find(|b| b.task_id == *id && b.open_defects > 0)
        {
            DelayCause::QualityBlock {
                defects: b.open_defects,
            }
        }
        // 3. Hujjat kutilyapti.
        else if ctx
            .required
            .iter()
            .filter(|r| r.task_id == *id && !r.signed)
            .count()
            > 0
        {
            DelayCause::WaitingDocs {
                missing: ctx
                    .required
                    .iter()
                    .filter(|r| r.task_id == *id && !r.signed)
                    .count(),
            }
        }
        // 4. Ishga hech kim yozilmagan.
        else if !ctx
            .timesheet
            .iter()
            .any(|e| e.task_id == Some(*id) && e.hours > 0.0)
        {
            DelayCause::NoCrew
        }
        // 5. Texnika ta'mirda.
        else if let Some(m) = ctx
            .machine_logs
            .iter()
            .filter(|l| l.task_id == Some(*id))
            .filter_map(|l| ctx.machines.iter().find(|m| m.id == l.machine_id))
            .find(|m| m.status == MachineStatus::Repair)
        {
            DelayCause::MachineDown {
                machine: m.name.clone(),
            }
        }
        // 6. Oldingi ish kechikkan.
        else if let Some((pred, pred_days)) = ctx
            .links
            .iter()
            .filter(|l| l.succ == *id)
            .filter_map(|l| ctx.tasks.iter().find(|t| t.id == l.pred))
            .filter(|p| p.progress < 99.999 && p.fact_end.is_none())
            .map(|p| {
                let end = p.plan_start + chrono::Duration::days(p.duration.max(0));
                (p, (ctx.today - end).num_days().max(0))
            })
            .max_by_key(|(_, d)| *d)
        {
            DelayCause::PredecessorLate {
                task: format!("{} {}", pred.wbs, pred.name),
                days: pred_days,
            }
        } else {
            DelayCause::Unknown
        };

        // Kunlik narx: shu ishga yozilgan odamlarning kunlik haqi.
        let mut ids: Vec<i64> = ctx
            .timesheet
            .iter()
            .filter(|e| e.task_id == Some(*id) && e.hours > 0.0)
            .map(|e| e.worker_id)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        let daily_cost: f64 = ids
            .iter()
            .filter_map(|w| ctx.workers.iter().find(|x| x.id == *w))
            .map(|w| w.hourly_rate * 8.0)
            .sum();

        out.push(TaskDelay {
            task_id: *id,
            days,
            cause,
            daily_cost,
        });
    }

    // Eng qimmat kechikish oldinda; narx noma'lum bo'lsa kun bo'yicha.
    out.sort_by(|a, b| b.cost().total_cmp(&a.cost()).then(b.days.cmp(&a.days)));
    out
}

/// Prognoz turi (TZ XVII.10).
#[derive(Debug, Clone, PartialEq)]
pub enum RiskKind {
    /// Muddat siljiydi.
    ScheduleSlip { days: i64 },
    /// Material tugaydi.
    StockOut { material: String, days: i64 },
    /// Kelishilmagan o'zgarishlar summasi byudjetga tushadi.
    PendingMoney { amount: f64 },
    /// Sifat balli pasaymoqda.
    QualityDrop { open: usize, overdue: usize },
    /// Xavfsizlik holatlari yopilmayapti.
    SafetyOpen { count: usize },
}

/// Bitta prognoz qatori.
#[derive(Debug, Clone)]
pub struct RiskLine {
    pub kind: RiskKind,
    /// Qachon yuz berishi kutilyapti. Sana noma'lum bo'lsa — `None`.
    pub when: Option<NaiveDate>,
    /// Qanchalik jiddiy: 0-100.
    pub weight: f64,
}

impl RiskLine {
    /// E'tibor talab qiladigan daraja.
    pub fn high(&self) -> bool {
        self.weight >= 60.0
    }

    /// Qaysi ekranda ko'riladi.
    pub fn screen(&self) -> crate::app::Screen {
        use crate::app::Screen as S;
        match self.kind {
            RiskKind::ScheduleSlip { .. } => S::Gantt,
            RiskKind::StockOut { .. } => S::Warehouse,
            RiskKind::PendingMoney { .. } => S::Contracts,
            RiskKind::QualityDrop { .. } => S::Quality,
            RiskKind::SafetyOpen { .. } => S::Safety,
        }
    }
}

/// TZ XVII.10: keyin nima bo'lishi mumkin.
///
/// Prognoz **bugungi sur'atga** tayanadi va uni ochiq aytadi: bu bashorat
/// emas, hozirgi holat davom etsa nima bo'lishining hisobi. Sur'at
/// o'zgarsa natija ham o'zgaradi.
pub fn risk_forecast(
    delay_days: i64,
    plan: &[PurchasePlanLine],
    materials: &[Material],
    changes: &[ContractChange],
    quality: &QualityWeek,
    safety_open: usize,
    today: NaiveDate,
) -> Vec<RiskLine> {
    let mut out = Vec::new();

    if delay_days > 0 {
        out.push(RiskLine {
            kind: RiskKind::ScheduleSlip { days: delay_days },
            when: None,
            // Har kun kechikish uch ball: 20 kun — 60, ya'ni yuqori daraja.
            weight: (delay_days as f64 * 3.0).clamp(0.0, 100.0),
        });
    }

    // Materiallar: sotib olish kerak bo'lganlari, eng yaqin muddatlisi
    // oldinda. Muddat ko'rsatilmagan qator prognozga tushmaydi — «qachon»
    // degan savolga javob bo'lmasa, bu prognoz emas.
    for line in plan.iter().filter(|l| l.to_buy > 0.0) {
        let Some(need_by) = line.need_by else {
            continue;
        };
        let days = (need_by - today).num_days().max(0);
        out.push(RiskLine {
            kind: RiskKind::StockOut {
                material: materials
                    .iter()
                    .find(|m| m.id == line.material_id)
                    .map(|m| m.name.clone())
                    .unwrap_or_default(),
                days,
            },
            when: Some(need_by),
            // Yaqinroq bo'lsa og'irroq: 0 kun — 100, 30 kun — 0.
            weight: ((30 - days.min(30)) as f64 * 100.0 / 30.0).clamp(0.0, 100.0),
        });
    }

    let pending: f64 = changes
        .iter()
        .filter(|c| matches!(c.status, ChangeStatus::Draft | ChangeStatus::Sent))
        .map(|c| c.amount)
        .sum();
    if pending > 0.0 {
        out.push(RiskLine {
            kind: RiskKind::PendingMoney { amount: pending },
            when: None,
            weight: 50.0,
        });
    }

    if quality.defects_open > 0 {
        out.push(RiskLine {
            kind: RiskKind::QualityDrop {
                open: quality.defects_open,
                overdue: quality.overdue,
            },
            when: None,
            // Muddati o'tgan nuqson og'irroq: har biri 20 ball.
            weight: (quality.overdue as f64 * 20.0 + quality.defects_open as f64 * 5.0)
                .clamp(0.0, 100.0),
        });
    }

    if safety_open > 0 {
        out.push(RiskLine {
            kind: RiskKind::SafetyOpen { count: safety_open },
            when: None,
            weight: (safety_open as f64 * 25.0).clamp(0.0, 100.0),
        });
    }

    // Og'irlari oldinda.
    out.sort_by(|a, b| b.weight.total_cmp(&a.weight));
    out
}

// ================= X.23, 25-26, 47, XI.47. Ta'minot zanjiri =================

/// Ta'minot zanjirining bitta qatori (TZ X.47, XI.47).
///
/// Zanjir: **ariza → taklif → xarid → yetkazish → kirish nazorati →
/// ombor → ish → to'lov.** Har bosqich alohida modulda yozilgan; bu
/// yerda ular bir qatorda turadi, shuning uchun uzilish darhol ko'rinadi.
#[derive(Debug, Clone)]
pub struct SupplyChain {
    pub request_id: Option<i64>,
    pub material_id: Option<i64>,
    pub title: String,
    pub unit: String,
    /// Arizada so'ralgan miqdor.
    pub requested: f64,
    /// Olingan tijorat takliflari soni.
    pub quotes: usize,
    /// Buyurtma qilingan miqdor.
    pub ordered: f64,
    /// Yetkazilgan deb belgilangan miqdor.
    pub delivered: f64,
    /// Kirish nazoratidan o'tgan partiyalar soni.
    pub checks: usize,
    /// Omborga kirim qilingan miqdor.
    pub stocked: f64,
    /// Ishga berilgan miqdor.
    pub issued: f64,
    /// Xarid summasi va to'langan qismi.
    pub amount: f64,
    pub paid: f64,
}

/// Zanjirdagi uzilish (TZ X.47).
#[derive(Debug, Clone, PartialEq)]
pub enum ChainGap {
    /// Buyurtma arizadagidan ko'p.
    OrderOverRequest { over: f64 },
    /// Yetkazilgan miqdor buyurtmadan ko'p.
    DeliveryOverOrder { over: f64 },
    /// Yetkazilgan, lekin omborga kirim qilinmagan.
    NotStocked { qty: f64 },
    /// Omborda yo'q material ishga berilgan.
    IssuedOverStock { over: f64 },
    /// Yetkazilgan, lekin kirish nazoratidan o'tmagan.
    NoInputCheck,
    /// Taklif olinmagan: taqqoslashsiz xarid.
    NoQuotes,
    /// To'lov summadan oshgan.
    Overpaid { over: f64 },
    /// To'lov muddati o'tgan.
    PaymentOverdue { unpaid: f64 },
}

impl ChainGap {
    /// Hisobni buzadigan uzilish.
    pub fn severe(&self) -> bool {
        matches!(
            self,
            ChainGap::DeliveryOverOrder { .. }
                | ChainGap::IssuedOverStock { .. }
                | ChainGap::Overpaid { .. }
                | ChainGap::PaymentOverdue { .. }
        )
    }
}

impl SupplyChain {
    /// Zanjirdagi barcha uzilishlar.
    ///
    /// Har bosqichda qiymat kamaymasligi kerak: keyingi bosqich
    /// oldingisidan katta bo'lsa, biror yozuv hujjatsiz o'tgan.
    pub fn gaps(&self, today: NaiveDate, pay_due: Option<NaiveDate>) -> Vec<ChainGap> {
        let mut out = Vec::new();
        let eps = 0.0001;

        if self.requested > 0.0 && self.ordered > self.requested + eps {
            out.push(ChainGap::OrderOverRequest {
                over: self.ordered - self.requested,
            });
        }
        if self.ordered > 0.0 && self.delivered > self.ordered + eps {
            out.push(ChainGap::DeliveryOverOrder {
                over: self.delivered - self.ordered,
            });
        }
        if self.delivered > eps && self.stocked + eps < self.delivered {
            out.push(ChainGap::NotStocked {
                qty: self.delivered - self.stocked,
            });
        }
        if self.issued > self.stocked + eps {
            out.push(ChainGap::IssuedOverStock {
                over: self.issued - self.stocked,
            });
        }
        if self.delivered > eps && self.checks == 0 {
            out.push(ChainGap::NoInputCheck);
        }
        if self.ordered > eps && self.quotes == 0 {
            out.push(ChainGap::NoQuotes);
        }
        if self.paid > self.amount + 0.01 {
            out.push(ChainGap::Overpaid {
                over: self.paid - self.amount,
            });
        }
        let unpaid = (self.amount - self.paid).max(0.0);
        if unpaid > 0.01 && pay_due.is_some_and(|d| d < today) {
            out.push(ChainGap::PaymentOverdue { unpaid });
        }

        out.sort_by_key(|g| !g.severe());
        out
    }
}

/// Zanjir yakuni.
#[derive(Debug, Clone, Default)]
pub struct ChainSummary {
    pub lines: usize,
    pub with_gaps: usize,
    pub amount: f64,
    pub paid: f64,
    /// Muddati o'tgan to'lov summasi.
    pub overdue_pay: f64,
}

/// TZ X.47, XI.47: ta'minot zanjirini xaridlar bo'yicha yig'adi.
///
/// Guruhlash **xarid bo'yicha**: zanjirning markazida xarid turadi,
/// chunki undan oldin ariza va taklif, keyin yetkazish, ombor va to'lov
/// bo'ladi. Arizasiz xarid ham qatorga tushadi — u ham zanjirning bir
/// holati va ko'rinib turishi kerak.
#[allow(clippy::too_many_arguments)]
pub fn supply_chain(
    purchases: &[Purchase],
    requests: &[Request],
    quotes: &[Quote],
    moves: &[StockMove],
    quality: &[QualityCheck],
    materials: &[Material],
    today: NaiveDate,
) -> (Vec<(SupplyChain, Vec<ChainGap>)>, ChainSummary) {
    let key = |s: &str| s.trim().to_lowercase();
    let mut out = Vec::new();
    let mut sum = ChainSummary::default();

    for p in purchases
        .iter()
        .filter(|p| p.status != PurchaseStatus::Draft)
    {
        let request = p
            .request_id
            .and_then(|id| requests.iter().find(|r| r.id == id));
        let material_id = p
            .material_id
            .or_else(|| request.and_then(|r| r.material_id));

        // Ombor harakatlari: hujjat raqami xarid raqamini o'z ichiga olsa
        // yoki material bo'yicha bog'lansa.
        let linked = |m: &StockMove| -> bool {
            (!p.number.trim().is_empty() && m.document.contains(p.number.trim()))
                || material_id.is_some_and(|id| m.material_id == id)
        };
        let stocked: f64 = moves
            .iter()
            .filter(|m| matches!(m.kind, MoveKind::In) && linked(m))
            .map(|m| m.qty)
            .sum();
        let issued: f64 = moves
            .iter()
            .filter(|m| matches!(m.kind, MoveKind::Out) && linked(m))
            .map(|m| m.qty)
            .sum();

        let line = SupplyChain {
            request_id: p.request_id,
            material_id,
            title: p.title.clone(),
            unit: p.unit.clone(),
            requested: request.map(|r| r.qty).unwrap_or(0.0),
            quotes: quotes
                .iter()
                .filter(|q| {
                    key(&q.title) == key(&p.title)
                        || (q.request_id.is_some() && q.request_id == p.request_id)
                })
                .count(),
            ordered: p.qty,
            delivered: p.delivered_qty,
            checks: quality
                .iter()
                .filter(|q| {
                    q.kind == QualityKind::Input
                        && (q.material_id == material_id && material_id.is_some()
                            || key(&q.subject).contains(&key(&p.title)))
                })
                .count(),
            stocked,
            issued,
            amount: p.amount(),
            paid: p.paid,
        };

        let gaps = line.gaps(today, p.pay_due);
        sum.amount += line.amount;
        sum.paid += line.paid;
        if p.payment_overdue(today) {
            sum.overdue_pay += p.unpaid();
        }
        if !gaps.is_empty() {
            sum.with_gaps += 1;
        }
        out.push((line, gaps));
    }

    sum.lines = out.len();
    let _ = materials;
    // Uzilishi ko'p qatorlar oldinda.
    out.sort_by_key(|(_, g)| std::cmp::Reverse(g.len()));
    (out, sum)
}

// ================= XVI.8, 21, 26, 42, 48. Texnika zanjiri va prognoz =================

/// Bitta texnika bo'yicha to'liq manzara (TZ XVI.21, 42, 48).
///
/// Zanjir: **ariza → biriktirish → smena → yoqilg'i → ta'mir → tannarx.**
/// Har bosqich alohida yozuvda; bu yerda ular bir qatorda turadi.
#[derive(Debug, Clone)]
pub struct MachineChain {
    pub machine_id: i64,
    pub name: String,
    pub rented: bool,
    /// Shu texnikaga berilgan arizalar soni.
    pub requests: usize,
    /// Ishlangan soat va bo'sh turgan kunlar.
    pub hours: f64,
    pub idle_days: i64,
    /// Yoqilg'i: yozilgan va normativ bo'yicha kutilgan.
    pub fuel_used: f64,
    pub fuel_norm: f64,
    /// Ta'mirlar soni va xarajati.
    pub repairs: usize,
    pub repair_cost: f64,
    /// Ishlagan soatning qiymati.
    pub work_cost: f64,
}

impl MachineChain {
    /// Yoqilg'i normadan chetlanishi, foizda. Norma yo'q bo'lsa `None`.
    pub fn fuel_deviation(&self) -> Option<f64> {
        (self.fuel_norm > 0.0).then(|| (self.fuel_used - self.fuel_norm) * 100.0 / self.fuel_norm)
    }

    /// Bir motosoatning to'liq qiymati: ish va ta'mir birga.
    pub fn cost_per_hour(&self) -> Option<f64> {
        (self.hours > 0.0).then(|| (self.work_cost + self.repair_cost) / self.hours)
    }

    /// Samaradorlik balli, 0-100 (TZ XVI.42).
    ///
    /// Uch qism teng vaznda: foydalanish (bo'sh turmaganmi), ishonchlilik
    /// (ta'mirga tushmaganmi) va yoqilg'i intizomi. Narx **ballga
    /// kirmaydi**: qimmat, lekin doim ishlaydigan texnikani yomon deb
    /// ko'rsatib qo'ymaslik uchun — narx alohida ustunda turadi.
    pub fn score(&self, period_days: i64) -> f64 {
        let days = period_days.max(1) as f64;
        let usage = (self.hours / (days * SHIFT_HOURS) * 100.0).clamp(0.0, 100.0);
        // Har ta'mir 20 ball tushiradi.
        let reliability = (100.0 - self.repairs as f64 * 20.0).clamp(0.0, 100.0);
        let fuel = match self.fuel_deviation() {
            // Norma yo'q — bu qism baholanmaydi va to'liq ball beriladi:
            // ma'lumot yo'qligi uchun jazolash noto'g'ri bo'lardi.
            None => 100.0,
            Some(d) => (100.0 - d.abs()).clamp(0.0, 100.0),
        };
        (usage + reliability + fuel) / 3.0
    }
}

/// Ta'mir prognozi (TZ XVI.26).
#[derive(Debug, Clone)]
pub struct RepairForecast {
    pub machine_id: i64,
    pub name: String,
    /// Ta'mirlar orasidagi o'rtacha motosoat (MTBF).
    pub mtbf_hours: f64,
    /// Oxirgi ta'mirdan keyin ishlangan soat.
    pub since_last: f64,
    /// Keyingi ta'mirgacha qolgan soat. Manfiy — muddat o'tgan.
    pub hours_left: f64,
    /// Kunlik o'rtacha soat bo'yicha taxminiy sana.
    pub when: Option<NaiveDate>,
    /// Prognoz nechta ta'mirga tayangan. Ikkitadan kam bo'lsa prognoz
    /// berilmaydi — bitta hodisadan qonuniyat chiqmaydi.
    pub based_on: usize,
}

impl RepairForecast {
    /// Ta'mir yaqinlashdimi.
    pub fn soon(&self) -> bool {
        self.mtbf_hours > 0.0 && self.hours_left <= self.mtbf_hours * 0.2
    }
}

/// TZ XVI.26: keyingi ta'mir qachon kutilishini baholaydi.
///
/// Hisob **tarixga** tayanadi: ta'mirlar orasidagi o'rtacha motosoat
/// (MTBF) topiladi va oxirgi ta'mirdan keyin ishlangan soat undan
/// ayriladi. Ikkitadan kam ta'miri bor texnikaga prognoz berilmaydi —
/// bitta hodisadan qonuniyat chiqmaydi.
pub fn repair_forecast(
    machines: &[Machine],
    logs: &[MachineLog],
    repairs: &[MachineRepair],
    today: NaiveDate,
    period_days: i64,
) -> Vec<RepairForecast> {
    let mut out = Vec::new();
    for m in machines {
        let mut mine: Vec<&MachineRepair> = repairs
            .iter()
            .filter(|r| r.machine_id == m.id && r.kind != RepairKind::Planned)
            .collect();
        if mine.len() < 2 {
            continue;
        }
        mine.sort_by_key(|r| r.started);

        // Ta'mirlar orasidagi motosoat: har oraliqda ishlangan soat.
        let mut gaps: Vec<f64> = Vec::new();
        for pair in mine.windows(2) {
            let hours: f64 = logs
                .iter()
                .filter(|l| {
                    l.machine_id == m.id && l.date > pair[0].started && l.date <= pair[1].started
                })
                .map(|l| l.hours)
                .sum();
            if hours > 0.0 {
                gaps.push(hours);
            }
        }
        if gaps.is_empty() {
            continue;
        }
        let mtbf = gaps.iter().sum::<f64>() / gaps.len() as f64;

        let last = mine.last().unwrap().started;
        let since: f64 = logs
            .iter()
            .filter(|l| l.machine_id == m.id && l.date > last)
            .map(|l| l.hours)
            .sum();

        // Kunlik sur'at: oxirgi davr bo'yicha.
        let from = today - chrono::Duration::days(period_days.max(1));
        let recent: f64 = logs
            .iter()
            .filter(|l| l.machine_id == m.id && l.date > from && l.date <= today)
            .map(|l| l.hours)
            .sum();
        let per_day = recent / period_days.max(1) as f64;
        let left = mtbf - since;

        out.push(RepairForecast {
            machine_id: m.id,
            name: m.name.clone(),
            mtbf_hours: mtbf,
            since_last: since,
            hours_left: left,
            when: (per_day > 0.0)
                .then(|| today + chrono::Duration::days((left.max(0.0) / per_day).round() as i64)),
            based_on: mine.len(),
        });
    }
    // Eng yaqin ta'mir oldinda.
    out.sort_by(|a, b| a.hours_left.total_cmp(&b.hours_left));
    out
}

/// TZ XVI.21, 42, 48: texnika bo'yicha to'liq zanjirni yig'adi.
pub fn machine_chain(
    machines: &[Machine],
    logs: &[MachineLog],
    repairs: &[MachineRepair],
    requests: &[Request],
    today: NaiveDate,
    period_days: i64,
) -> Vec<MachineChain> {
    let from = today - chrono::Duration::days(period_days.max(1));
    let key = |s: &str| s.trim().to_lowercase();

    let mut out = Vec::new();
    for m in machines {
        let mine: Vec<&MachineLog> = logs
            .iter()
            .filter(|l| l.machine_id == m.id && l.date > from && l.date <= today)
            .collect();
        let hours: f64 = mine.iter().map(|l| l.hours).sum();
        let worked_days = {
            let mut d: Vec<NaiveDate> = mine
                .iter()
                .filter(|l| l.hours > 0.0)
                .map(|l| l.date)
                .collect();
            d.sort_unstable();
            d.dedup();
            d.len() as i64
        };
        let period_repairs: Vec<&MachineRepair> = repairs
            .iter()
            .filter(|r| r.machine_id == m.id && r.started > from && r.started <= today)
            .collect();

        out.push(MachineChain {
            machine_id: m.id,
            name: m.name.clone(),
            rented: m.rented,
            // Texnikaga ariza: nomi arizada uchraydigan yozuvlar (TZ XVI.8).
            requests: requests
                .iter()
                .filter(|r| {
                    matches!(r.kind, RequestKind::Machine | RequestKind::Transport)
                        && (key(&r.title).contains(&key(&m.name))
                            || key(&r.note).contains(&key(&m.name)))
                })
                .count(),
            hours,
            idle_days: period_days.max(1) - worked_days,
            fuel_used: mine.iter().map(|l| l.fuel).sum(),
            fuel_norm: m.fuel_norm * hours,
            repairs: period_repairs.len(),
            repair_cost: period_repairs.iter().map(|r| r.cost).sum(),
            work_cost: hours * m.hour_rate,
        });
    }
    // Eng past ball oldinda.
    out.sort_by(|a, b| a.score(period_days).total_cmp(&b.score(period_days)));
    out
}

// ================= XIV.15, 19, 26-27. Bo'limlar, ustuvorlik, ketma-ketlik =================

/// Bo'lim bo'yicha sifat yakuni (TZ XIV.15).
#[derive(Debug, Clone)]
pub struct SectionQuality {
    pub section: Section,
    pub checks: usize,
    pub failed: usize,
    pub defects_open: usize,
    pub overdue: usize,
    /// Sifat balli, 0-100 — umumiy ball bilan bir xil qoidada.
    pub score: f64,
}

/// TZ XIV.15: bo'limlar kesimida sifat.
///
/// Ball [`quality_score`] bilan **bir xil qoidada** hisoblanadi, faqat
/// tanlangan bo'lim yozuvlari bo'yicha — shuning uchun bo'lim balli va
/// umumiy ball bir mantiqdan chiqadi.
pub fn section_quality(
    quality: &[QualityCheck],
    tasks: &[Task],
    today: NaiveDate,
) -> Vec<SectionQuality> {
    let section_of = |q: &QualityCheck| -> Option<Section> {
        q.task_id
            .and_then(|id| tasks.iter().find(|t| t.id == id))
            .map(|t| t.section)
    };

    let mut out = Vec::new();
    for section in Section::ALL {
        if section == Section::None {
            continue;
        }
        let mine: Vec<QualityCheck> = quality
            .iter()
            .filter(|q| section_of(q) == Some(section))
            .cloned()
            .collect();
        if mine.is_empty() {
            continue;
        }
        let has_defect = |q: &QualityCheck| !q.defect.trim().is_empty();
        out.push(SectionQuality {
            section,
            checks: mine.len(),
            failed: mine
                .iter()
                .filter(|q| q.result == QualityResult::Fail)
                .count(),
            defects_open: mine
                .iter()
                .filter(|q| has_defect(q) && q.fixed_at.is_none())
                .count(),
            overdue: mine
                .iter()
                .filter(|q| {
                    has_defect(q) && q.fixed_at.is_none() && q.deadline.is_some_and(|d| d < today)
                })
                .count(),
            score: quality_score(&mine, today).score,
        });
    }
    // Eng past ball oldinda.
    out.sort_by(|a, b| a.score.total_cmp(&b.score));
    out
}

/// Nuqsonning ustuvorligi (TZ XIV.19).
#[derive(Debug, Clone)]
pub struct DefectPriority {
    pub check_id: i64,
    pub defect: String,
    /// Ustuvorlik balli: qanchalik katta bo'lsa, shunchalik oldin.
    pub weight: f64,
    /// Ball qaysi sabablardan yig'ilgani — i18n kalitlari.
    pub reasons: Vec<&'static str>,
}

impl DefectPriority {
    /// Birinchi navbatda hal qilinadigan nuqson.
    pub fn urgent(&self) -> bool {
        self.weight >= 60.0
    }
}

/// TZ XIV.19: nuqsonlarni ustuvorlik bo'yicha tartiblaydi.
///
/// Ball to'rt sababdan yig'iladi va **har sabab ko'rsatiladi**: nima
/// uchun aynan shu nuqson birinchi ekani ko'rinib turishi kerak, aks
/// holda tartib ishonchsiz bo'ladi.
pub fn defect_priority(
    quality: &[QualityCheck],
    tasks: &[Task],
    today: NaiveDate,
) -> Vec<DefectPriority> {
    let mut out = Vec::new();
    for q in quality
        .iter()
        .filter(|q| !q.defect.trim().is_empty() && q.fixed_at.is_none())
    {
        let mut weight = 20.0_f64;
        let mut reasons = vec!["dp_open"];

        // Muddati o'tgan — eng og'ir sabab.
        if q.deadline.is_some_and(|d| d < today) {
            weight += 40.0;
            reasons.push("dp_overdue");
        }
        // Salbiy natija.
        if q.result == QualityResult::Fail {
            weight += 20.0;
            reasons.push("dp_failed");
        }
        // Yopilishga yaqin ish: nuqson bosqich ostida ko'milib qoladi.
        if let Some(task) = q.task_id.and_then(|id| tasks.iter().find(|t| t.id == id)) {
            if task.progress >= CLOSING_PROGRESS {
                weight += 20.0;
                reasons.push("dp_closing");
            }
            // Konstruksiya va tarmoq: keyin ochib bo'lmaydi.
            if matches!(
                task.section,
                Section::Kj | Section::Km | Section::Vk | Section::Ov | Section::Eom | Section::Pb
            ) {
                weight += 10.0;
                reasons.push("dp_hidden");
            }
        }

        out.push(DefectPriority {
            check_id: q.id,
            defect: q.defect.clone(),
            weight: weight.min(100.0),
            reasons,
        });
    }
    out.sort_by(|a, b| b.weight.total_cmp(&a.weight));
    out
}

/// Texnologik ketma-ketlik buzilishi (TZ XIV.27, VII.28).
#[derive(Debug, Clone, PartialEq)]
pub struct SequenceBreak {
    /// Boshlangan ish.
    pub task_id: i64,
    /// Tugallanmagan oldingi ish.
    pub pred_id: i64,
    /// Oldingi ishning bajarilishi.
    pub pred_progress: f64,
    /// Oldingi ishda sifat tekshiruvi o'tkazilganmi.
    pub pred_checked: bool,
}

/// TZ XIV.27, VII.28: ish oldingisi tugamasdan boshlanganmi.
///
/// Bu **taqiq emas**: qurilishda ishlar qisman ustma-ust ketadi. Lekin
/// oldingi ish sifat tekshiruvidan o'tmagan bo'lsa, keyingisi uni
/// ko'mib yuboradi — shuning uchun tekshiruv holati alohida ustunda.
pub fn sequence_breaks(
    tasks: &[Task],
    links: &[crate::model::Link],
    quality: &[QualityCheck],
) -> Vec<SequenceBreak> {
    let mut out = Vec::new();
    for l in links {
        let (Some(pred), Some(succ)) = (
            tasks.iter().find(|t| t.id == l.pred),
            tasks.iter().find(|t| t.id == l.succ),
        ) else {
            continue;
        };
        let succ_started = succ.progress > 0.0 || succ.fact_start.is_some();
        let pred_done = pred.progress >= 99.999 || pred.fact_end.is_some();
        if !succ_started || pred_done {
            continue;
        }
        out.push(SequenceBreak {
            task_id: succ.id,
            pred_id: pred.id,
            pred_progress: pred.progress,
            pred_checked: quality.iter().any(|q| q.task_id == Some(pred.id)),
        });
    }
    // Tekshirilmaganlari oldinda: xavf shu yerda.
    out.sort_by_key(|b| (b.pred_checked, b.task_id));
    out
}
