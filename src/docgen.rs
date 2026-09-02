//! Hujjat generatsiyasi va Excel eksporti (TZ IV.5–7, XI.25, umumiy talab).
//!
//! Bu modul bazadagi ma'lumotdan rasmiy shakllarni yig'adi: **KS-2** (bajarilgan
//! ishlarni qabul qilish dalolatnomasi), **KS-3** (bajarilgan ish qiymati
//! haqida ma'lumotnoma), **M-29** (material sarfi hisoboti) va **yashirin
//! ishlar dalolatnomasi**. Har bir hujjat `.xlsx` bo'lib chiqadi — bu shakllar
//! amalda Excel da yuritiladi va shu ko'rinishda topshiriladi.
//!
//! Ikkita qat'iy qoida:
//!
//! 1. **Son o'ylab topilmaydi.** Hujjatga faqat bazada bor qiymat tushadi:
//!    hajm ijro foizidan, narx smetadan, material sarfi ombordan olinadi.
//!    Ma'lumot yetishmasa katak bo'sh qoladi — nol yozilmaydi.
//! 2. **Imzo joyi bo'sh qoladi.** Dastur hech kimni imzolamaydi; hujjatda
//!    faqat kim imzolashi kerakligi yoziladi.

use crate::checks::{ConsumptionLine, CostSummary};
use crate::domain::{ExecDoc, Material, MaterialNorm, MoveKind, StockMove};
use crate::i18n::t;
use crate::model::{Party, PartyRole, Project, Task};
use chrono::NaiveDate;
use rust_xlsxwriter::{Format, FormatAlign, FormatBorder, Workbook, Worksheet, XlsxError};
use std::path::Path;

/// Hujjatga kerak bo'ladigan umumiy ma'lumot.
pub struct DocInput<'a> {
    pub project: &'a Project,
    pub parties: &'a [Party],
    pub tasks: &'a [Task],
    pub today: NaiveDate,
    /// Hisobot davri.
    pub from: NaiveDate,
    pub to: NaiveDate,
}

impl DocInput<'_> {
    /// Tomon nomi rol bo'yicha. Topilmasa bo'sh satr — hujjatda joy qoladi.
    fn party(&self, role: PartyRole) -> &str {
        self.parties
            .iter()
            .find(|p| p.role == role)
            .map(|p| p.name.as_str())
            .unwrap_or("")
    }

    fn person(&self, role: PartyRole) -> &str {
        self.parties
            .iter()
            .find(|p| p.role == role)
            .map(|p| p.person.as_str())
            .unwrap_or("")
    }
}

/// Hujjat uslublari — barcha shakllarda bir xil ko'rinish.
struct Styles {
    title: Format,
    head: Format,
    label: Format,
    cell: Format,
    cell_num: Format,
    cell_money: Format,
    total: Format,
    total_money: Format,
    note: Format,
}

impl Styles {
    fn new() -> Self {
        let border = |f: Format| f.set_border(FormatBorder::Thin);
        Self {
            title: Format::new()
                .set_bold()
                .set_font_size(13.0)
                .set_align(FormatAlign::Center),
            head: border(Format::new())
                .set_bold()
                .set_align(FormatAlign::Center)
                .set_align(FormatAlign::VerticalCenter)
                .set_text_wrap()
                .set_background_color(0xEFEFEF),
            label: Format::new().set_bold(),
            cell: border(Format::new()).set_text_wrap(),
            cell_num: border(Format::new())
                .set_align(FormatAlign::Right)
                .set_num_format("#,##0.###"),
            cell_money: border(Format::new())
                .set_align(FormatAlign::Right)
                .set_num_format("#,##0.00"),
            total: border(Format::new()).set_bold(),
            total_money: border(Format::new())
                .set_bold()
                .set_align(FormatAlign::Right)
                .set_num_format("#,##0.00"),
            note: Format::new().set_font_size(9.0).set_italic(),
        }
    }
}

