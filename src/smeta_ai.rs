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
use crate::smeta::{
    eval, Fact, Finding, List, PageDigest, Question, Resource, Source, Stage, Work,
};
use crate::takeoff::SpecTable;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

const WORKERS: usize = 4;

// ================================================================ 1. Varaq

const PAGE_SYSTEM: &str = "Ты — инженер ПТО. Тебе дан текст одного или нескольких листов проектной документации, \
извлечённый программой (каждый лист начинается строкой «=== Лист N ===»; строки сверху вниз, ячейки разделены « | »). \
Иногда приложен PDF листа. Опиши каждый лист отдельно, в JSON.\n\
Правила: числа переписывай точно как на листе, ничего не вычисляй и не придумывай; чего нет на листе — того нет в ответе. \
Листы не смешивай: данные листа N только в объекте с page=N.\n\
Поля листа:\n\
page — номер листа из строки «=== Лист N ===».\n\
sheet — название листа из штампа или заголовка.\n\
kind — раздел: ГП, АР, КЖ, КМ, КМД, ВК, ОВ, ЭОМ, ПЗ, ТХ или другое.\n\
facts — технико-экономические показатели и размеры, прямо написанные на листе: площадь застройки, общая/полезная площадь, \
строительный объём, этажность, высота этажа, размеры в осях, шаг колонн, отметки, количество квартир/помещений, толщина \
стен, марка бетона, класс арматуры, тип кровли, тип фундамента, нагрузки, сейсмичность. name ОБЯЗАТЕЛЬНО самодостаточный: \
с объектом/зданием/элементом и маркой — «Площадь застройки АБК», «Объём бетона B20 фундамента Фм1», «Высота этажа корпуса №1», \
«Толщина наружных стен корпуса №2». Голые «Площадь», «Объём», «Высота», «Отметка» — ошибка. Масштаб, формат, стадию, номера \
листов, отметки осей и размерные цепочки не включай. Не больше 20 фактов на лист, самые важные для сметы. \
Формат: {\"name\":\"\",\"value\":\"\",\"unit\":\"\"}.\n\
lists — таблицы, кроме спецификаций конструкций: ведомости объёмов работ, экспликации помещений с площадями, \
ведомости отделки, ведомости полов, проёмов, перемычек, расхода стали. Формат: {\"title\":\"\",\"columns\":[],\"rows\":[[]]}, \
каждая строка — массив ячеек по столбцам. Экспликация помещений — это list, а не десятки facts «Площадь».\n\
tables — спецификации конструкций и изделий (столбцы Поз./Марка, Обозначение, Наименование, Кол., Масса, Примечание) \
и «Выборка металла» с листов КМ/КМД (designation=\"КМД\", name=\"<профиль> <марка стали>\", mass=<масса общая>, unit=\"кг\"; \
строки Итого не включай). Строки-заголовки внутри таблицы передавай отдельной строкой с заполненным только name. \
Формат строки: {\"pos\":\"\",\"designation\":\"\",\"name\":\"\",\"qty\":\"\",\"mass\":\"\",\"total\":\"\",\"unit\":\"\"}; \
total — масса общая либо число из столбца «Примечание». Название таблицы — в title.\n\
owner — марка конструкции, которой посвящён весь лист, если она прямо написана в штампе (например «К3»), иначе \"\".\n\
Штамп, примечания, ведомости чертежей и ссылочных документов не включай. Пустые поля не описывай словами — пустой массив.\n\
Ответ — только JSON: {\"pages\":[{\"page\":1,\"sheet\":\"\",\"kind\":\"\",\"owner\":\"\",\"facts\":[],\"lists\":[],\"tables\":[]}]}";

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

