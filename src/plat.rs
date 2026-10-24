//! Platforma qatlami: desktop (Windows) va brauzer (WebAssembly) uchun
//! bitta interfeys.
//!
//! Ilova kodi fon ishi, kutish, vaqt va faylni shu yerdan oladi — qaysi
//! platformada ekanini bilmaydi. Farqlar faqat shu faylda:
//!
//! * **Fon ishi.** Desktopda — alohida ip (`std::thread`) ichida
//!   `block_on`. Brauzerda ip yo'q: ish shu ipda asinxron ketadi
//!   (`spawn_local`), tarmoq so'rovi `fetch` orqali, interfeys qotmaydi.
//! * **Bloklovchi ish** (`run_blocking`): desktopda alohida ipda — bir
//!   nechta AI so'rovi parallel ketadi. Brauzerda shu zahoti bajariladi.
//! * **Vaqt.** `std::time::Instant` brauzerda ishlamaydi (vahima beradi);
//!   `web_time::Instant` — xuddi shu interfeys.
//! * **Fayl.** Desktopda dialog va disk; brauzerda yuklab olish va
//!   `<input type=file>`.

use std::future::Future;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;
#[cfg(target_arch = "wasm32")]
pub use web_time::Instant;

/// Brauzerda ishlayaptimi.
pub const WEB: bool = cfg!(target_arch = "wasm32");

// ================================================================ Fon ishi

/// Fon ishini boshlaydi. `make` kelajakni (future) yaratadi — desktopda u
/// yangi ip ichida yaratiladi, shuning uchun kelajakning o'zi `Send`
/// bo'lishi shart emas.
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn<F, Fut>(make: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + 'static,
{
    std::thread::spawn(move || futures::executor::block_on(make()));
}

#[cfg(target_arch = "wasm32")]
pub fn spawn<F, Fut>(make: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(make());
}

/// Bloklovchi ishni bajaradi. Desktopda — alohida ipda, natija
/// kutiladi; bir nechta shunday ish `join_all` bilan parallel ketadi.
#[cfg(not(target_arch = "wasm32"))]
pub async fn run_blocking<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (tx, rx) = futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.await.expect("fon ipi natijasiz tugadi")
}

#[cfg(target_arch = "wasm32")]
pub async fn run_blocking<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    f()
}

/// Kutish — boshqa ishlarni to'xtatmaydi.
#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(d: Duration) {
    run_blocking(move || std::thread::sleep(d)).await
}

#[cfg(target_arch = "wasm32")]
pub async fn sleep(d: Duration) {
    let ms = d.as_millis().min(i32::MAX as u128) as i32;
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Ilova ichidagi shrift (DejaVu Sans, erkin litsenziya —
/// assets/fonts/LICENSE_DEJAVU.txt). Faqat brauzer yig'ilishida: u yerda
/// tizim shriftlari o'qilmaydi. Interfeys va PDF eksporti shuni ishlatadi.
#[cfg(target_arch = "wasm32")]
pub const EMBEDDED_FONT: Option<&[u8]> = Some(include_bytes!("../assets/fonts/DejaVuSans.ttf"));
#[cfg(not(target_arch = "wasm32"))]
pub const EMBEDDED_FONT: Option<&[u8]> = None;

/// Diagnostika sozlamasi: desktopda muhit o'zgaruvchisi (`QURAI_SCREEN`),
/// brauzerda manzil parametri (`?screen=`). Sinov va suratga olish uchun.
pub fn diag(name: &str) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var(format!("QURAI_{}", name.to_uppercase())).ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let search = web_sys::window()?.location().search().ok()?;
        search
            .trim_start_matches('?')
            .split('&')
            .filter_map(|kv| kv.split_once('='))
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.to_string())
    }
}