/// Sarlavha bloki: hujjat nomi, obyekt, tomonlar va davr.
fn header(
    sh: &mut Worksheet,
    st: &Styles,
    inp: &DocInput,
    title: &str,
    number: &str,
    width: u16,
) -> Result<u32, XlsxError> {
    sh.merge_range(0, 0, 0, width - 1, title, &st.title)?;
    let mut r = 2u32;
    let mut pair = |sh: &mut Worksheet, label: &str, value: &str| -> Result<(), XlsxError> {
        sh.write_string_with_format(r, 0, label, &st.label)?;
        sh.merge_range(r, 1, r, width - 1, value, &Format::new())?;
        r += 1;
        Ok(())
    };
    pair(sh, t("doc_number"), number)?;
    pair(sh, t("doc_date"), &inp.today.format("%d.%m.%Y").to_string())?;
    pair(sh, t("doc_object"), &inp.project.name)?;
    if !inp.project.address.is_empty() {
        pair(sh, t("doc_address"), &inp.project.address)?;
    }
    pair(sh, t("doc_client"), inp.party(PartyRole::Client))?;
    pair(sh, t("doc_contractor"), inp.party(PartyRole::Contractor))?;
    pair(
        sh,
        t("doc_period"),
        &format!(
            "{} — {}",
            inp.from.format("%d.%m.%Y"),
            inp.to.format("%d.%m.%Y")
        ),
    )?;
    Ok(r + 1)
}

/// Imzo bloki: kim imzolashi kerak. Imzoning o'zi qo'yilmaydi.
fn signatures(
    sh: &mut Worksheet,
    st: &Styles,
    inp: &DocInput,
    row: u32,
    roles: &[PartyRole],
) -> Result<(), XlsxError> {
    let mut r = row + 2;
    sh.write_string_with_format(r, 0, t("doc_signatures"), &st.label)?;
    r += 1;
    for role in roles {
        sh.write_string(r, 0, role.label())?;
        sh.write_string(r, 1, inp.party(*role))?;
        sh.write_string(r, 2, inp.person(*role))?;
        sh.write_string_with_format(r, 3, t("doc_sign_line"), &st.note)?;
        r += 1;
    }
    r += 1;
    sh.write_string_with_format(r, 0, t("doc_made_by"), &st.note)?;
    Ok(())
}

/// Ustun kengliklarini o'rnatadi.
fn widths(sh: &mut Worksheet, w: &[f64]) -> Result<(), XlsxError> {
    for (i, v) in w.iter().enumerate() {
        sh.set_column_width(i as u16, *v)?;
    }
    Ok(())
}

// ================================================================ KS-2

/// KS-2 uchun bitta qator: bajarilgan ish va uning qiymati.
#[derive(Debug, Clone)]
pub struct Ks2Line {
    pub pos: i64,
    pub code: String,
    pub name: String,
    pub unit: String,
    /// Shartnoma (smeta) bo'yicha hajm.
    pub plan_qty: f64,
    /// Davr ichida bajarilgan hajm.
    pub done_qty: f64,
    pub price: f64,
    pub cost: f64,
}

