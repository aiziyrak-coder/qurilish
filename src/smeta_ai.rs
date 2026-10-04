//! Smeta yo'lining AI qismi: varaqlarni o'qish, savollar, spetsifikatsiya.
//!
//! Ish taqsimoti [`crate::smeta`] dagidek: AI **tuzilmani to'ldiradi**,
//! sonni dastur hisoblaydi, narxni katalog beradi. Bu modul uch so'rov
//! turini biladi:
//!
//! 1. **Varaq** — har varaq alohida: varaqning PDF'i va matni beriladi,
//!    AI undan nimani ko'rganini qaytaradi: varaq nomi, bo'lim, o'lcham va
//!    maydonlar, vedomostlar, spetsifikatsiya jadvallari (qator-qator).
//! 2. **Savollar** — barcha varaqlar xulosasi beriladi, AI loyihada
//!    ko'rinmagan narsalarni so'raydi va loyihada bor javoblarni o'zi
//!    to'ldiradi.
//! 3. **Spetsifikatsiya** — avval bosqichlar ro'yxati, keyin har bosqich
//!    alohida: ishlar, miqdor, manba, formula, materiallar.
//!
//! Har so'rov faqat foydalanuvchi tugma bosganda ketadi; kalit faqat
//! `Authorization` sarlavhasida.

use crate::llm::{self, Config, Part};
use crate::smeta::{eval, Fact, List, PageDigest, Question, Resource, Source, Stage, Work};
use crate::takeoff::SpecTable;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

const WORKERS: usize = 4;

// ================================================================ 1. Varaq

const PAGE_SYSTEM: &str = "Ты — инженер ПТО. Тебе дан ОДИН лист проектной документации: PDF листа и текст, \
который программа извлекла из него (строки сверху вниз, ячейки разделены « | »). Опиши, что на листе, в JSON.\n\
Правила: числа переписывай точно как на листе, ничего не вычисляй и не придумывай; чего нет на листе — того нет в ответе.\n\
Поля:\n\
sheet — название листа из штампа или заголовка.\n\
kind — раздел: ГП, АР, КЖ, КМ, КМД, ВК, ОВ, ЭОМ, ПЗ, ТХ или другое.\n\
facts — размеры и показатели, прямо написанные на листе: площадь застройки, общая площадь, строительный объём, \
этажность, высота этажа, размеры в осях, шаг колонн, отметки, количество квартир/помещений, толщина стен, \
марка бетона, класс арматуры, тип кровли, тип фундамента и т.п. Формат: {\"name\":\"\",\"value\":\"\",\"unit\":\"\"}.\n\
lists — таблицы, кроме спецификаций конструкций: ведомости объёмов работ, экспликации помещений с площадями, \
ведомости отделки, ведомости полов, проёмов, перемычек, расхода стали. Формат: {\"title\":\"\",\"columns\":[],\"rows\":[[]]}, \
каждая строка — массив ячеек по столбцам.\n\
tables — спецификации конструкций и изделий (столбцы Поз./Марка, Обозначение, Наименование, Кол., Масса, Примечание) \
и «Выборка металла» с листов КМ/КМД (designation=\"КМД\", name=\"<профиль> <марка стали>\", mass=<масса общая>, unit=\"кг\"; \
строки Итого не включай). Строки-заголовки внутри таблицы передавай отдельной строкой с заполненным только name. \
Формат строки: {\"pos\":\"\",\"designation\":\"\",\"name\":\"\",\"qty\":\"\",\"mass\":\"\",\"total\":\"\",\"unit\":\"\"}; \
total — масса общая либо число из столбца «Примечание». Название таблицы — в title.\n\
owner — марка конструкции, которой посвящён весь лист, если она прямо написана в штампе (например «К3»), иначе \"\".\n\
Штамп, примечания, ведомости чертежей и ссылочных документов не включай.\n\
Ответ — только JSON: {\"sheet\":\"\",\"kind\":\"\",\"owner\":\"\",\"facts\":[],\"lists\":[],\"tables\":[]}";

/// Bitta varaq natijasi.
#[derive(Debug, Clone, Default)]
pub struct PageOut {
    pub digest: PageDigest,
    pub owner: String,
    pub tables: Vec<SpecTable>,
    pub tokens: u32,
}

