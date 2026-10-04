//! Loyiha varaqlarini AI bilan o'qish.
//!
//! Joylashuv qoidasi (`takeoff::tables`) matn parchalarining
//! koordinatasiga tayanadi va chizma dasturi jadvalni g'alati tergan joyda
//! adashadi. AI esa varaqni **ko'radi**: unga varaqning o'zi PDF holida va
//! dastur ajratib olgan matn beriladi, u spetsifikatsiya jadvallarini
//! qator-qator ko'chirib beradi.
//!
//! Ish taqsimoti qat'iy:
//!
//! - **AI o'qiydi** — jadvalni tuzilmaga soladi. Unga hisoblash
//!   taqiqlangan: sonlar varaqdagidek ko'chiriladi.
//! - **Dastur hisoblaydi** — ko'paytirish va yig'ish `takeoff` da, bitta
//!   joyda. AI o'qigan jadval ham, qoida o'qigani ham o'sha hisobdan
//!   o'tadi.
//! - **Ikkalasi solishtiriladi** — har varaq ikki mustaqil yo'l bilan
//!   o'qiladi; farq qilgan varaq odamga ko'rsatiladi.
//!
//! Bu faqat foydalanuvchi tugmani bosganda ishlaydi va varaqlarni tashqi
//! xizmatga yuboradi — ekranda shunday deb yozilgan. Kalit faqat
//! `Authorization` sarlavhasida ketadi.

use crate::llm::{self, Config, Part};
use crate::pdfread::Piece;
use crate::takeoff::{SpecRow, SpecTable};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

/// Bir vaqtda ketadigan so'rovlar soni.
const WORKERS: usize = 4;
/// Modelga beriladigan varaq matni chegarasi, belgida.
const MAX_TEXT: usize = 14_000;
/// Bundan katta varaq PDF holida yuborilmaydi — faqat matni.
const MAX_PDF: usize = 12 * 1024 * 1024;

/// Modelga ko'rsatma. Hujjat rus tilida, shuning uchun ko'rsatma ham.
pub const SYSTEM: &str = "Ты — инженер-сметчик. Тебе дан ОДИН лист проектной документации: \
PDF листа и текст, который программа извлекла из него (строки сверху вниз, ячейки разделены « | »). \
Перепиши ВСЕ таблицы спецификаций с этого листа в JSON.\n\
Правила:\n\
1. Числа переписывай точно как на листе. Ничего не вычисляй, не суммируй и не округляй.\n\
2. Ничего не добавляй от себя: чего нет на листе — того нет в ответе. Нечитаемую ячейку оставь пустой строкой.\n\
3. Сохраняй порядок строк. Строки-заголовки внутри таблицы («Фундамент Фм1 (на 1 шт.)», \
«Арматурная сетка С1», «Сборочные единицы», «Детали», «Материалы») передавай отдельной строкой, \
где заполнено только name (и qty/total, если они написаны в этой же строке).\n\
4. Название таблицы над ней («Спецификация элементов на каркас КП11-6») пиши в title, не в строках.\n\
5. Таблицу «Выборка металла» (столбцы Профиль, Материал, Масса общ.) с листов КМ/КМД \
передавай строками: designation=\"КМД\", name=\"<профиль> <марка стали>\", mass=<масса общая, кг>, unit=\"кг\". \
Строки «Итого» и «Всего» не включай.\n\
6. НЕ включай: штамп, примечания, экспликации помещений, ведомости чертежей и ссылочных документов, \
а также «Ведомость расхода стали» — это итог той же спецификации, а не отдельные материалы.\n\
7. owner — марка конструкции, которой посвящён весь лист, если она прямо написана в штампе или \
заголовке листа (например «К3»). Иначе — пустая строка.\n\
Поля строки: pos — Поз./Марка; designation — Обозначение; name — Наименование; qty — Кол.; \
mass — Масса ед., кг; total — масса общая либо число из столбца «Примечание»; \
unit — единица измерения, если указана (шт., м3, п.м., кг, м, м2, т).\n\
Ответ — только JSON такого вида:\n\
{\"sheet\":\"название листа\",\"owner\":\"\",\"tables\":[{\"title\":\"\",\"rows\":[{\"pos\":\"\",\
\"designation\":\"\",\"name\":\"\",\"qty\":\"\",\"mass\":\"\",\"total\":\"\",\"unit\":\"\"}]}]}\n\
Если таблиц спецификаций на листе нет — верни tables: [].";