// =================================================================== Fayl
//
// Desktopda fayl — diskdagi fayl. Brauzerda disk yo'q:
//
// * **Saqlash** dialogi virtual yo'l qaytaradi: `dl/<nom>`. Shu yo'lga
//   yozilgan narsa (`create`, `write`) brauzerda **yuklab olinadi**.
// * **Ochish** asinxron: foydalanuvchi faylni tanlaydi, u virtual yo'l
//   (`in/<nom>`) bo'lib xotiraga tushadi va `read` uni beradi.
// * **Ilova fayllari** (ilovaga qo'shilgan rasm, hujjat) brauzerda
//   alohida `files.db` bazasida, IndexedDB ichida saqlanadi.

use std::io;
use std::path::{Path, PathBuf};

/// `rfd::FileDialog` o'rnini bosuvchi: interfeys bir xil, brauzerda ham
/// ishlaydi.
#[derive(Default)]
pub struct FileDialog {
    title: String,
    file_name: String,
    filters: Vec<(String, Vec<String>)>,
}

impl FileDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }

    pub fn set_file_name(mut self, n: impl Into<String>) -> Self {
        self.file_name = n.into();
        self
    }

    pub fn add_filter(mut self, name: impl Into<String>, ext: &[&str]) -> Self {
        self.filters
            .push((name.into(), ext.iter().map(|e| e.to_string()).collect()));
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn native(&self) -> rfd::FileDialog {
        let mut d = rfd::FileDialog::new();
        if !self.title.is_empty() {
            d = d.set_title(&self.title);
        }
        if !self.file_name.is_empty() {
            d = d.set_file_name(&self.file_name);
        }
        for (n, e) in &self.filters {
            let e: Vec<&str> = e.iter().map(String::as_str).collect();
            d = d.add_filter(n, &e);
        }
        d
    }

    /// Saqlash joyi. Brauzerda dialog yo'q — fayl yuklab olinadi.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_file(self) -> Option<PathBuf> {
        self.native().save_file()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn save_file(self) -> Option<PathBuf> {
        let name = if self.file_name.is_empty() {
            let ext = self
                .filters
                .first()
                .and_then(|f| f.1.first())
                .cloned()
                .unwrap_or_else(|| "bin".into());
            format!("qurai.{ext}")
        } else {
            self.file_name.clone()
        };
        Some(PathBuf::from(format!("dl/{name}")))
    }

    /// Bitta faylni tanlash. Har kadrda chaqiriladi: `start` — tugma
    /// bosildi. Desktopda dialog shu zahoti ochiladi va javob qaytadi;
    /// brauzerda tanlash asinxron — javob keyingi kadrlardan birida
    /// (`key` bo'yicha) keladi.
    pub fn pick_file_when(self, start: bool, key: &str) -> Option<PathBuf> {
        self.pick_ctx(start.then(String::new), key, false)
            .and_then(|(v, _)| v.into_iter().next())
    }

    /// Bir nechta fayl.
    pub fn pick_files_when(self, start: bool, key: &str) -> Option<Vec<PathBuf>> {
        self.pick_ctx(start.then(String::new), key, true)
            .map(|(v, _)| v)
    }

    /// Tanlash va unga bog'liq kontekst (masalan, hujjat raqami): javob
    /// kech kelsa ham qaysi narsa uchun tanlangani yo'qolmaydi.
    /// `start` — `Some(kontekst)` bo'lsa tanlash boshlanadi.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn pick_ctx(
        self,
        start: Option<String>,
        _key: &str,
        many: bool,
    ) -> Option<(Vec<PathBuf>, String)> {
        let ctx = start?;
        let files = if many {
            self.native().pick_files()
        } else {
            self.native().pick_file().map(|p| vec![p])
        }?;
        Some((files, ctx))
    }

    /// Brauzerda fayl oynasini faqat foydalanuvchi bosishi ochadi, egui
    /// kadri esa bosishdan keyin chiziladi. Shuning uchun sahifaga kichik
    /// HTML panel chiqariladi («Faylni tanlash» tugmasi): uni bosish
    /// haqiqiy bosish bo'lib, oyna darhol ochiladi. Tanlangach panel
    /// yo'qoladi, fayl keyingi kadrda keladi.
    #[cfg(target_arch = "wasm32")]
    pub fn pick_ctx(
        self,
        start: Option<String>,
        key: &str,
        many: bool,
    ) -> Option<(Vec<PathBuf>, String)> {
        if let Some(ctx) = start {
            let accept: Vec<String> = self
                .filters
                .iter()
                .flat_map(|(_, e)| e.iter().map(|x| format!(".{}", x.to_lowercase())))
                .collect();
            let title = if self.title.is_empty() {
                crate::i18n::t("pick_file_title").to_string()
            } else {
                self.title.clone()
            };
            web::show_picker(key.to_string(), ctx, title, accept.join(","), many);
        }
        web::READY.with(|m| m.borrow_mut().remove(key))
    }
}