fn cell(v: Option<&serde_json::Value>) -> String {
    match v {
        Some(serde_json::Value::String(s)) => s.split_whitespace().collect::<Vec<_>>().join(" "),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// Varaq javobini o'qiydi.
pub fn parse_page(body: &str, page: usize) -> Result<PageOut, String> {
    let (owner, tables) = crate::aitake::parse(body, page)?;
    let text = body.trim();
    let text = text
        .strip_prefix("```json")
        .or_else(|| text.strip_prefix("```"))
        .unwrap_or(text);
    let text = text.strip_suffix("```").unwrap_or(text);
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("JSON: {e}"))?;
    let arr = |k: &str| {
        v.get(k)
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default()
    };
    let facts = arr("facts")
        .iter()
        .map(|f| Fact {
            name: cell(f.get("name")),
            value: cell(f.get("value")),
            unit: cell(f.get("unit")),
            page: Some(page),
        })
        .filter(|f| !f.name.is_empty() && !f.value.is_empty())
        .collect();
    let lists = arr("lists")
        .iter()
        .map(|l| List {
            title: cell(l.get("title")),
            columns: l
                .get("columns")
                .and_then(|c| c.as_array())
                .map(|c| c.iter().map(|x| cell(Some(x))).collect())
                .unwrap_or_default(),
            rows: l
                .get("rows")
                .and_then(|r| r.as_array())
                .map(|rows| {
                    rows.iter()
                        .filter_map(|r| r.as_array())
                        .map(|r| r.iter().map(|x| cell(Some(x))).collect())
                        .collect()
                })
                .unwrap_or_default(),
        })
        .filter(|l| !l.rows.is_empty())
        .collect();
    Ok(PageOut {
        digest: PageDigest {
            page,
            sheet: cell(v.get("sheet")),
            kind: cell(v.get("kind")),
            facts,
            lists,
        },
        owner,
        tables,
        tokens: 0,
    })
}

/// Fon ishidan keladigan xabar — varaqlar.
pub enum PageMsg {
    Plan(Vec<usize>),
    Page(usize, Result<PageOut, String>),
    Failed(String),
    Done,
}

/// Varaqda o'qiydigan narsa bormi: matni bor, lekin faqat o'lchamlardan
/// iborat emas.
fn readable(pieces: &[crate::pdfread::Piece]) -> bool {
    let words = pieces
        .iter()
        .filter(|p| p.text.chars().any(|c| c.is_alphabetic()))
        .count();
    words >= 8
}

/// Barcha o'qiladigan varaqlarni fon iplarida AI ga beradi.
pub fn spawn_pages(cfg: Config, path: PathBuf, cancel: Arc<AtomicBool>) -> Receiver<PageMsg> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let pages = match crate::pdfread::pages(&path) {
            Ok(p) => p,
            Err(e) => {
                let _ = tx.send(PageMsg::Failed(e));
                return;
            }
        };
        let doc = match lopdf::Document::load(&path) {
            Ok(d) => d,
            Err(e) => {
                let _ = tx.send(PageMsg::Failed(e.to_string()));
                return;
            }
        };
        let plan: Vec<usize> = pages
            .iter()
            .enumerate()
            .filter(|(_, p)| readable(p))
            .map(|(i, _)| i + 1)
            .collect();
        let _ = tx.send(PageMsg::Plan(plan.clone()));
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
                    let text = format!(
                        "Лист {page}. Текст, извлечённый программой:\n{}",
                        crate::aitake::page_text(&pages[page - 1])
                    );
                    let mut parts = vec![Part::Text(text)];
                    if let Some(data) = crate::aitake::page_pdf(&doc, page as u32) {
                        parts.push(Part::Pdf {
                            name: format!("list-{page}.pdf"),
                            data,
                        });
                    }
                    let mut result = llm::extract(&cfg, PAGE_SYSTEM, &parts);
                    if let Err(e) = &result {
                        if e.retryable() {
                            std::thread::sleep(std::time::Duration::from_secs(3));
                            result = llm::extract(&cfg, PAGE_SYSTEM, &parts);
                        }
                    }
                    let out = result.map_err(|e| e.to_string()).and_then(|a| {
                        parse_page(&a.text, page).map(|mut p| {
                            p.tokens = a.usage.total;
                            p
                        })
                    });
                    if tx.send(PageMsg::Page(page, out)).is_err() {
                        break;
                    }
                });
            }
        });
        let _ = tx.send(PageMsg::Done);
    });
    rx
}

// ============================================================= 2. Savollar