/// TZ IV.5: KS-2 qatorlarini GPR ishlaridan yig'adi.
///
/// Hajm ijro foizidan olinadi: `hajm × bajarilish%`. Narx smetadagi shu nomli
/// pozitsiyadan izlanadi; topilmasa nol qoladi va hujjatda bo'sh ko'rinadi —
/// narxni o'ylab topmaymiz.
pub fn ks2_lines(
    tasks: &[Task],
    estimate: &[crate::domain::EstimateItem],
    from: NaiveDate,
    to: NaiveDate,
) -> Vec<Ks2Line> {
    let norm = |s: &str| {
        s.to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    tasks
        .iter()
        // Davr ichida ishlangan yoki davr ichida yopilgan ishlar.
        .filter(|t| t.progress > 0.0)
        .filter(|t| {
            let start = t.fact_start.unwrap_or(t.plan_start);
            let end = t
                .fact_end
                .unwrap_or(t.plan_start + chrono::Duration::days(t.duration.max(1) - 1));
            start <= to && end >= from
        })
        .enumerate()
        .map(|(i, task)| {
            let item = estimate.iter().find(|e| norm(&e.name) == norm(&task.name));
            let price = item.map_or(0.0, |e| e.price);
            let done = task.volume * task.progress / 100.0;
            Ks2Line {
                pos: i as i64 + 1,
                code: item.map(|e| e.code.clone()).unwrap_or_default(),
                name: format!("{} {}", task.wbs, task.name),
                unit: task.unit.clone(),
                plan_qty: task.volume,
                done_qty: done,
                price,
                cost: done * price,
            }
        })
        .collect()
}

/// KS-2 — bajarilgan ishlarni qabul qilish dalolatnomasi.
pub fn write_ks2(path: &Path, inp: &DocInput, lines: &[Ks2Line]) -> Result<f64, XlsxError> {
    let st = Styles::new();
    let mut wb = Workbook::new();
    let sh = wb.add_worksheet();
    sh.set_name(t("doc_ks2_short"))?;
    widths(sh, &[8.0, 14.0, 46.0, 10.0, 12.0, 12.0, 14.0, 16.0])?;

    let mut r = header(sh, &st, inp, t("doc_ks2"), &next_number(inp, "KS-2"), 8)?;

    for (i, h) in [
        t("doc_pos"),
        t("doc_code"),
        t("doc_work"),
        t("col_unit"),
        t("doc_plan_qty"),
        t("doc_done_qty"),
        t("col_price"),
        t("col_sum"),
    ]
    .iter()
    .enumerate()
    {
        sh.write_string_with_format(r, i as u16, *h, &st.head)?;
    }
    sh.set_row_height(r, 30.0)?;
    r += 1;

    let mut total = 0.0;
    for l in lines {
        sh.write_number_with_format(r, 0, l.pos as f64, &st.cell_num)?;
        sh.write_string_with_format(r, 1, &l.code, &st.cell)?;
        sh.write_string_with_format(r, 2, &l.name, &st.cell)?;
        sh.write_string_with_format(r, 3, &l.unit, &st.cell)?;
        sh.write_number_with_format(r, 4, l.plan_qty, &st.cell_num)?;
        sh.write_number_with_format(r, 5, l.done_qty, &st.cell_num)?;
        // Narx topilmagan bo'lsa katak bo'sh qoladi — nol yozmaymiz.
        if l.price > 0.0 {
            sh.write_number_with_format(r, 6, l.price, &st.cell_money)?;
            sh.write_number_with_format(r, 7, l.cost, &st.cell_money)?;
        } else {
            sh.write_string_with_format(r, 6, "", &st.cell)?;
            sh.write_string_with_format(r, 7, "", &st.cell)?;
        }
        total += l.cost;
        r += 1;
    }

    sh.merge_range(r, 0, r, 6, t("doc_total"), &st.total)?;
    sh.write_number_with_format(r, 7, total, &st.total_money)?;

    // Narxsiz qatorlar bo'lsa — buni yashirmaymiz.
    let missing = lines.iter().filter(|l| l.price <= 0.0).count();
    if missing > 0 {
        r += 2;
        sh.write_string_with_format(r, 0, format!("{} {missing}", t("doc_no_price")), &st.note)?;
    }

    signatures(
        sh,
        &st,
        inp,
        r,
        &[
            PartyRole::Contractor,
            PartyRole::TechSupervision,
            PartyRole::Client,
        ],
    )?;
    wb.save(path)?;
    Ok(total)
}

// ================================================================ KS-3

/// KS-3 — bajarilgan ish qiymati va xarajatlar haqida ma'lumotnoma.
///
/// Uch ustun: shartnoma boshidan, yil boshidan va shu davr uchun. Bu yerda
/// yil boshi va davr bir xil bo'lishi mumkin — davr foydalanuvchi tanlaydi.
pub fn write_ks3(
    path: &Path,
    inp: &DocInput,
    period_total: f64,
    since_start: f64,
    cost: &CostSummary,
) -> Result<(), XlsxError> {
    let st = Styles::new();
    let mut wb = Workbook::new();
    let sh = wb.add_worksheet();
    sh.set_name(t("doc_ks3_short"))?;
    widths(sh, &[8.0, 52.0, 18.0, 18.0, 18.0])?;

    let mut r = header(sh, &st, inp, t("doc_ks3"), &next_number(inp, "KS-3"), 5)?;

    for (i, h) in [
        t("doc_pos"),
        t("doc_kind"),
        t("doc_since_contract"),
        t("doc_since_year"),
        t("doc_period_col"),
    ]
    .iter()
    .enumerate()
    {
        sh.write_string_with_format(r, i as u16, *h, &st.head)?;
    }
    sh.set_row_height(r, 30.0)?;
    r += 1;

    // Shartnoma summasi — pasportdan, bajarilgani — KS-2 dan.
    let rows: [(&str, f64, f64, f64); 3] = [
        (
            t("doc_row_contract"),
            inp.project.contract_sum,
            inp.project.contract_sum,
            0.0,
        ),
        (t("doc_row_done"), since_start, since_start, period_total),
        (
            t("doc_row_paid"),
            inp.project.paid_total,
            inp.project.paid_total,
            0.0,
        ),
    ];
    for (i, (name, a, b, c)) in rows.iter().enumerate() {
        sh.write_number_with_format(r, 0, i as f64 + 1.0, &st.cell_num)?;
        sh.write_string_with_format(r, 1, *name, &st.cell)?;
        sh.write_number_with_format(r, 2, *a, &st.cell_money)?;
        sh.write_number_with_format(r, 3, *b, &st.cell_money)?;
        sh.write_number_with_format(r, 4, *c, &st.cell_money)?;
        r += 1;
    }

    // To'lanmagan qoldiq — bajarilgan minus to'langan.
    let unpaid = (since_start - inp.project.paid_total).max(0.0);
    sh.merge_range(r, 0, r, 3, t("doc_row_unpaid"), &st.total)?;
    sh.write_number_with_format(r, 4, unpaid, &st.total_money)?;
    r += 2;

    // Smeta bilan solishtirish — hujjat o'zi tekshirib turadi.
    sh.write_string_with_format(r, 0, t("doc_estimate_check"), &st.label)?;
    r += 1;
    sh.write_string(r, 0, t("cl_estimate"))?;
    sh.write_number_with_format(r, 2, cost.total, &st.cell_money)?;
    r += 1;
    if inp.project.contract_sum > 0.0 && cost.total > inp.project.contract_sum {
        sh.write_string_with_format(r, 0, t("doc_over_contract"), &st.note)?;
        r += 1;
    }

    signatures(sh, &st, inp, r, &[PartyRole::Contractor, PartyRole::Client])?;
    wb.save(path)?;
    Ok(())
}

// ================================================================ M-29

/// M-29 — material sarfi hisoboti: normativ va haqiqiy sarf.
pub fn write_m29(
    path: &Path,
    inp: &DocInput,
    lines: &[ConsumptionLine],
    materials: &[Material],
    norms: &[MaterialNorm],
) -> Result<(), XlsxError> {
    let st = Styles::new();
    let mut wb = Workbook::new();
    let sh = wb.add_worksheet();
    sh.set_name(t("doc_m29_short"))?;
    widths(sh, &[8.0, 34.0, 30.0, 10.0, 12.0, 12.0, 12.0, 12.0, 14.0])?;

    let mut r = header(sh, &st, inp, t("doc_m29"), &next_number(inp, "M-29"), 9)?;

    for (i, h) in [
        t("doc_pos"),
        t("col_task"),
        t("col_material"),
        t("col_unit"),
        t("col_per_unit"),
        t("col_done_volume"),
        t("col_norm"),
        t("col_fact"),
        t("col_diff"),
    ]
    .iter()
    .enumerate()
    {
        sh.write_string_with_format(r, i as u16, *h, &st.head)?;
    }
    sh.set_row_height(r, 30.0)?;
    r += 1;

    for (i, l) in lines.iter().enumerate() {
        let task = inp
            .tasks
            .iter()
            .find(|t| t.id == l.task_id)
            .map(|t| format!("{} {}", t.wbs, t.name))
            .unwrap_or_default();
        let mat = materials.iter().find(|m| m.id == l.material_id);
        let per_unit = norms
            .iter()
            .find(|n| n.task_id == l.task_id && n.material_id == l.material_id)
            .map_or(0.0, |n| n.per_unit);

        sh.write_number_with_format(r, 0, i as f64 + 1.0, &st.cell_num)?;
        sh.write_string_with_format(r, 1, &task, &st.cell)?;
        sh.write_string_with_format(
            r,
            2,
            mat.map(|m| format!("{} {}", m.code, m.name))
                .unwrap_or_default(),
            &st.cell,
        )?;
        sh.write_string_with_format(r, 3, mat.map_or("", |m| m.unit.as_str()), &st.cell)?;
        sh.write_number_with_format(r, 4, per_unit, &st.cell_num)?;
        sh.write_number_with_format(r, 5, l.done_volume, &st.cell_num)?;
        sh.write_number_with_format(r, 6, l.norm, &st.cell_num)?;
        sh.write_number_with_format(r, 7, l.fact, &st.cell_num)?;
        sh.write_number_with_format(r, 8, l.diff, &st.cell_num)?;
        r += 1;
    }

    let over: f64 = lines.iter().filter(|l| l.over).map(|l| l.over_cost).sum();
    if over > 0.0 {
        r += 1;
        sh.write_string_with_format(r, 0, t("kpi_overuse_cost"), &st.total)?;
        sh.write_number_with_format(r, 8, over, &st.total_money)?;
    }

    signatures(
        sh,
        &st,
        inp,
        r,
        &[PartyRole::Contractor, PartyRole::TechSupervision],
    )?;
    wb.save(path)?;
    Ok(())
}

// ================================================== Yashirin ishlar dalolatnomasi

/// AOSR — yashirin ishlarni ko'zdan kechirish dalolatnomasi (TZ IV.5).
///
/// Ishga bog'langan ijro hujjatidan tuziladi: qaysi ish, qaysi materiallar
/// ishlatilgan, qaysi normativga muvofiq.
pub fn write_aosr(
    path: &Path,
    inp: &DocInput,
    doc: &ExecDoc,
    moves: &[StockMove],
    materials: &[Material],
) -> Result<(), XlsxError> {
    let st = Styles::new();
    let mut wb = Workbook::new();
    let sh = wb.add_worksheet();
    sh.set_name(t("doc_aosr_short"))?;
    widths(sh, &[8.0, 40.0, 12.0, 14.0, 22.0, 20.0])?;

    let mut r = header(sh, &st, inp, t("doc_aosr"), &doc.number, 6)?;

    let task = doc
        .task_id
        .and_then(|id| inp.tasks.iter().find(|t| t.id == id));
    sh.write_string_with_format(r, 0, t("doc_work"), &st.label)?;
    sh.merge_range(
        r,
        1,
        r,
        5,
        &task
            .map(|t| format!("{} {}", t.wbs, t.name))
            .unwrap_or_else(|| doc.name.clone()),
        &Format::new(),
    )?;
    r += 2;

    // Ishda ishlatilgan materiallar — ombordagi chiqimdan.
    sh.write_string_with_format(r, 0, t("doc_materials_used"), &st.label)?;
    r += 1;
    for (i, h) in [
        t("doc_pos"),
        t("col_material"),
        t("col_unit"),
        t("col_qty"),
        t("col_cert"),
        t("col_document"),
    ]
    .iter()
    .enumerate()
    {
        sh.write_string_with_format(r, i as u16, *h, &st.head)?;
    }
    r += 1;

    let used: Vec<&StockMove> = moves
        .iter()
        .filter(|m| doc.task_id.is_some() && m.task_id == doc.task_id)
        .filter(|m| m.kind == MoveKind::Out)
        .collect();
    if used.is_empty() {
        sh.merge_range(r, 0, r, 5, t("doc_no_materials"), &st.cell)?;
        r += 1;
    }
    for (i, m) in used.iter().enumerate() {
        let mat = materials.iter().find(|x| x.id == m.material_id);
        sh.write_number_with_format(r, 0, i as f64 + 1.0, &st.cell_num)?;
        sh.write_string_with_format(
            r,
            1,
            mat.map(|x| format!("{} {}", x.code, x.name))
                .unwrap_or_default(),
            &st.cell,
        )?;
        sh.write_string_with_format(r, 2, mat.map_or("", |x| x.unit.as_str()), &st.cell)?;
        sh.write_number_with_format(r, 3, m.qty, &st.cell_num)?;
        sh.write_string_with_format(r, 4, mat.map_or("", |x| x.cert_no.as_str()), &st.cell)?;
        sh.write_string_with_format(r, 5, &m.document, &st.cell)?;
        r += 1;
    }

    r += 1;
    sh.write_string_with_format(r, 0, t("doc_verdict"), &st.label)?;
    sh.merge_range(r, 1, r, 5, t("doc_verdict_text"), &Format::new())?;
    r += 1;

    signatures(
        sh,
        &st,
        inp,
        r,
        &[
            PartyRole::Contractor,
            PartyRole::TechSupervision,
            PartyRole::AuthorSupervision,
        ],
    )?;
    wb.save(path)?;
    Ok(())
}

// ================================================================ Jadval eksporti

/// Ixtiyoriy jadvalni Excel ga chiqarish (umumiy talab).
///
/// Ekrandagi jadval qanday bo'lsa — shunday chiqadi: sarlavhalar birinchi
/// qatorda, sonlar son bo'lib qoladi (matn emas), shuning uchun Excel da
/// darrov yig'indi olish mumkin.
pub struct Table {
    pub name: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
}

/// Jadval katagi: matn yoki son.
#[derive(Debug, Clone)]
pub enum Cell {
    Text(String),
    Num(f64),
    Money(f64),
    Date(NaiveDate),
    Empty,
}

/// Jadvalni `.xlsx` ga yozadi.
pub fn write_table(path: &Path, table: &Table) -> Result<(), XlsxError> {
    let st = Styles::new();
    let mut wb = Workbook::new();
    let sh = wb.add_worksheet();
    // Excel varaq nomida `:\/?*[]` bo'lmasligi kerak va 31 belgidan oshmasligi.
    sh.set_name(sheet_name(&table.name))?;

    for (i, h) in table.headers.iter().enumerate() {
        sh.write_string_with_format(0, i as u16, h, &st.head)?;
        // Ustun kengligi sarlavha va qiymatlarga qarab.
        let w = table
            .rows
            .iter()
            .filter_map(|r| r.get(i))
            .map(|c| match c {
                Cell::Text(s) => s.chars().count(),
                Cell::Money(_) => 14,
                _ => 10,
            })
            .max()
            .unwrap_or(0)
            .max(h.chars().count())
            .min(50);
        sh.set_column_width(i as u16, w as f64 + 2.0)?;
    }
    sh.set_row_height(0, 26.0)?;

    let date_fmt = Format::new()
        .set_border(FormatBorder::Thin)
        .set_num_format("dd.mm.yyyy");
    for (ri, row) in table.rows.iter().enumerate() {
        let r = ri as u32 + 1;
        for (ci, cell) in row.iter().enumerate() {
            let c = ci as u16;
            match cell {
                Cell::Text(s) => sh.write_string_with_format(r, c, s, &st.cell)?,
                Cell::Num(v) => sh.write_number_with_format(r, c, *v, &st.cell_num)?,
                Cell::Money(v) => sh.write_number_with_format(r, c, *v, &st.cell_money)?,
                Cell::Date(d) => {
                    let d = rust_xlsxwriter::ExcelDateTime::from_ymd(
                        chrono::Datelike::year(d) as u16,
                        chrono::Datelike::month(d) as u8,
                        chrono::Datelike::day(d) as u8,
                    )?;
                    sh.write_datetime_with_format(r, c, &d, &date_fmt)?
                }
                Cell::Empty => sh.write_string_with_format(r, c, "", &st.cell)?,
            };
        }
    }
    // Sarlavha qatori qotib turadi va filtr qo'yiladi — katta jadvalda kerak.
    sh.set_freeze_panes(1, 0)?;
    if !table.rows.is_empty() && !table.headers.is_empty() {
        sh.autofilter(
            0,
            0,
            table.rows.len() as u32,
            table.headers.len() as u16 - 1,
        )?;
    }
    wb.save(path)?;
    Ok(())
}

/// Excel varaq nomi uchun xavfsiz matn.
fn sheet_name(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if ":\\/?*[]".contains(c) { ' ' } else { c })
        .collect();
    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() {
        return "Jadval".into();
    }
    cleaned.chars().take(31).collect()
}