/// Tanlangan faylni ilovaga **saqlab qo'yadi** va doimiy yo'lini qaytaradi.
/// Desktopda fayl joyida qoladi (yo'li yoziladi). Brauzerda tanlangan fayl
/// faqat xotirada — u `files.db` ga ko'chiriladi.
pub fn keep(path: &Path) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        path.display().to_string()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "fayl".into());
        let id = uuid::Uuid::new_v4().simple().to_string();
        let to = PathBuf::from(format!("att/{}/{name}", &id[..12]));
        match read(path).and_then(|b| write(&to, &b)) {
            Ok(()) => to.to_string_lossy().replace('\\', "/"),
            Err(_) => path.display().to_string(),
        }
    }
}

/// Faylni tashqi dasturda ochadi: desktopda — tizim dasturi, brauzerda —
/// yuklab olish.
pub fn open_external(path: &str) {
    #[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
    {
        // `explorer` nolga teng bo'lmagan kod qaytarishi mumkin — bu xato emas.
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    }
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "windows")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
    #[cfg(target_arch = "wasm32")]
    {
        let p = Path::new(path);
        if let Ok(bytes) = read(p) {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "fayl".into());
            let _ = download(&name, &bytes);
        }
    }
}

/// Yozish uchun fayl: desktopda diskdagi fayl, brauzerda xotira (yopilganda
/// yuklab olinadi yoki `files.db` ga yoziladi).
pub enum FileOut {
    #[cfg(not(target_arch = "wasm32"))]
    Disk(std::fs::File),
    #[cfg(target_arch = "wasm32")]
    Mem(PathBuf, io::Cursor<Vec<u8>>),
}

impl io::Write for FileOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            FileOut::Disk(f) => f.write(buf),
            #[cfg(target_arch = "wasm32")]
            FileOut::Mem(_, c) => c.write(buf),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            FileOut::Disk(f) => f.flush(),
            #[cfg(target_arch = "wasm32")]
            FileOut::Mem(..) => Ok(()),
        }
    }
}

impl io::Seek for FileOut {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            FileOut::Disk(f) => f.seek(pos),
            #[cfg(target_arch = "wasm32")]
            FileOut::Mem(_, c) => c.seek(pos),
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for FileOut {
    fn drop(&mut self) {
        let FileOut::Mem(path, c) = self;
        let bytes = std::mem::take(c.get_mut());
        let _ = write(path, &bytes);
    }
}

/// Yozish uchun fayl ochadi.
pub fn create(path: &Path) -> io::Result<FileOut> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::File::create(path).map(FileOut::Disk)
    }
    #[cfg(target_arch = "wasm32")]
    {
        Ok(FileOut::Mem(
            path.to_path_buf(),
            io::Cursor::new(Vec::new()),
        ))
    }
}

/// Faylga yozadi.
pub fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::write(path, bytes)
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Ok(rest) = path.strip_prefix("dl") {
            let name = rest.to_string_lossy().to_string();
            return download(&name, bytes).map_err(io::Error::other);
        }
        web::store_put(path, bytes)
    }
}

/// Faylni o'qiydi.
pub fn read(path: &Path) -> io::Result<Vec<u8>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read(path)
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(b) = web::INBOX.with(|m| m.borrow().get(path).cloned()) {
            return Ok(b);
        }
        web::store_get(path)
    }
}