/// Varaq matnini qatorlarga teradi: yuqoridan pastga, kataklar ` | `
/// bilan ajratilgan.
///
/// Model PDF ni o'zi ko'radi; bu matn — sonlarni aniq ko'chirishi uchun
/// tayanch (rasmdan o'qilgan raqam adashishi mumkin, matn qatlami — yo'q).
pub fn page_text(pieces: &[Piece]) -> String {
    let mut out = String::new();
    for dir in 0..4u8 {
        let mut part: Vec<&Piece> = pieces.iter().filter(|p| p.dir == dir).collect();
        if part.is_empty() {
            continue;
        }
        part.sort_by(|a, b| b.y.total_cmp(&a.y).then(a.x.total_cmp(&b.x)));
        let mut lines: Vec<Vec<&Piece>> = Vec::new();
        for p in part {
            match lines.last_mut() {
                Some(l) if (l[0].y - p.y).abs() <= p.size * 0.5 => l.push(p),
                _ => lines.push(vec![p]),
            }
        }
        for mut l in lines {
            l.sort_by(|a, b| a.x.total_cmp(&b.x));
            let mut end = f64::MIN;
            for p in l {
                let gap = p.x - end;
                if end != f64::MIN {
                    if gap > p.size * 1.5 {
                        out.push_str(" | ");
                    } else if gap > p.size * 0.33 {
                        out.push(' ');
                    }
                }
                out.push_str(p.text.trim());
                end = p.x + p.text.chars().count() as f64 * p.size * 0.5;
            }
            out.push('\n');
            if out.len() > MAX_TEXT {
                // Belgi chegarasida kesiladi — UTF-8 buzilmasin.
                let cut: String = out.chars().take(MAX_TEXT).collect();
                return cut;
            }
        }
    }
    out
}

/// Varaqda spetsifikatsiya jadvali bo'lishi mumkinmi.
///
/// Reja va qirqim varaqlari AI ga yuborilmaydi: ularda o'qiydigan jadval
/// yo'q, so'rov esa pul va vaqt.
pub fn wants(pieces: &[Piece]) -> bool {
    let has = |word: &str| {
        pieces
            .iter()
            .any(|p| p.text.trim().to_lowercase().starts_with(word))
    };
    (has("наимен") && (has("поз") || has("кол"))) || (has("профиль") && has("материал"))
}

/// Bitta varaqdan iborat PDF.
pub fn page_pdf(doc: &lopdf::Document, page: u32) -> Option<Vec<u8>> {
    let mut one = doc.clone();
    let others: Vec<u32> = one
        .get_pages()
        .keys()
        .copied()
        .filter(|n| *n != page)
        .collect();
    one.delete_pages(&others);
    one.prune_objects();
    let mut out = Vec::new();
    one.save_to(&mut out).ok()?;
    (!out.is_empty()).then_some(out)
}