/// Guruh javobini varaqlarga ajratadi: har varaq uchun o'z JSON matni.
/// Eski bir varaqli javob (`pages` yo'q) ham qabul qilinadi.
pub fn split_pages(body: &str, wanted: &[usize]) -> Vec<(usize, Result<String, String>)> {
    let v: serde_json::Value = match serde_json::from_str(strip(body)) {
        Ok(v) => v,
        Err(e) => {
            return wanted
                .iter()
                .map(|p| (*p, Err(format!("JSON: {e}"))))
                .collect()
        }
    };
    let Some(pages) = v.get("pages").and_then(|p| p.as_array()) else {
        // Bir varaqli javob.
        return wanted.iter().map(|p| (*p, Ok(v.to_string()))).collect();
    };
    wanted
        .iter()
        .map(|p| {
            let found = pages
                .iter()
                .find(|x| x.get("page").and_then(|n| n.as_u64()) == Some(*p as u64))
                .or_else(|| (wanted.len() == 1 && pages.len() == 1).then(|| &pages[0]));
            match found {
                Some(x) => (*p, Ok(x.to_string())),
                None => (*p, Err("javobda bu varaq yo'q".to_string())),
            }
        })
        .collect()
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
    let kind = cell(v.get("kind"));
    // Varaq egasi (masalan «К3») faqat konstruksiya bo'limlarida ma'noli:
    // ВК varaqidagi «К2 — внутренний водосток» tizim kodi, ustun К2 emas.
    // Aks holda butun ВК spetsifikatsiyasi ustunlar soniga ko'payib
    // ketadi (AL QUDRA: 44 voronka → 704).
    let owner = match crate::smeta::section_of(&kind) {
        Some("КЖ") | Some("КМ") | Some("АР") | None => owner,
        _ => String::new(),
    };
    Ok(PageOut {
        digest: PageDigest {
            page,
            sheet: cell(v.get("sheet")),
            kind,
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

/// Bir so'rovga sig'adigan matn (belgi). Tizim ko'rsatmasi har so'rovda
/// takrorlanadi — varaqlar guruhlansa u amortizatsiya bo'ladi.
const BATCH_CHARS: usize = 16_000;
/// Bir so'rovda ko'pi bilan shuncha varaq: model varaqlarni aralashtirmasin.
const BATCH_PAGES: usize = 4;
/// Shu belgidan kam matnli varaq «kam matnli»: unga PDF ham qo'shiladi.
const SPARSE_CHARS: usize = 800;

/// Varaqlarni so'rov guruhlariga bo'ladi: ketma-ket, matn byudjeti va
/// varaq soni bo'yicha. Kam matnli varaq alohida ketadi (PDF bilan).
pub fn batches(sizes: &[(usize, usize)]) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = Vec::new();
    let mut cur: Vec<usize> = Vec::new();
    let mut cur_chars = 0usize;
    for &(page, chars) in sizes {
        if chars < SPARSE_CHARS {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
                cur_chars = 0;
            }
            out.push(vec![page]);
            continue;
        }
        if !cur.is_empty() && (cur.len() >= BATCH_PAGES || cur_chars + chars > BATCH_CHARS) {
            out.push(std::mem::take(&mut cur));
            cur_chars = 0;
        }
        cur.push(page);
        cur_chars += chars;
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Barcha o'qiladigan varaqlarni fon iplarida AI ga beradi.
///
/// Token tejash: varaqlar faqat matn bilan, guruhlab yuboriladi (tizim
/// ko'rsatmasi 77 marta emas, ~25 marta ketadi); PDF ko'rinishi faqat
/// matni kam varaqlarga (chizma, kam yozuv) qo'shiladi.
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
        let texts: Vec<(usize, String)> = pages
            .iter()
            .enumerate()
            .filter(|(_, p)| readable(p))
            .map(|(i, p)| (i + 1, crate::aitake::page_text(p)))
            .collect();
        let plan: Vec<usize> = texts.iter().map(|t| t.0).collect();
        let _ = tx.send(PageMsg::Plan(plan));
        let sizes: Vec<(usize, usize)> =
            texts.iter().map(|(p, t)| (*p, t.chars().count())).collect();
        let text_of = |page: usize| -> &str {
            texts
                .iter()
                .find(|t| t.0 == page)
                .map(|t| t.1.as_str())
                .unwrap_or("")
        };
        let queue = Mutex::new(VecDeque::from(batches(&sizes)));
        std::thread::scope(|s| {
            for _ in 0..WORKERS {
                s.spawn(|| loop {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let Some(batch) = queue.lock().ok().and_then(|mut q| q.pop_front()) else {
                        break;
                    };
                    let mut text = String::new();
                    for page in &batch {
                        text.push_str(&format!("=== Лист {page} ===\n{}\n\n", text_of(*page)));
                    }
                    let mut parts = vec![Part::Text(text)];
                    if batch.len() == 1 && text_of(batch[0]).chars().count() < SPARSE_CHARS {
                        if let Some(data) = crate::aitake::page_pdf(&doc, batch[0] as u32) {
                            parts.push(Part::Pdf {
                                name: format!("list-{}.pdf", batch[0]),
                                data,
                            });
                        }
                    }
                    let mut result = llm::extract(&cfg, PAGE_SYSTEM, &parts);
                    if let Err(e) = &result {
                        if e.retryable() {
                            std::thread::sleep(std::time::Duration::from_secs(3));
                            result = llm::extract(&cfg, PAGE_SYSTEM, &parts);
                        }
                    }
                    let mut stop = false;
                    match result {
                        Ok(a) => {
                            // Tokenlar guruh varaqlariga teng bo'linadi.
                            let share = a.usage.total / batch.len().max(1) as u32;
                            for (page, body) in split_pages(&a.text, &batch) {
                                let out = body.and_then(|b| parse_page(&b, page)).map(|mut p| {
                                    p.tokens = share;
                                    p
                                });
                                if tx.send(PageMsg::Page(page, out)).is_err() {
                                    stop = true;
                                }
                            }
                        }
                        Err(e) => {
                            for page in &batch {
                                if tx.send(PageMsg::Page(*page, Err(e.to_string()))).is_err() {
                                    stop = true;
                                }
                            }
                        }
                    }
                    if stop {
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

const QUESTIONS_SYSTEM: &str = "Ты — главный инженер проекта и ведущий сметчик с 20-летним опытом промышленного и \
гражданского строительства. Перед тобой выжимка рабочего проекта: показатели с листов, ведомости, конструкции и материалы, \
снятые программой. Ты готовишься к предконтрактной встрече с заказчиком и должен задать ему только те вопросы, которые \
задаёт профессионал — не ученик.\n\
1. summary — 3–5 предложений инженерным языком: что за объект, конструктив, этажность, площади, фундамент, каркас, \
стены, перекрытия, кровля, инженерные разделы. Только из данных.\n\
2. questions — вопросы к заказчику. Жёсткие правила:\n\
- НИКОГДА не спрашивай то, что есть на листах (марки бетона, классы арматуры, типы фундаментов, кровли, стен, площади \
и т.п.). Сначала проверь данные. Вопрос про уже известное — грубая ошибка.\n\
- Спрашивай о том, что реально определяет стоимость и не содержится в рабочих чертежах: инженерно-геологические условия \
и уровень грунтовых вод (если нет в данных); границы поставки — что покупает заказчик сам (технологическое оборудование, \
металлоконструкции, сэндвич-панели), давальческие материалы; точки подключения и протяжённость наружных сетей, \
мощность электроснабжения, источник тепла; класс пожарной опасности и требования к огнезащите; категории помещений и \
уровень отделки по группам помещений; требования к полам (нагрузка, топпинг, допуски); производители/бренды окон, \
ворот, кровельных систем; логистика площадки (подъезд, временные сети, вахта); сроки и этапность (пусковые комплексы), \
работа в сезон; базис цен (с НДС/без), валюта, авансирование, гарантии; что исключить из договора.\n\
- Если на листах противоречие (например площадь участка 4 га и 4,85 га) — один вопрос с обоими значениями и номерами \
листов, варианты — эти значения.\n\
- Формулируй конкретно, со ссылкой на данные проекта (здание, лист, объём) и с указанием, на что влияет ответ. \
Варианты ответа — инженерно осмысленные альтернативы (2–5), последний — «уточнить у заказчика».\n\
- why — одна фраза: как ответ меняет стоимость или состав работ (с порядком величины, если можно).\n\
- impact — high / medium / low по влиянию на стоимость.\n\
- Не больше 10 вопросов, самые дорогие по влиянию первыми. Если спрашивать нечего — пустой список. answer всегда \"\".\n\
Ответ — только JSON: {\"summary\":\"\",\"questions\":[{\"topic\":\"\",\"text\":\"\",\"options\":[],\"why\":\"\",\
\"impact\":\"high\",\"answer\":\"\"}]}";

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
            why: cell(q.get("why")),
            impact: cell(q.get("impact")).to_lowercase(),
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

// ============================================= 2b. Savollarni tekshirish

const VERIFY_SYSTEM: &str = "Ты — проверяющий. Даны данные проекта (показатели с листов, ведомости, конструкции, \
материалы) и список вопросов, которые помощник собирается задать заказчику. Найди вопросы, ответ на которые УЖЕ \
содержится в данных проекта (прямо или однозначно выводится), и дай этот ответ со ссылкой на лист. Такие вопросы \
заказчику задавать нельзя. Если данных нет или они противоречивы — вопрос оставь (не включай в found).\n\
Ответ — только JSON: {\"found\":[{\"i\":0,\"answer\":\"краткий ответ из данных\",\"page\":12}]} — i совпадает с \
номером вопроса во входе; page — номер листа или 0.";

/// Tekshiruv javobi: `(o'rin, javob, varaq)`.
pub fn parse_verify(body: &str) -> Result<Vec<(usize, String, usize)>, String> {
    let v: serde_json::Value =
        serde_json::from_str(strip(body)).map_err(|e| format!("JSON: {e}"))?;
    Ok(v.get("found")
        .and_then(|f| f.as_array())
        .ok_or_else(|| "JSON: found yo'q".to_string())?
        .iter()
        .filter_map(|f| {
            let i = f.get("i")?.as_u64()? as usize;
            let answer = cell(f.get("answer"));
            let page = f.get("page").and_then(|p| p.as_u64()).unwrap_or(0) as usize;
            (!answer.is_empty()).then_some((i, answer, page))
        })
        .collect())
}

pub type VerifyRx = Receiver<Result<(Vec<(usize, String, usize)>, u32), String>>;

/// Savollarni loyiha ma'lumotiga qarshi tekshiradi: javobi loyihada bor
/// savol mijozga berilmaydi — javobi varaq raqami bilan yoziladi.
pub fn spawn_verify(cfg: Config, context: String) -> VerifyRx {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let out = llm::extract(&cfg, VERIFY_SYSTEM, &[Part::Text(context)])
            .map_err(|e| e.to_string())
            .and_then(|a| parse_verify(&a.text).map(|r| (r, a.usage.total)));
        let _ = tx.send(out);
    });
    rx
}

// ===================================================== 2a. AI javoblari

const ANSWERS_SYSTEM: &str = "Ты — опытный подрядчик. Даны описание объекта и список вопросов без ответа, у каждого \
варианты. Для каждого вопроса выбери НАИБОЛЕЕ типичный для такого объекта вариант (из предложенных, кроме \
«уточнить у заказчика») и в одной фразе объясни почему. Это допущения — человек их проверит; не выбирай крайние \
дорогие варианты без оснований.\n\
Ответ — только JSON: {\"answers\":[{\"i\":0,\"answer\":\"текст варианта\",\"why\":\"\"}]} — i совпадает с номером вопроса.";

/// AI javoblari: `(o'rin, javob, sabab)`.
pub fn parse_answers(body: &str) -> Result<Vec<(usize, String, String)>, String> {
    let v: serde_json::Value =
        serde_json::from_str(strip(body)).map_err(|e| format!("JSON: {e}"))?;
    Ok(v.get("answers")
        .and_then(|a| a.as_array())
        .ok_or_else(|| "JSON: answers yo'q".to_string())?
        .iter()
        .filter_map(|a| {
            let i = a.get("i")?.as_u64()? as usize;
            let answer = cell(a.get("answer"));
            (!answer.is_empty()).then(|| (i, answer, cell(a.get("why"))))
        })
        .collect())
}

pub type AnswersRx = Receiver<Result<(Vec<(usize, String, String)>, u32), String>>;

/// Javobsiz savollarga AI eng tipik variantni tanlaydi.
pub fn spawn_answers(cfg: Config, context: String) -> AnswersRx {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let out = llm::extract(&cfg, ANSWERS_SYSTEM, &[Part::Text(context)])
            .map_err(|e| e.to_string())
            .and_then(|a| parse_answers(&a.text).map(|r| (r, a.usage.total)));
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

const STAGE_SYSTEM: &str = "Ты — опытный сметчик. Тебе даны данные объекта (показатели с листов, ответы заказчика), \
список всех этапов, ОДИН этап для развёртки и МАТЕРИАЛЫ ПРОЕКТА, ЗАКРЕПЛЁННЫЕ ЗА ЭТИМ ЭТАПОМ. Разверни этап в работы \
и материалы.\n\
Главные правила:\n\
А. Из материалов проекта используй ТОЛЬКО закреплённые за этапом — каждый ровно один раз, с его количеством и листом. \
Материалы других этапов не добавляй, даже если они кажутся уместными: их учтёт свой этап.\n\
Б. Работы — только по существу этапа. Окна и двери — только в этапе проёмов, отделка — в этапе отделки, сети — в своих \
этапах; не повторяй чужие работы.\n\
В. Не создавай отдельных работ «доставка», «применение автокрана», «разбивка осей», «механизация»: техника и доставка — \
строки materials внутри работы (unit смена / рейс), и только если без них работу не выполнить.\n\
Г. qty «1 компл.» допустим только для действительно комплектных работ (пусконаладка, сдача). Измеримая работа всегда \
с числом и единицей; если данных нет — source=assumption с инженерной оценкой и note, что уточнить.\n\
Д. Если за этапом нет материалов проекта и нет ответа заказчика — не выдумывай состав: максимум 3 работы, все \
source=assumption, с note «раздел в проекте отсутствует».\n\
Остальные правила:\n\
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

const ALLOCATE_SYSTEM: &str = "Ты — опытный сметчик. Даны этапы строительства (с номерами) и материалы/конструкции, \
снятые программой с проекта (с номерами). Закрепи КАЖДЫЙ материал ровно за ОДНИМ этапом, где он монтируется или \
расходуется (бетон подготовки — фундаменты; арматура фундаментов — фундаменты; арматура полов — полы; прогоны, фермы, \
связи — каркас; сэндвич-панели стен — ограждающие конструкции, кровельные — кровля; трубы и воронки водостока — \
инженерные сети; лотки, бортовые камни, асфальт — благоустройство). Учитывай лист материала: материалы одного листа \
обычно относятся к одному этапу. Если материал ни к одному этапу не подходит — этап 0.\n\
Ответ — только JSON: {\"assign\":[{\"r\":1,\"s\":3}]} — r номер материала, s номер этапа (1-based) или 0.";

/// Taqsimlash javobi: material raqami → bosqich raqami (1 dan; 0 — hech qaysi).
pub fn parse_allocation(body: &str) -> Result<Vec<(usize, usize)>, String> {
    let v: serde_json::Value =
        serde_json::from_str(strip(body)).map_err(|e| format!("JSON: {e}"))?;
    Ok(v.get("assign")
        .and_then(|a| a.as_array())
        .ok_or_else(|| "JSON: assign yo'q".to_string())?
        .iter()
        .filter_map(|x| {
            let r = x.get("r")?.as_u64()? as usize;
            let s = x.get("s").and_then(|s| s.as_u64()).unwrap_or(0) as usize;
            Some((r, s))
        })
        .collect())
}

/// Materiallarni bosqichlarga bo'ladi: har bosqich uchun unga tegishli
/// qatorlar. Taqsimlanmagan (0) yoki javobda yo'q material — oxirgi
/// «qolganlar» ro'yxatiga; u barcha bosqichlarga ko'rsatilmaydi, lekin
/// yo'qolmasin deb birinchi mos bosqichga emas, hisobotga qoladi.
pub fn split_resources(
    resources: &[String],
    assign: &[(usize, usize)],
    stages: usize,
) -> Vec<Vec<String>> {
    let mut out = vec![Vec::new(); stages];
    for (i, line) in resources.iter().enumerate() {
        let r = i + 1;
        if let Some(&(_, s)) = assign.iter().find(|a| a.0 == r) {
            if s >= 1 && s <= stages {
                out[s - 1].push(line.clone());
            }
        }
    }
    out
}

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
/// `context` — bosqichlar ro'yxati uchun to'liq ma'lumot (bir marta);
/// `stage_context` — har bosqich so'roviga qisqa ma'lumot (15 marta
/// takrorlanadi, shuning uchun ixcham).
/// `resources` — loyihadan olingan materiallar va konstruksiyalar
/// (bir qator — bir material, raqamlanadi); ular avval bosqichlarga
/// taqsimlanadi, har bosqich faqat o'zinikini ko'radi — takror yo'q,
/// kontekst kichik.
pub fn spawn_spec(
    cfg: Config,
    context: String,
    stage_context: String,
    resources: Vec<String>,
    cancel: Arc<AtomicBool>,
) -> Receiver<StageMsg> {
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
        let all: Vec<String> = stages
            .iter()
            .enumerate()
            .map(|(i, (n, s))| format!("{}. {n}: {s}", i + 1))
            .collect();
        // Materiallarni bosqichlarga taqsimlash — bitta so'rov.
        let per_stage: Vec<Vec<String>> = if resources.is_empty() {
            vec![Vec::new(); stages.len()]
        } else {
            let numbered: Vec<String> = resources
                .iter()
                .enumerate()
                .map(|(i, r)| format!("{}. {r}", i + 1))
                .collect();
            let text = format!(
                "ЭТАПЫ:\n{}\n\nМАТЕРИАЛЫ И КОНСТРУКЦИИ ПРОЕКТА:\n{}",
                all.join("\n"),
                numbered.join("\n")
            );
            match llm::extract(&cfg, ALLOCATE_SYSTEM, &[Part::Text(text)])
                .map_err(|e| e.to_string())
                .and_then(|a| parse_allocation(&a.text))
            {
                Ok(assign) => split_resources(&resources, &assign, stages.len()),
                // Taqsimlash o'tmasa — eski yo'l: hamma bosqich hammasini ko'radi.
                Err(_) => vec![resources.clone(); stages.len()],
            }
        };
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
                    let mine = &per_stage[i];
                    let materials = if mine.is_empty() {
                        "— за этапом нет материалов проекта (см. правило Д)".to_string()
                    } else {
                        mine.iter().map(|m| format!("- {m}")).collect::<Vec<_>>().join("\n")
                    };
                    let text = format!(
                        "{stage_context}\n\nВСЕ ЭТАПЫ ОБЪЕКТА (чтобы не дублировать работы между ними):\n{}\n\n\
                         ЭТАП ДЛЯ РАЗВЁРТКИ: {}. {name} — {scope}\n\n\
                         МАТЕРИАЛЫ ПРОЕКТА, ЗАКРЕПЛЁННЫЕ ЗА ЭТИМ ЭТАПОМ (уже умножены на количество конструкций; \
                         использовать каждый ровно один раз, source=project, лист указан):\n{materials}",
                        all.join("\n"),
                        i + 1
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

// ================================================================ 4. Narx

const PRICES_SYSTEM: &str = "Ты — сметчик с опытом работы в Узбекистане. Дан список позиций сметы: \
вид (работа или материал), название, единица измерения. Для каждой дай ОРИЕНТИРОВОЧНУЮ рыночную цену в \
узбекских сумах (UZS) для Ташкента на текущий год: для работы — стоимость выполнения за единицу (только труд, \
без материала), для материала — закупочная цена за единицу с доставкой по городу. \
В note одним коротким предложением напиши, на чём основана цена (типичный диапазон, аналог). \
Если позицию оценить нельзя (неясно, что это, или единица не подходит) — поставь price 0 и объясни в note. \
Не завышай точность: это ориентир, который человек проверит. \
Ответ — только JSON: {\"prices\":[{\"i\":0,\"price\":0,\"note\":\"\"}]} — i совпадает с номером позиции во входе.";

/// Bir so'rovda baholanadigan pozitsiyalar.
const PRICE_BATCH: usize = 50;

/// Narx taklifi javobini o'qiydi: `(o'rin, narx, izoh)`.
pub fn parse_prices(body: &str) -> Result<Vec<(usize, f64, String)>, String> {
    let v: serde_json::Value =
        serde_json::from_str(strip(body)).map_err(|e| format!("JSON: {e}"))?;
    let list = v
        .get("prices")
        .and_then(|p| p.as_array())
        .ok_or_else(|| "JSON: prices yo'q".to_string())?
        .iter()
        .filter_map(|p| {
            let i = p.get("i")?.as_u64()? as usize;
            let price = number(p.get("price"));
            Some((i, price, cell(p.get("note"))))
        })
        .collect();
    Ok(list)
}

pub enum PriceMsg {
    /// `(kalit, narx, izoh)` — nol narx kelmaydi.
    Batch(Vec<(String, f64, String)>),
    Failed(String),
    Done,
}

/// Narxsiz pozitsiyalar uchun AI taklifini fonda so'raydi.
///
/// `items` — `(ishmi, nom, birlik)`. Javoblar partiyalab keladi; nol
/// narxli (baholab bo'lmagan) pozitsiya tashlab yuboriladi.
pub fn spawn_prices(
    cfg: Config,
    items: Vec<(bool, String, String)>,
    cancel: Arc<AtomicBool>,
) -> Receiver<PriceMsg> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let batches: Vec<Vec<(bool, String, String)>> =
            items.chunks(PRICE_BATCH).map(|c| c.to_vec()).collect();
        let queue = Mutex::new(VecDeque::from(batches));
        std::thread::scope(|s| {
            for _ in 0..WORKERS {
                s.spawn(|| loop {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let Some(batch) = queue.lock().ok().and_then(|mut q| q.pop_front()) else {
                        break;
                    };
                    let text = batch
                        .iter()
                        .enumerate()
                        .map(|(i, (work, name, unit))| {
                            format!(
                                "{i} | {} | {name} | {unit}",
                                if *work {
                                    "работа"
                                } else {
                                    "материал"
                                }
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    let text = format!("Позиции (i | вид | название | единица):\n{text}");
                    let mut result = llm::extract(&cfg, PRICES_SYSTEM, &[Part::Text(text.clone())]);
                    if let Err(e) = &result {
                        if e.retryable() {
                            std::thread::sleep(std::time::Duration::from_secs(3));
                            result = llm::extract(&cfg, PRICES_SYSTEM, &[Part::Text(text)]);
                        }
                    }
                    let msg = match result
                        .map_err(|e| e.to_string())
                        .and_then(|a| parse_prices(&a.text))
                    {
                        Ok(list) => PriceMsg::Batch(
                            list.into_iter()
                                .filter(|(i, price, _)| *price > 0.0 && *i < batch.len())
                                .map(|(i, price, note)| {
                                    let (_, name, unit) = &batch[i];
                                    (crate::smeta::key(name, unit), price, note)
                                })
                                .collect(),
                        ),
                        Err(e) => PriceMsg::Failed(e),
                    };
                    if tx.send(msg).is_err() {
                        break;
                    }
                });
            }
        });
        let _ = tx.send(PriceMsg::Done);
    });
    rx
}

// ============================================================ 5. Tekshiruv

const REVIEW_SYSTEM: &str = "Ты — главный сметчик, проверяющий смету, собранную помощником. Дана структура: \
этапы, работы, материалы с адресами (s<этап>.w<работа>.m<материал>), количества, источник каждого количества \
(из проекта / расчёт / стандарт / допущение) и цены. Найди проблемы и верни JSON:\n\
{\"summary\":\"2–3 предложения общего вывода\",\"findings\":[{\"kind\":\"qty|missing|dup|price|ask\",\
\"id\":\"s3.w2\",\"text\":\"что не так и почему\",\"qty\":0,\"unit\":\"\",\"name\":\"\"}]}\n\
Правила:\n\
qty — количество неправдоподобно для такого объекта (сравни с показателями); id — адрес строки; qty — твоё \
исправленное значение, если его можно вывести из показателей объекта (иначе 0), unit — единица.\n\
missing — работа, которая обычно есть в таком объекте, но её нет; id — адрес этапа (s3), name — название работы, \
unit — единица, qty — количество, если выводится из показателей (иначе 0).\n\
dup — одно и то же посчитано в двух местах; id — адрес строки, которую надо УБРАТЬ (оставь ту, где источник надёжнее).\n\
price — цена заметно выше или ниже рынка Узбекистана; id — адрес строки.\n\
ask — что уточнить у заказчика в первую очередь; id пустой.\n\
Не больше 25 находок, самые важные первыми. Опирайся только на данные.";

/// Tekshiruv javobini o'qiydi: `(xulosa, topilmalar)`.
pub fn parse_review(body: &str) -> Result<(String, Vec<Finding>), String> {
    let v: serde_json::Value =
        serde_json::from_str(strip(body)).map_err(|e| format!("JSON: {e}"))?;
    let list = v
        .get("findings")
        .and_then(|f| f.as_array())
        .ok_or_else(|| "JSON: findings yo'q".to_string())?
        .iter()
        .map(|f| {
            let qty = number(f.get("qty"));
            Finding {
                kind: cell(f.get("kind")),
                text: cell(f.get("text")),
                id: cell(f.get("id")),
                qty: (qty > 0.0).then_some(qty),
                unit: cell(f.get("unit")),
                name: cell(f.get("name")),
                done: false,
            }
        })
        .filter(|f| {
            !f.text.is_empty()
                && ["qty", "missing", "dup", "price", "ask"].contains(&f.kind.as_str())
        })
        .collect();
    Ok((cell(v.get("summary")), list))
}

/// Tekshiruv natijasi: xulosa, topilmalar, tokenlar.
pub type ReviewRx = Receiver<Result<(String, Vec<Finding>, u32), String>>;

/// Smetani AI ga tekshirtiradi.
pub fn spawn_review(cfg: Config, context: String) -> ReviewRx {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let out = llm::extract(&cfg, REVIEW_SYSTEM, &[Part::Text(context)])
            .map_err(|e| e.to_string())
            .and_then(|a| parse_review(&a.text).map(|(s, f)| (s, f, a.usage.total)));
        let _ = tx.send(out);
    });
    rx
}

// ============================================================== 7. Hisobot

const REPORT_SYSTEM: &str = "Ты — главный инженер проекта и сметчик. По данным объекта (показатели с листов, \
конструкции, материалы из проекта, ответы заказчика, спецификация по этапам, смета и заключение проверки) напиши \
разделы отчёта для заказчика и руководителя. Пиши деловым языком, конкретно, с числами из данных; язык — из строки ЯЗЫК ОТВЕТА в данных, если её нет — русский. \
Ничего не выдумывай: если данных нет — так и пиши. Разделы (ровно эти, в этом порядке):\n\
1. Краткое резюме — 5–7 предложений: что за объект, главные объёмы, ориентировочная стоимость, степень надёжности расчёта.\n\
2. Объект и конструктив — здания, площади, этажность, фундаменты, каркас, стены, перекрытия, кровля, инженерные разделы.\n\
3. Основные объёмы — бетон по классам, арматура, металлопрокат, кладка, кровля, покрытия; откуда взяты (лист/расчёт).\n\
4. Допущения и неопределённости — что принято без данных, что уточнить у заказчика, как это влияет на стоимость.\n\
5. Риски — где расчёт может ошибаться (дубли, завышения, пропуски), на что обратить внимание при проверке.\n\
6. Рекомендации — следующие шаги для получения точной сметы и договора.\n\
7. Границы расчёта — что НЕ учтено (оборудование, НДС, накладные, непредвиденные, сроки), какие цены использованы (каталог/ориентир AI).\n\
Ответ — только JSON: {\"sections\":[{\"title\":\"\",\"body\":\"\"}]}. Общий объём до 1200 слов.";

/// Hisobot javobi: bo'limlar.
pub fn parse_report(body: &str) -> Result<Vec<(String, String)>, String> {
    let v: serde_json::Value =
        serde_json::from_str(strip(body)).map_err(|e| format!("JSON: {e}"))?;
    Ok(v.get("sections")
        .and_then(|a| a.as_array())
        .ok_or_else(|| "JSON: sections yo'q".to_string())?
        .iter()
        .map(|x| (cell(x.get("title")), cell(x.get("body"))))
        .filter(|x| !x.0.is_empty() && !x.1.is_empty())
        .collect())
}

pub type ReportRx = Receiver<Result<(Vec<(String, String)>, u32), String>>;

pub fn spawn_report(cfg: Config, context: String) -> ReportRx {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let out = llm::extract(&cfg, REPORT_SYSTEM, &[Part::Text(context)])
            .map_err(|e| e.to_string())
            .and_then(|a| parse_report(&a.text).map(|r| (r, a.usage.total)));
        let _ = tx.send(out);
    });
    rx
}

// ========================================================= 6. Konsolidatsiya

const CONSOLIDATE_SYSTEM: &str = "Ты — сметчик. Этапы сметы собирались параллельно и не видели друг друга, поэтому \
одна и та же работа или материал могли попасть в несколько этапов. Дан полный список с адресами (s<этап>.w<работа>, \
s<этап>.w<работа>.m<материал>). Найди строки, которые считают ОДИН И ТОТ ЖЕ физический объём повторно (одинаковое или \
почти одинаковое название и количество в разных этапах; материал из проекта, отнесённый к двум работам). Оставь строку \
в наиболее подходящем этапе, остальные верни на удаление. Не удаляй строки, которые лишь похожи по названию, но \
относятся к разным объёмам (разные количества, разные здания).\n\
Ответ — только JSON: {\"remove\":[{\"id\":\"s3.w2\",\"reason\":\"коротко\"}]}";

/// Konsolidatsiya javobi: olib tashlanadigan manzillar va sabab.
pub fn parse_consolidate(body: &str) -> Result<Vec<(String, String)>, String> {
    let v: serde_json::Value =
        serde_json::from_str(strip(body)).map_err(|e| format!("JSON: {e}"))?;
    Ok(v.get("remove")
        .and_then(|r| r.as_array())
        .ok_or_else(|| "JSON: remove yo'q".to_string())?
        .iter()
        .map(|r| (cell(r.get("id")), cell(r.get("reason"))))
        .filter(|r| crate::smeta::locate(&r.0).is_some_and(|l| l.1.is_some()))
        .collect())
}

pub type ConsolidateRx = Receiver<Result<(Vec<(String, String)>, u32), String>>;

pub fn spawn_consolidate(cfg: Config, context: String) -> ConsolidateRx {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let out = llm::extract(&cfg, CONSOLIDATE_SYSTEM, &[Part::Text(context)])
            .map_err(|e| e.to_string())
            .and_then(|a| parse_consolidate(&a.text).map(|r| (r, a.usage.total)));
        let _ = tx.send(out);
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_and_consolidation_replies_are_structured() {
        let body = r#"{"summary":"В целом полно.","findings":[
          {"kind":"qty","id":"s7.w0","text":"Кровля 132300 м2 завышена","qty":25000,"unit":"м2"},
          {"kind":"missing","id":"s10","text":"Нет лестниц","name":"Лестничные марши","unit":"шт","qty":4},
          {"kind":"dup","id":"s11.w3","text":"Водостоки повторно"},
          {"kind":"weird","id":"","text":"x"},
          {"kind":"ask","id":"","text":"Уточнить площадь кровли"}]}"#;
        let (summary, f) = parse_review(body).unwrap();
        assert_eq!(summary, "В целом полно.");
        assert_eq!(f.len(), 4);
        assert_eq!(f[0].qty, Some(25000.0));
        assert_eq!(f[1].name, "Лестничные марши");
        let rem = parse_consolidate(
            r#"{"remove":[{"id":"s2.w1","reason":"дубль"},{"id":"s3","reason":"x"}]}"#,
        )
        .unwrap();
        assert_eq!(rem, vec![("s2.w1".to_string(), "дубль".to_string())]);
    }

    #[test]
    fn batches_group_pages_by_budget_and_isolate_sparse_ones() {
        // 1–3 sig'adi, 4 kam matnli — alohida, 5–8 to'rtta, 9 qoladi.
        let sizes: Vec<(usize, usize)> = vec![
            (1, 5000),
            (2, 5000),
            (3, 5000),
            (4, 200),
            (5, 3000),
            (6, 3000),
            (7, 3000),
            (8, 3000),
            (9, 3000),
        ];
        assert_eq!(
            batches(&sizes),
            vec![vec![1, 2, 3], vec![4], vec![5, 6, 7, 8], vec![9]]
        );
        assert_eq!(batches(&[(1, 20_000)]), vec![vec![1]]);
        assert!(batches(&[]).is_empty());
    }

    #[test]
    fn allocation_splits_resources_per_stage() {
        let assign = parse_allocation(
            r#"{"assign":[{"r":1,"s":2},{"r":2,"s":0},{"r":3,"s":2},{"r":4,"s":9}]}"#,
        )
        .unwrap();
        let res: Vec<String> = ["a", "b", "c", "d"].iter().map(|s| s.to_string()).collect();
        let per = split_resources(&res, &assign, 3);
        assert!(per[0].is_empty());
        assert_eq!(per[1], vec!["a".to_string(), "c".to_string()]);
        assert!(per[2].is_empty(), "9-bosqich yo'q — tashlanadi");
        assert!(parse_allocation("{}").is_err());
    }

    #[test]
    fn owner_is_kept_only_for_construction_sections() {
        let vk = r#"{"sheet":"ВК-1","kind":"ВК","owner":"К2","facts":[],"lists":[],"tables":[]}"#;
        assert_eq!(parse_page(vk, 38).unwrap().owner, "");
        let kj = r#"{"sheet":"КЖ-3","kind":"КЖ","owner":"К2","facts":[],"lists":[],"tables":[]}"#;
        assert_eq!(parse_page(kj, 16).unwrap().owner, "К2");
    }

    #[test]
    fn grouped_reply_is_split_per_page() {
        let body = r#"{"pages":[{"page":3,"sheet":"A","facts":[],"lists":[],"tables":[]},{"page":4,"sheet":"B","facts":[],"lists":[],"tables":[]}]}"#;
        let parts = split_pages(body, &[3, 4, 5]);
        assert!(parts[0].1.as_ref().unwrap().contains("\"A\""));
        assert!(parts[1].1.as_ref().unwrap().contains("\"B\""));
        assert!(parts[2].1.is_err());
        // Eski bir varaqli javob ham o'qiladi.
        let single = r#"{"sheet":"C","facts":[],"lists":[],"tables":[]}"#;
        let out = parse_page(&split_pages(single, &[7])[0].1.clone().unwrap(), 7).unwrap();
        assert_eq!(out.digest.sheet, "C");
    }

    /// Haqiqiy PDF da so'rovlar soni (tarmoqsiz). `QURAI_LIVE_PDF`.
    /// Natija `%TEMP%\qurai_batches.txt`.
    #[test]
    #[ignore]
    fn batches_on_a_real_pdf() {
        let Ok(path) = std::env::var("QURAI_LIVE_PDF") else {
            return;
        };
        let pages = crate::pdfread::pages(&PathBuf::from(&path)).expect("pdf");
        let sizes: Vec<(usize, usize)> = pages
            .iter()
            .enumerate()
            .filter(|(_, p)| readable(p))
            .map(|(i, p)| (i + 1, crate::aitake::page_text(p).chars().count()))
            .collect();
        let b = batches(&sizes);
        let chars: usize = sizes.iter().map(|s| s.1).sum();
        let sparse = sizes.iter().filter(|s| s.1 < SPARSE_CHARS).count();
        let log = format!(
            "varaqlar {} / o'qiladigan {} / so'rovlar {} / PDF bilan {} / matn {} belgi
",
            pages.len(),
            sizes.len(),
            b.len(),
            sparse,
            chars
        );
        std::fs::write(std::env::temp_dir().join("qurai_batches.txt"), &log).unwrap();
        assert!(b.len() < sizes.len());
    }

    #[test]
    fn verify_replies_give_answer_and_page() {
        let list = parse_verify(
            r#"{"found":[{"i":1,"answer":"Монолитная лента","page":10},{"i":2,"answer":""}]}"#,
        )
        .unwrap();
        assert_eq!(list, vec![(1, "Монолитная лента".to_string(), 10)]);
        assert!(parse_verify("{}").is_err());
    }

    #[test]
    fn answer_replies_keep_index_and_reason() {
        let list = parse_answers(r#"{"answers":[{"i":2,"answer":"Кирпичные перегородки","why":"типично"},{"i":3,"answer":""}]}"#).unwrap();
        assert_eq!(
            list,
            vec![(
                2,
                "Кирпичные перегородки".to_string(),
                "типично".to_string()
            )]
        );
    }

    #[test]
    fn report_sections_are_read() {
        let r = parse_report(
            r#"{"sections":[{"title":"Резюме","body":"Цех 9310 м2."},{"title":"","body":"x"}]}"#,
        )
        .unwrap();
        assert_eq!(r, vec![("Резюме".to_string(), "Цех 9310 м2.".to_string())]);
    }

    #[test]
    fn price_replies_are_keyed_and_zero_is_dropped() {
        let body = r#"{"prices":[{"i":0,"price":650000,"note":"B20 Ташкент"},{"i":1,"price":0,"note":"неясно"},{"i":"x"}]}"#;
        let list = parse_prices(body).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], (0, 650000.0, "B20 Ташкент".to_string()));
        assert!(parse_prices("{}").is_err());
    }

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