/// Faylni matn sifatida o'qiydi.
pub fn read_to_string(path: &Path) -> io::Result<String> {
    String::from_utf8(read(path)?).map_err(io::Error::other)
}

/// Fayl bormi.
pub fn exists(path: &Path) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        path.exists()
    }
    #[cfg(target_arch = "wasm32")]
    {
        // Har kadrda bazadan o'qimaslik uchun natija eslab qolinadi.
        if let Some(v) = web::EXISTS.with(|m| m.borrow().get(path).copied()) {
            return v;
        }
        let v = read(path).is_ok();
        web::EXISTS.with(|m| m.borrow_mut().insert(path.to_path_buf(), v));
        v
    }
}

/// Rasm manzili `egui::Image` uchun: desktopda `file://`, brauzerda
/// baytlar bir marta xotiraga yuklanadi (`bytes://`).
pub fn image_uri(ctx: &egui::Context, path: &str) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = ctx;
        format!("file://{path}")
    }
    #[cfg(target_arch = "wasm32")]
    {
        let uri = format!("bytes://{path}");
        let loaded = web::IMAGES.with(|m| m.borrow().contains(&uri));
        if !loaded {
            if let Ok(bytes) = read(Path::new(path)) {
                ctx.include_bytes(uri.clone(), bytes);
            }
            web::IMAGES.with(|m| m.borrow_mut().insert(uri.clone()));
        }
        uri
    }
}

/// Brauzerda faylni yuklab olishga beradi.
#[cfg(target_arch = "wasm32")]
pub fn download(name: &str, bytes: &[u8]) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    let err = |e: wasm_bindgen::JsValue| format!("{e:?}");
    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::of1(&array);
    let blob = web_sys::Blob::new_with_u8_array_sequence(&parts).map_err(err)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(err)?;
    let doc = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("document yo'q")?;
    let a: web_sys::HtmlAnchorElement = doc
        .create_element("a")
        .map_err(err)?
        .dyn_into()
        .map_err(|_| "a emas".to_string())?;
    a.set_href(&url);
    a.set_download(name);
    a.click();
    // Manzil darhol o'chirilmaydi: Firefox va Safari yuklab olishni
    // keyingi tsiklda boshlaydi — oldin o'chirilsa fayl tushmaydi.
    let revoke = wasm_bindgen::closure::Closure::once_into_js(move || {
        let _ = web_sys::Url::revoke_object_url(&url);
    });
    if let Some(w) = web_sys::window() {
        let _ =
            w.set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 60_000);
    }
    Ok(())
}

// ================================================================= Kadr

/// Canvas piksel o'lchamini tekshiradi va kerak bo'lsa to'g'rilaydi.
///
/// eframe o'lchamni `devicePixelContentBoxSize` dan oladi. Ba'zi muhitlarda
/// (Chrome emulyatsiyasi, eski brauzerlar) u CSS pikselda keladi va ilova
/// zichlik barobaricha kattalashib, ekranga sig'may qoladi. To'g'ri o'lcham:
/// CSS o'lchami × `devicePixelRatio`. Farq bo'lsa — shu qiymat yoziladi.
#[cfg(target_arch = "wasm32")]
pub fn fix_canvas_size() {
    use wasm_bindgen::JsCast;
    let Some(win) = web_sys::window() else { return };
    let dpr = win.device_pixel_ratio();
    let Some(canvas) = win
        .document()
        .and_then(|d| d.get_element_by_id("qurai_canvas"))
        .and_then(|c| c.dyn_into::<web_sys::HtmlCanvasElement>().ok())
    else {
        return;
    };
    let rect = canvas.get_bounding_client_rect();
    let want_w = (rect.width() * dpr).round() as u32;
    let want_h = (rect.height() * dpr).round() as u32;
    // Bir-ikki piksel yaxlitlash farqi e'tiborsiz.
    if want_w > 0 && canvas.width().abs_diff(want_w) > 2 {
        canvas.set_width(want_w);
        canvas.set_height(want_h);
        repaint();
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn fix_canvas_size() {}

thread_local! {
    static CTX: std::cell::RefCell<Option<egui::Context>> = const { std::cell::RefCell::new(None) };
}

/// Interfeys konteksti — fon ishi tugaganda kadrni qayta chizdirish uchun.
pub fn set_ctx(ctx: &egui::Context) {
    CTX.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(ctx.clone());
        }
    });
}