const QUESTIONS_SYSTEM: &str = "Ты — опытный сметчик. Тебе дана выжимка из листов проекта: показатели, ведомости, \
список конструкций и материалов, которые программа уже сняла с проекта. Нужно:\n\
1. summary — 3–5 предложений: что за объект, конструктив, этажность, площади, из чего фундамент, каркас, стены, \
перекрытия, кровля, инженерные разделы. Только то, что есть в данных.\n\
2. questions — вопросы о том, чего в проекте НЕ видно, но без чего смету не собрать: состав работ по разделам, \
отделка, инженерные сети, благоустройство, условия (грунт, доставка, подъезд), границы договора. На каждый вопрос \
дай 2–5 вариантов ответа, последний — «уточнить у заказчика». Если ответ прямо есть в данных — впиши его в answer \
и укажи лист в тексте вопроса; иначе answer=\"\". Не больше 20 вопросов, без повторов.\n\
Ответ — только JSON: {\"summary\":\"\",\"questions\":[{\"topic\":\"\",\"text\":\"\",\"options\":[],\"answer\":\"\"}]}";

/// Savollar javobini o'qiydi: `(xulosa, savollar)`.
pub fn parse_questions(body: &str) -> Result<(String, Vec<Question>), String> {
    let v: serde_json::Value =
        serde_json::from_str(strip(body)).map_err(|e| format!("JSON: {e}"))?;
    let qs = v
        .get("questions")
        .and_then(|q| q.as_array())
        .ok_or_else(|| "JSON: questions yo'q".to_string())?
        .iter()
        .map(|q| Question {
            topic: cell(q.get("topic")),
            text: cell(q.get("text")),
            options: q
                .get("options")
                .and_then(|o| o.as_array())
                .map(|o| {
                    o.iter()
                        .map(|x| cell(Some(x)))
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            answer: cell(q.get("answer")),
        })
        .filter(|q| !q.text.is_empty())
        .collect();
    Ok((cell(v.get("summary")), qs))
}

fn strip(body: &str) -> &str {
    let text = body.trim();
    let text = text
        .strip_prefix("```json")
        .or_else(|| text.strip_prefix("```"))
        .unwrap_or(text);
    text.strip_suffix("```").unwrap_or(text).trim()
}

/// Savollarni fonda so'raydi.
pub fn spawn_questions(cfg: Config, context: String) -> crate::app::QuestionsRx {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let out = llm::extract(&cfg, QUESTIONS_SYSTEM, &[Part::Text(context)])
            .map_err(|e| e.to_string())
            .and_then(|a| parse_questions(&a.text).map(|(s, q)| (s, q, a.usage.total)));
        let _ = tx.send(out);
    });
    rx
}

// ====================================================== 3. Spetsifikatsiya

const STAGES_SYSTEM: &str = "Ты — опытный сметчик. По данным объекта составь список этапов строительства в \
технологическом порядке (земляные работы, фундаменты, каркас, стены, перекрытия, кровля, проёмы, инженерные сети, \
отделка, благоустройство — только те, что относятся к этому объекту и к границам договора из ответов). \
Для каждого этапа — короткое название и одно предложение, что в него входит. Не больше 15 этапов.\n\
Ответ — только JSON: {\"stages\":[{\"name\":\"\",\"scope\":\"\"}]}";

const STAGE_SYSTEM: &str = "Ты — опытный сметчик. Тебе даны данные объекта (показатели с листов, ответы заказчика, \
материалы и конструкции, снятые программой с проекта) и ОДИН этап. Разверни этап в работы и материалы.\n\
Правила:\n\
1. Каждая работа: name, qty, unit, source, materials. Единицы — м3, м2, м, шт, т, кг, компл.\n\
2. source — откуда количество:\n\
   \"project\" — число взято из материалов/конструкций проекта или ведомости; укажи page (номер листа).\n\
   \"calc\" — посчитано из размеров объекта; ОБЯЗАТЕЛЬНО дай formula из чисел и знаков + - * / ( ), например \"2*(60+24)*4.2\", \
   и в note напиши, что означают числа. Программа сама вычислит формулу; qty должен ей соответствовать.\n\
   \"standard\" — норма расхода или комплектность по технологии; в note — норма (например «1,02 м3 бетона на 1 м3 конструкции»).\n\
   \"assumption\" — данных не хватило, величина принята; в note — почему и что уточнить.\n\
3. Материалы из проекта используй как есть, с source=\"project\" и листом — они точнее нормативов. Не дублируй один и тот же \
материал в разных работах, если в проекте он дан общим количеством: отнеси его к одной работе.\n\
4. Ничего не цени: цен в ответе нет.\n\
5. К материалам относи также технику и доставку отдельными строками (unit смена, рейс), если они нужны этапу.\n\
Ответ — только JSON: {\"works\":[{\"name\":\"\",\"qty\":0,\"unit\":\"\",\"source\":\"\",\"page\":null,\"formula\":\"\",\"note\":\"\",\
\"materials\":[{\"name\":\"\",\"qty\":0,\"unit\":\"\",\"source\":\"\",\"page\":null,\"formula\":\"\",\"note\":\"\"}]}]}";