/// Hujjat raqami: sana va turdan tuziladi.
///
/// Haqiqiy raqamlash tashkilotning ichki tartibiga bog'liq, shuning uchun bu
/// faqat taklif — hujjatda tahrirlash mumkin.
fn next_number(inp: &DocInput, prefix: &str) -> String {
    format!("{prefix}-{}", inp.to.format("%Y%m%d"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Project {
        Project {
            id: 1,
            name: "Sinov obyekti".into(),
            code: String::new(),
            address: "Toshkent".into(),
            object_type: String::new(),
            floors: 9,
            area_total: 0.0,
            status: crate::model::ObjectStatus::InProgress,
            start_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            planned_end: NaiveDate::from_ymd_opt(2026, 12, 1).unwrap(),
            contract_sum: 1_000_000.0,
            paid_total: 400_000.0,
            currency: "UZS".into(),
            funding_source: String::new(),
            notes: String::new(),
        }
    }

    fn task(id: i64, name: &str, volume: f64, progress: f64) -> Task {
        Task {
            id,
            project_id: 1,
            wbs: id.to_string(),
            name: name.into(),
            section: crate::model::Section::None,
            responsible: String::new(),
            duration: 10,
            plan_start: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            fact_start: None,
            fact_end: None,
            progress,
            pinned: false,
            volume,
            unit: "m3".into(),
        }
    }

    fn item(name: &str, price: f64) -> crate::domain::EstimateItem {
        crate::domain::EstimateItem {
            id: 1,
            estimate_id: 1,
            pos: 1,
            section: crate::model::Section::None,
            code: "E-1".into(),
            name: name.into(),
            unit: "m3".into(),
            qty: 100.0,
            price,
            cost: 0.0,
            task_id: None,
            note: String::new(),
        }
    }

    /// TZ IV.5: KS-2 hajmi ijro foizidan olinadi, narx smetadan.
    #[test]
    fn ks2_takes_volume_from_progress_and_price_from_estimate() {
        let from = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap();
        let tasks = vec![
            task(1, "Betonlash", 100.0, 40.0),
            // Boshlanmagan ish dalolatnomaga tushmaydi.
            task(2, "Pardozlash", 50.0, 0.0),
        ];
        let est = vec![item("betonlash", 1_000.0)];

        let lines = ks2_lines(&tasks, &est, from, to);
        assert_eq!(lines.len(), 1, "boshlanmagan ish kirmaydi");
        let l = &lines[0];
        assert_eq!(l.done_qty, 40.0);
        assert_eq!(l.price, 1_000.0, "nom katta-kichik harfsiz solishtiriladi");
        assert_eq!(l.cost, 40_000.0);
        assert_eq!(l.code, "E-1");
    }

    /// Smetada narx topilmasa — nol qoladi, o'ylab topilmaydi.
    #[test]
    fn ks2_leaves_price_empty_when_estimate_has_none() {
        let from = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap();
        let lines = ks2_lines(&[task(1, "Nomsiz ish", 10.0, 50.0)], &[], from, to);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].price, 0.0);
        assert_eq!(lines[0].cost, 0.0);
        assert!(lines[0].code.is_empty());
    }

    /// Davrdan tashqaridagi ish dalolatnomaga tushmaydi.
    #[test]
    fn ks2_respects_the_period() {
        let mut t = task(1, "Ish", 10.0, 100.0);
        t.plan_start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        t.duration = 5;
        let from = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap();
        assert!(ks2_lines(std::slice::from_ref(&t), &[], from, to).is_empty());

        // Davrga tushadigan ish esa chiqadi.
        let inside = task(2, "Ish 2", 10.0, 100.0);
        assert_eq!(ks2_lines(&[inside], &[], from, to).len(), 1);
    }

    /// KS-2 haqiqiy fayl bo'lib chiqadi va yakuniy summa qaytadi.
    #[test]
    fn ks2_is_written_and_returns_the_total() {
        let dir = std::env::temp_dir().join("qurai_docgen_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("ks2.xlsx");
        let _ = std::fs::remove_file(&path);

        let project = project();
        let tasks = vec![
            task(1, "Betonlash", 100.0, 40.0),
            task(2, "Qoplama", 20.0, 50.0),
        ];
        let est = vec![item("betonlash", 1_000.0)];
        let from = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap();
        let lines = ks2_lines(&tasks, &est, from, to);
        let inp = DocInput {
            project: &project,
            // Tomonlar kiritilmagan — hujjatda imzo joyi bo'sh qoladi.
            parties: &[],
            tasks: &tasks,
            today: to,
            from,
            to,
        };

        let total = write_ks2(&path, &inp, &lines).expect("yozildi");
        assert_eq!(total, 40_000.0, "narxsiz qator summaga qo'shilmaydi");
        let size = std::fs::metadata(&path).expect("fayl").len();
        assert!(size > 1000, "xlsx juda kichik: {size}");
        let _ = std::fs::remove_file(&path);
    }

    /// Yozilgan fayl haqiqiy Excel bo'lib ochiladi va ichida kutilgan qiymatlar
    /// turadi — bu yerda `calamine` bilan qayta o'qiymiz.
    ///
    /// Faqat "fayl yozildi" deb tekshirish yetarli emas: buzuq xlsx ham
    /// diskda joy egallaydi.
    #[test]
    fn written_ks2_can_be_read_back() {
        use calamine::{Data, Reader};

        let dir = std::env::temp_dir().join("qurai_docgen_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("ks2-roundtrip.xlsx");
        let _ = std::fs::remove_file(&path);

        let project = project();
        let tasks = vec![task(1, "Betonlash", 100.0, 40.0)];
        let est = vec![item("betonlash", 1_000.0)];
        let from = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap();
        let lines = ks2_lines(&tasks, &est, from, to);
        let inp = DocInput {
            project: &project,
            parties: &[],
            tasks: &tasks,
            today: to,
            from,
            to,
        };
        write_ks2(&path, &inp, &lines).expect("yozildi");

        let mut wb: calamine::Xlsx<_> = calamine::open_workbook(&path).expect("ochildi");
        let name = wb.sheet_names().first().cloned().expect("varaq");
        let sheet = wb.worksheet_range(&name).expect("varaq o'qildi");

        let text: Vec<String> = sheet
            .rows()
            .flat_map(|r| r.iter())
            .filter_map(|c| match c {
                Data::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        assert!(
            text.iter().any(|s| s == &project.name),
            "obyekt nomi hujjatda yo'q"
        );
        assert!(
            text.iter().any(|s| s.contains("Betonlash")),
            "ish nomi hujjatda yo'q"
        );

        // Yakuniy summa son bo'lib turishi kerak — matn emas.
        let nums: Vec<f64> = sheet
            .rows()
            .flat_map(|r| r.iter())
            .filter_map(|c| match c {
                Data::Float(v) => Some(*v),
                Data::Int(v) => Some(*v as f64),
                _ => None,
            })
            .collect();
        assert!(nums.contains(&40_000.0), "summa son bo'lib yozilmagan");
        assert!(nums.contains(&40.0), "bajarilgan hajm yo'q");

        let _ = std::fs::remove_file(&path);
    }

    /// Varaq nomi Excel qoidalariga moslanadi.
    #[test]
    fn sheet_name_is_safe_for_excel() {
        assert_eq!(sheet_name("Ombor: qoldiq/harakat"), "Ombor  qoldiq harakat");
        assert_eq!(sheet_name(""), "Jadval");
        assert_eq!(sheet_name("   "), "Jadval");
        assert_eq!(sheet_name(&"a".repeat(40)).chars().count(), 31);
    }

    /// Fayl haqiqatda yoziladi va bo'sh bo'lmaydi.
    #[test]
    fn table_is_written_to_a_real_file() {
        let dir = std::env::temp_dir().join("qurai_docgen_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("jadval.xlsx");
        let _ = std::fs::remove_file(&path);

        let table = Table {
            name: "Sinov".into(),
            headers: vec!["Nom".into(), "Miqdor".into(), "Summa".into()],
            rows: vec![
                vec![
                    Cell::Text("Beton".into()),
                    Cell::Num(12.5),
                    Cell::Money(1_000.0),
                ],
                vec![
                    Cell::Text("Armatura".into()),
                    Cell::Empty,
                    Cell::Date(NaiveDate::from_ymd_opt(2026, 5, 4).unwrap()),
                ],
            ],
        };
        write_table(&path, &table).expect("yozildi");
        let size = std::fs::metadata(&path).expect("fayl").len();
        assert!(size > 1000, "xlsx juda kichik: {size}");
        let _ = std::fs::remove_file(&path);
    }
}