/// Kadrni qayta chizdiradi (fayl tanlandi, javob keldi).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn repaint() {
    CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.request_repaint();
        }
    });
}

/// Brauzer ichki holati: tanlangan fayllar va ilova fayllari bazasi.
#[cfg(target_arch = "wasm32")]
mod web {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::io;
    use std::path::{Path, PathBuf};

    thread_local! {
        /// Tanlangan fayllar mazmuni: virtual yo'l → baytlar.
        pub static INBOX: RefCell<HashMap<PathBuf, Vec<u8>>> = RefCell::new(HashMap::new());
        /// Tayyor tanlovlar: kalit → yo'llar.
        pub static READY: RefCell<HashMap<String, (Vec<PathBuf>, String)>> = RefCell::new(HashMap::new());
        static STORE: RefCell<Option<rusqlite::Connection>> = const { RefCell::new(None) };
        /// `exists` natijalari.
        pub static EXISTS: RefCell<HashMap<PathBuf, bool>> = RefCell::new(HashMap::new());
        /// Xotiraga yuklangan rasmlar.
        pub static IMAGES: RefCell<std::collections::HashSet<String>> = RefCell::new(Default::default());
    }

    fn with_store<T>(
        f: impl FnOnce(&rusqlite::Connection) -> rusqlite::Result<T>,
    ) -> io::Result<T> {
        STORE.with(|s| {
            let mut s = s.borrow_mut();
            if s.is_none() {
                let c = rusqlite::Connection::open("files.db").map_err(io::Error::other)?;
                c.execute_batch(
                    "CREATE TABLE IF NOT EXISTS file (path TEXT PRIMARY KEY, data BLOB NOT NULL)",
                )
                .map_err(io::Error::other)?;
                *s = Some(c);
            }
            f(s.as_ref().expect("ochildi")).map_err(io::Error::other)
        })
    }