fn source_of(v: &serde_json::Value, qty: &mut f64) -> Source {
    let page = v.get("page").and_then(|p| p.as_u64()).map(|p| p as usize);
    let formula = cell(v.get("formula"));
    let note = cell(v.get("note"));
    match cell(v.get("source")).as_str() {
        "project" => Source::Project { page },
        "calc" => match eval(&formula) {
            // Son formuladan chiqadi — AI yozgan qiymat emas.
            Some(x) if x > 0.0 => {
                *qty = x;
                Source::Calc {
                    formula: if note.is_empty() {
                        formula
                    } else {
                        format!("{formula} — {note}")
                    },
                }
            }
            _ => Source::Assumption {
                note: format!("{} ({formula})", crate::i18n::t("sm_formula_bad")),
            },
        },
        "standard" => Source::Standard { note },
        _ => Source::Assumption { note },
    }
}

fn number(v: Option<&serde_json::Value>) -> f64 {
    match v {
        Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(serde_json::Value::String(s)) => crate::takeoff::number(s).unwrap_or(0.0),
        _ => 0.0,
    }
}

/// Bosqichlar ro'yxati javobi.
pub fn parse_stages(body: &str) -> Result<Vec<(String, String)>, String> {
    let v: serde_json::Value =
        serde_json::from_str(strip(body)).map_err(|e| format!("JSON: {e}"))?;
    let list = v
        .get("stages")
        .and_then(|s| s.as_array())
        .ok_or_else(|| "JSON: stages yo'q".to_string())?
        .iter()
        .map(|s| (cell(s.get("name")), cell(s.get("scope"))))
        .filter(|s| !s.0.is_empty())
        .collect();
    Ok(list)
}