/// JSON qiymatini matnga: model sonni son qilib ham, satr qilib ham
/// berishi mumkin.
fn cell(v: Option<&serde_json::Value>) -> String {
    match v {
        Some(serde_json::Value::String(s)) => s.trim().to_string(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// Model javobini jadvallarga aylantiradi: `(ega, jadvallar)`.
///
/// Javob JSON bo'lmasa yoki kutilgan shaklda bo'lmasa — xato; taxmin
/// qilib tuzatilmaydi.
pub fn parse(body: &str, page: usize) -> Result<(String, Vec<SpecTable>), String> {
    // Ba'zi modellar JSON ni ``` ichiga o'raydi.
    let text = body.trim();
    let text = text
        .strip_prefix("```json")
        .or_else(|| text.strip_prefix("```"))
        .unwrap_or(text);
    let text = text.strip_suffix("```").unwrap_or(text).trim();
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("JSON: {e}"))?;
    let tables = v
        .get("tables")
        .and_then(|t| t.as_array())
        .ok_or_else(|| "JSON: tables yo'q".to_string())?;
    // Ega — belgining o'zi: `К3(2К80-6М3-с-а)` → `К3`.
    let owner = cell(v.get("owner"))
        .split(['(', ' ', ','])
        .next()
        .unwrap_or_default()
        .to_string();
    let mut out = Vec::new();
    for t in tables {
        let mut rows = Vec::new();
        // Jadval nomi — sarlavha qatori: unda konstruksiya belgisi
        // bo'lishi mumkin («Спецификация на Фм1»).
        let title = cell(t.get("title"));
        if !title.is_empty() {
            rows.push(SpecRow {
                name: title,
                ..Default::default()
            });
        }
        for r in t
            .get("rows")
            .and_then(|r| r.as_array())
            .into_iter()
            .flatten()
        {
            let total = cell(r.get("total"));
            let unit = cell(r.get("unit"));
            let row = SpecRow {
                pos: cell(r.get("pos")),
                designation: cell(r.get("designation")),
                name: cell(r.get("name"))
                    .replace('∅', "Ø")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" "),
                qty: cell(r.get("qty")),
                mass: cell(r.get("mass")),
                // Hisob «Примеч.» ustunidan jami massani yoki birlikni
                // kutadi: `68.0`, `шт.`, `294 п.м.`.
                note: format!("{total} {unit}").trim().to_string(),
                unit,
            };
            if row != SpecRow::default() {
                rows.push(row);
            }
        }
        // Po'lat sarfi vedomosti — shu varaqdagi spetsifikatsiyaning
        // yig'indisi. Model uni ko'rsatmaga qaramay bersa ham, u hisobga
        // kirmaydi: aks holda armatura ikki marta sanalardi. Belgisi —
        // «metall tanlovi» ichida armatura diametri.
        let summary = rows
            .iter()
            .any(|r| r.designation == crate::takeoff::KMD && r.name.trim_start().starts_with('Ø'));
        if summary {
            continue;
        }
        // Faqat sarlavhadan iborat jadval — bo'sh.
        if rows
            .iter()
            .any(|r| !r.qty.is_empty() || !r.mass.is_empty() || !r.note.is_empty())
        {
            out.push(SpecTable {
                page,
                rows,
                ai: true,
                ..Default::default()
            });
        }
    }
    Ok((owner, out))
}

/// Bitta varaq natijasi.
#[derive(Debug, Clone)]
pub struct PageResult {
    pub owner: String,
    pub tables: Vec<SpecTable>,
    pub tokens: u32,
}

/// Fon ishidan keladigan xabar.
pub enum Msg {
    /// Qaysi varaqlar o'qiladi.
    Plan(Vec<usize>),
    Page(usize, Result<PageResult, String>),
    /// Ish boshlanmadi: fayl ochilmadi.
    Failed(String),
    Done,
}

/// Bitta varaqni o'qiydi.
fn one(
    cfg: &Config,
    doc: &lopdf::Document,
    pieces: &[Piece],
    page: usize,
) -> Result<PageResult, String> {
    let text = format!(
        "Лист {page}. Текст, извлечённый программой:\n{}",
        page_text(pieces)
    );
    let pdf = page_pdf(doc, page as u32).filter(|d| d.len() <= MAX_PDF);
    let ask = |with_pdf: bool| {
        let mut parts = vec![Part::Text(text.clone())];
        if let (true, Some(data)) = (with_pdf, &pdf) {
            parts.push(Part::Pdf {
                name: format!("list-{page}.pdf"),
                data: data.clone(),
            });
        }
        llm::extract(cfg, SYSTEM, &parts)
    };
    let mut result = ask(true);
    // Model fayl qabul qilmasa — faqat matn bilan; vaqtinchalik xatoda —
    // bir marta qayta.
    if let Err(e) = &result {
        if matches!(e, llm::Error::BadRequest(_)) && pdf.is_some() {
            result = ask(false);
        } else if e.retryable() {
            std::thread::sleep(std::time::Duration::from_secs(3));
            result = ask(true);
        }
    }
    let answer = result.map_err(|e| e.to_string())?;
    let (owner, tables) = parse(&answer.text, page)?;
    Ok(PageResult {
        owner,
        tables,
        tokens: answer.usage.total,
    })
}

/// O'qishni fon iplarida boshlaydi.
///
/// `limit` — faqat birinchi N ta jadvalli varaq (sinab ko'rish uchun).
/// `cancel` ko'tarilsa, navbatdagi varaqlar boshlanmaydi.
pub fn spawn(
    cfg: Config,
    path: PathBuf,
    limit: Option<usize>,
    cancel: Arc<AtomicBool>,
) -> Receiver<Msg> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let pages = match crate::pdfread::pages(&path) {
            Ok(p) => p,
            Err(e) => {
                let _ = tx.send(Msg::Failed(e));
                return;
            }
        };
        let doc = match lopdf::Document::load(&path) {
            Ok(d) => d,
            Err(e) => {
                let _ = tx.send(Msg::Failed(e.to_string()));
                return;
            }
        };
        let mut plan: Vec<usize> = pages
            .iter()
            .enumerate()
            .filter(|(_, p)| wants(p))
            .map(|(i, _)| i + 1)
            .collect();
        if let Some(n) = limit {
            plan.truncate(n);
        }
        let _ = tx.send(Msg::Plan(plan.clone()));
        let queue = Mutex::new(VecDeque::from(plan));
        std::thread::scope(|s| {
            for _ in 0..WORKERS {
                s.spawn(|| loop {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let Some(page) = queue.lock().ok().and_then(|mut q| q.pop_front()) else {
                        break;
                    };
                    let result = one(&cfg, &doc, &pages[page - 1], page);
                    if tx.send(Msg::Page(page, result)).is_err() {
                        break;
                    }
                });
            }
        });
        let _ = tx.send(Msg::Done);
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64, text: &str) -> Piece {
        Piece {
            x,
            y,
            size: 10.0,
            text: text.to_string(),
            dir: 0,
        }
    }

    #[test]
    fn the_sheet_text_keeps_rows_and_cells() {
        let pieces = vec![
            p(200.0, 100.0, "Наименование"),
            p(100.0, 100.0, "Поз."),
            p(104.0, 80.0, "Фм"),
            p(114.0, 80.0, "1"),
            p(200.0, 80.0, "Фундамент монолитный"),
            p(400.0, 80.0, "20"),
        ];
        assert_eq!(
            page_text(&pieces),
            "Поз. | Наименование\nФм1 | Фундамент монолитный | 20\n"
        );
        assert!(wants(&pieces));
        assert!(!wants(&[p(0.0, 0.0, "План на отм. 0.000")]));
    }

    /// Model javobi hisob kutgan shaklga keladi va o'sha hisobdan o'tadi.
    #[test]
    fn a_reply_becomes_tables_the_engine_can_count() {
        let body = r#"```json
{"sheet":"Фм3","owner":"Фм3","tables":[
 {"title":"Спецификация","rows":[
   {"pos":"","designation":"","name":"Фундамент Фм3 (на 1 шт.)","qty":"","mass":"","total":"","unit":""},
   {"pos":"1","designation":"ГОСТ 5781-82","name":"∅14 A-III L=2550","qty":22,"mass":3.09,"total":"68.0","unit":""},
   {"pos":"","designation":"","name":"Бетон кл. В20","qty":"3.66","mass":"","total":"","unit":"м3"}
 ]},
 {"title":"Пустая","rows":[]}
]}
```"#;
        let (owner, tables) = parse(body, 13).expect("jadval");
        assert_eq!(owner, "Фм3");
        assert_eq!(tables.len(), 1, "bo'sh jadval tashlanadi");
        let t = &tables[0];
        assert!(t.ai && t.page == 13);
        assert_eq!(t.rows[0].name, "Спецификация");
        assert_eq!(t.rows[2].qty, "22");
        assert_eq!(t.rows[2].note, "68.0");
        let rebar = crate::takeoff::item(&t.rows[2]).expect("armatura");
        assert_eq!(rebar.material, "Armatura Ø14 A-III");
        assert_eq!(rebar.amount, 68.0);
        let beton = crate::takeoff::item(&t.rows[3]).expect("beton");
        assert_eq!((beton.material.as_str(), beton.amount), ("Beton B20", 3.66));
    }

    /// Po'lat sarfi vedomosti ikkinchi marta sanalmaydi; ega — belgi.
    #[test]
    fn a_steel_summary_is_dropped_and_the_owner_is_a_mark() {
        let body = r#"{"owner":"К3(2К80-6М3-c-а)","tables":[
 {"title":"Ведомость расхода стали","rows":[
   {"designation":"КМД","name":"Ø32 A-III ГОСТ 5781-82","mass":"375.6","unit":"кг"}]},
 {"title":"Выборка металла","rows":[
   {"designation":"КМД","name":"□80х80х4 С255","mass":"3856,8","unit":"кг"}]},
 {"title":"","rows":[
   {"pos":"10","name":"Лист 12x390x680\nС235","qty":"1","mass":"25","total":"25.0","unit":"кг"}]}
]}"#;
        let (owner, tables) = parse(body, 19).unwrap();
        assert_eq!(owner, "К3");
        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].rows[1].designation, crate::takeoff::KMD);
        assert_eq!(tables[1].rows[0].name, "Лист 12x390x680 С235");
    }

    /// Buzuq javob taxmin qilib tuzatilmaydi.
    #[test]
    fn a_broken_reply_is_an_error_not_a_guess() {
        assert!(parse("jadval topilmadi", 1).is_err());
        assert!(parse(r#"{"sheet":"x"}"#, 1).is_err());
        assert_eq!(parse(r#"{"tables":[]}"#, 1).unwrap().1.len(), 0);
    }

    /// O'chiq sozlamada tarmoqqa chiqilmaydi: har varaq xato bilan qaytadi
    /// va ish tugaydi.
    #[test]
    fn a_missing_file_reports_and_stops() {
        let rx = spawn(
            Config::default(),
            PathBuf::from("yoq-fayl.pdf"),
            None,
            Arc::new(AtomicBool::new(false)),
        );
        assert!(matches!(rx.recv(), Ok(Msg::Failed(_))));
    }
}

/// Jonli tekshiruv: haqiqiy xizmat bilan bir necha varaqni o'qiydi.
///
/// Odatdagi sinovlarda ishlamaydi (`#[ignore]`) — u tarmoqqa chiqadi va
/// pullik. Sozlama bazadan olinadi; kalit hech qayerga chiqarilmaydi.
/// Ishga tushirish: `QURAI_LIVE_DB`, `QURAI_LIVE_PDF`, `QURAI_LIVE_PAGES`
/// (`13,19`) va ixtiyoriy `QURAI_LIVE_MODEL`.
#[cfg(test)]
mod live {
    #[test]
    #[ignore]
    fn read_real_sheets() {
        let (Ok(db), Ok(pdf), Ok(list)) = (
            std::env::var("QURAI_LIVE_DB"),
            std::env::var("QURAI_LIVE_PDF"),
            std::env::var("QURAI_LIVE_PAGES"),
        ) else {
            return;
        };
        let db = crate::db::Db::open(&std::path::PathBuf::from(&db)).expect("baza");
        let mut cfg = crate::llm::Config::default();
        cfg.set_key(&db.get_setting("llm_key").unwrap_or_default());
        cfg.model = std::env::var("QURAI_LIVE_MODEL")
            .ok()
            .or_else(|| db.get_setting("llm_model"))
            .unwrap_or(cfg.model);
        let path = std::path::Path::new(&pdf);
        let pages = crate::pdfread::pages(path).expect("pdf");
        let doc = lopdf::Document::load(path).expect("pdf");
        let mut out = format!("model {}\n", cfg.model);
        for page in list
            .split(',')
            .filter_map(|p| p.trim().parse::<usize>().ok())
        {
            let t0 = std::time::Instant::now();
            match super::one(&cfg, &doc, &pages[page - 1], page) {
                Ok(r) => {
                    out.push_str(&format!(
                        "\n# varaq {page}: {} jadval, ega «{}», {} token, {:.0} s\n",
                        r.tables.len(),
                        r.owner,
                        r.tokens,
                        t0.elapsed().as_secs_f64()
                    ));
                    for t in &r.tables {
                        out.push_str("  --\n");
                        for r in &t.rows {
                            out.push_str(&format!(
                                "  {} | {} | {} | {} | {} | {}\n",
                                r.pos, r.designation, r.name, r.qty, r.mass, r.note
                            ));
                        }
                    }
                }
                Err(e) => out.push_str(&format!("\n# varaq {page}: XATO {e}\n")),
            }
        }
        std::fs::write(std::env::temp_dir().join("qurai_live.txt"), out).expect("yozish");
    }
}