    /// Fayl tanlash paneli (HTML): sarlavha, «Faylni tanlash» va «Bekor».
    pub fn show_picker(key: String, ctx: String, title: String, accept: String, many: bool) {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;
        let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Some(body) = doc.body() else { return };
        if let Some(old) = doc.get_element_by_id("qurai_picker") {
            old.remove();
        }
        let Ok(panel) = doc.create_element("div") else {
            return;
        };
        panel.set_id("qurai_picker");
        let _ = panel.set_attribute(
            "style",
            "position:fixed;inset:0;z-index:50;display:grid;place-items:center;\
             background:rgba(0,0,0,.35);font-family:system-ui,'Segoe UI',sans-serif",
        );
        let card = "<div style=\"background:#fff;color:#1b2533;border-radius:12px;padding:20px 22px;\
             width:min(380px,calc(100vw - 32px));box-shadow:0 8px 30px rgba(0,0,0,.25)\">\
             <div data-part=\"title\" style=\"font-size:16px;font-weight:600;margin-bottom:6px\"></div>\
             <div data-part=\"hint\" style=\"font-size:13px;color:#5c6b80;margin-bottom:14px\"></div>\
             <div style=\"display:flex;gap:8px;justify-content:flex-end\">\
             <button data-act=\"cancel\" style=\"padding:8px 14px;border-radius:8px;border:1px solid #cdd5e0;\
             background:#fff;font-size:14px;cursor:pointer\"></button>\
             <button data-act=\"pick\" style=\"padding:8px 14px;border-radius:8px;border:0;background:#1f4e9c;\
             color:#fff;font-size:14px;font-weight:600;cursor:pointer\"></button></div>\
             </div>";
        panel.set_inner_html(card);
        // Matnlar `textContent` bilan — HTML sifatida talqin qilinmaydi.
        let set = |sel: &str, text: &str| {
            if let Ok(Some(el)) = panel.query_selector(sel) {
                el.set_text_content(Some(text));
            }
        };
        set("[data-part=title]", &title);
        set("[data-part=hint]", crate::i18n::t("pick_file_hint"));
        set("button[data-act=cancel]", crate::i18n::t("cancel"));
        set("button[data-act=pick]", crate::i18n::t("pick_file_button"));
        // Fayl maydoni alohida yaratiladi (HTML matni ichidan qidirish
        // brauzerlarda turlicha ishlaydi).
        let input: web_sys::HtmlInputElement = match doc
            .create_element("input")
            .ok()
            .and_then(|e| e.dyn_into().ok())
        {
            Some(i) => i,
            None => {
                web_sys::console::error_1(&"QURAi: fayl maydoni yaratilmadi".into());
                return;
            }
        };
        input.set_type("file");
        let _ = input.set_attribute("style", "display:none");
        let _ = panel.append_child(&input);
        input.set_accept(&accept);
        input.set_multiple(many);

        // «Faylni tanlash» — haqiqiy bosish ichida oyna ochiladi.
        if let Ok(Some(btn)) = panel.query_selector("button[data-act=pick]") {
            let inp = input.clone();
            let on = Closure::<dyn FnMut()>::new(move || inp.click());
            let _ = btn.add_event_listener_with_callback("click", on.as_ref().unchecked_ref());
            on.forget();
        }
        if let Ok(Some(btn)) = panel.query_selector("button[data-act=cancel]") {
            let p = panel.clone();
            let on = Closure::<dyn FnMut()>::new(move || p.remove());
            let _ = btn.add_event_listener_with_callback("click", on.as_ref().unchecked_ref());
            on.forget();
        }
        // Fayl tanlandi — o'qiladi, panel yopiladi.
        {
            let inp = input.clone();
            let p = panel.clone();
            let on = Closure::<dyn FnMut()>::new(move || {
                let Some(list) = inp.files() else { return };
                let files: Vec<web_sys::File> =
                    (0..list.length()).filter_map(|i| list.get(i)).collect();
                p.remove();
                if files.is_empty() {
                    return;
                }
                let key = key.clone();
                let ctx = ctx.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let mut paths = Vec::new();
                    for f in files {
                        let Ok(buf) = wasm_bindgen_futures::JsFuture::from(f.array_buffer()).await
                        else {
                            continue;
                        };
                        let bytes = js_sys::Uint8Array::new(&buf).to_vec();
                        let path = PathBuf::from(format!("in/{}", f.name()));
                        INBOX.with(|m| m.borrow_mut().insert(path.clone(), bytes));
                        paths.push(path);
                    }
                    if !paths.is_empty() {
                        READY.with(|m| m.borrow_mut().insert(key, (paths, ctx)));
                        super::repaint();
                    }
                });
            });
            let _ = input.add_event_listener_with_callback("change", on.as_ref().unchecked_ref());
            on.forget();
        }
        let _ = body.append_child(&panel);
    }

    pub fn store_put(path: &Path, bytes: &[u8]) -> io::Result<()> {
        EXISTS.with(|m| m.borrow_mut().insert(path.to_path_buf(), true));
        let key = path.to_string_lossy().replace('\\', "/");
        with_store(|c| {
            c.execute(
                "INSERT OR REPLACE INTO file (path, data) VALUES (?1, ?2)",
                rusqlite::params![key, bytes],
            )
            .map(|_| ())
        })
    }

    pub fn store_get(path: &Path) -> io::Result<Vec<u8>> {
        let key = path.to_string_lossy().replace('\\', "/");
        with_store(|c| c.query_row("SELECT data FROM file WHERE path = ?1", [key], |r| r.get(0)))
    }
}