/// Bitta bosqich javobi: ishlar va materiallar.
pub fn parse_stage(body: &str, name: &str) -> Result<Stage, String> {
    let v: serde_json::Value =
        serde_json::from_str(strip(body)).map_err(|e| format!("JSON: {e}"))?;
    let works = v
        .get("works")
        .and_then(|w| w.as_array())
        .ok_or_else(|| "JSON: works yo'q".to_string())?
        .iter()
        .filter_map(|w| {
            let mut qty = number(w.get("qty"));
            let source = source_of(w, &mut qty);
            let name = cell(w.get("name"));
            if name.is_empty() || qty <= 0.0 {
                return None;
            }
            let materials = w
                .get("materials")
                .and_then(|m| m.as_array())
                .map(|ms| {
                    ms.iter()
                        .filter_map(|m| {
                            let mut qty = number(m.get("qty"));
                            let source = source_of(m, &mut qty);
                            let name = cell(m.get("name"));
                            (!name.is_empty() && qty > 0.0).then(|| Resource {
                                name,
                                qty,
                                unit: cell(m.get("unit")),
                                source,
                                price: None,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(Work {
                name,
                qty,
                unit: cell(w.get("unit")),
                source,
                price: None,
                materials,
            })
        })
        .collect();
    Ok(Stage {
        name: name.to_string(),
        works,
        markup: 0.0,
    })
}

pub enum StageMsg {
    Plan(Vec<String>),
    Stage(usize, Result<Stage, String>),
    Failed(String),
    Done,
}

/// Spetsifikatsiyani fonda tuzadi: avval bosqichlar, keyin har biri.
pub fn spawn_spec(cfg: Config, context: String, cancel: Arc<AtomicBool>) -> Receiver<StageMsg> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let stages = match llm::extract(&cfg, STAGES_SYSTEM, &[Part::Text(context.clone())])
            .map_err(|e| e.to_string())
            .and_then(|a| parse_stages(&a.text))
        {
            Ok(s) if !s.is_empty() => s,
            Ok(_) => {
                let _ = tx.send(StageMsg::Failed(crate::i18n::t("sm_no_stages").to_string()));
                return;
            }
            Err(e) => {
                let _ = tx.send(StageMsg::Failed(e));
                return;
            }
        };
        let _ = tx.send(StageMsg::Plan(stages.iter().map(|s| s.0.clone()).collect()));
        let all: Vec<String> = stages.iter().map(|(n, s)| format!("- {n}: {s}")).collect();
        let queue = Mutex::new(VecDeque::from(
            stages.into_iter().enumerate().collect::<Vec<_>>(),
        ));
        std::thread::scope(|s| {
            for _ in 0..WORKERS {
                s.spawn(|| loop {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let Some((i, (name, scope))) =
                        queue.lock().ok().and_then(|mut q| q.pop_front())
                    else {
                        break;
                    };
                    let text = format!(
                        "{context}\n\nВсе этапы объекта (чтобы не дублировать работы между ними):\n{}\n\n\
                         ЭТАП ДЛЯ РАЗВЁРТКИ: {name} — {scope}",
                        all.join("\n")
                    );
                    let mut result = llm::extract(&cfg, STAGE_SYSTEM, &[Part::Text(text.clone())]);
                    if let Err(e) = &result {
                        if e.retryable() {
                            std::thread::sleep(std::time::Duration::from_secs(3));
                            result = llm::extract(&cfg, STAGE_SYSTEM, &[Part::Text(text)]);
                        }
                    }
                    let out = result
                        .map_err(|e| e.to_string())
                        .and_then(|a| parse_stage(&a.text, &name));
                    if tx.send(StageMsg::Stage(i, out)).is_err() {
                        break;
                    }
                });
            }
        });
        let _ = tx.send(StageMsg::Done);
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_reply_gives_facts_lists_and_tables() {
        let body = r#"{"sheet":"План на отм. 0.000","kind":"АР","owner":"",
 "facts":[{"name":"Площадь застройки","value":"2 940","unit":"м2"},{"name":"","value":"x"}],
 "lists":[{"title":"Экспликация помещений","columns":["№","Наименование","Площадь"],
           "rows":[["1","Цех","2 400,5"],["2","Склад","380"]]},{"title":"пустая","rows":[]}],
 "tables":[{"title":"","rows":[{"pos":"1","name":"Ø12 A-III L=1000","qty":"10","mass":"0.89","total":"8.9"}]}]}"#;
        let out = parse_page(body, 5).unwrap();
        assert_eq!(out.digest.kind, "АР");
        assert_eq!(out.digest.facts.len(), 1);
        assert_eq!(out.digest.facts[0].page, Some(5));
        assert_eq!(out.digest.lists.len(), 1);
        assert_eq!(out.digest.lists[0].rows[0][2], "2 400,5");
        assert_eq!(out.tables.len(), 1);
    }

    #[test]
    fn questions_keep_prefilled_answers() {
        let body = r#"{"summary":"Одноэтажный цех.","questions":[
          {"topic":"Фундамент","text":"Тип фундамента?","options":["Монолитный","Сваи"],"answer":"Монолитный (лист 10)"},
          {"topic":"Отделка","text":"Отделка стен?","options":["Штукатурка","Нет"],"answer":""},
          {"topic":"","text":"","options":[],"answer":""}]}"#;
        let (summary, qs) = parse_questions(body).unwrap();
        assert_eq!(summary, "Одноэтажный цех.");
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].answer, "Монолитный (лист 10)");
        assert!(qs[1].answer.is_empty());
    }

    /// Formula ustun: AI yozgan son emas, dastur hisoblagani olinadi;
    /// hisoblanmaydigan formula — taxmin.
    #[test]
    fn stage_quantities_come_from_formulas_not_from_the_model() {
        let body = r#"{"works":[
          {"name":"Опалубка","qty":999,"unit":"м2","source":"calc","formula":"2*(60+24)*1.2","note":"периметр × высота",
           "materials":[{"name":"Доска","qty":3,"unit":"м3","source":"standard","note":"0.03 м3/м2"}]},
          {"name":"Бетон","qty":120,"unit":"м3","source":"project","page":11,"materials":[]},
          {"name":"Разметка","qty":1,"unit":"компл","source":"calc","formula":"S участка / 2","materials":[]},
          {"name":"","qty":1,"unit":"","source":"","materials":[]}]}"#;
        let st = parse_stage(body, "Фундамент").unwrap();
        assert_eq!(st.works.len(), 3);
        assert!((st.works[0].qty - 201.6).abs() < 1e-9);
        assert!(matches!(st.works[0].source, Source::Calc { .. }));
        assert_eq!(st.works[1].source, Source::Project { page: Some(11) });
        assert!(matches!(st.works[2].source, Source::Assumption { .. }));
        assert_eq!(st.works[2].qty, 1.0);
        assert_eq!(
            parse_stages(r#"{"stages":[{"name":"A","scope":"b"},{"name":""}]}"#)
                .unwrap()
                .len(),
            1
        );
    }
}
