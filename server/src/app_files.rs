//! To'liq QURAi ilovasi brauzerda: `/app`.
//!
//! Ilova WebAssembly bo'lib yig'iladi (`tools/wasm/build.sh` →
//! `dist/web`). Server faqat fayllarni beradi — hisob-kitob va baza
//! brauzerning o'zida (IndexedDB). Papka:
//!
//! 1. `QURAI_WEB_DIR` muhit o'zgaruvchisi;
//! 2. server fayli yonidagi `web/`;
//! 3. joriy papkadagi `dist/web/`.
//!
//! Oldindan siqilgan nusxa (`qurai_bg.wasm.gz`) bo'lsa va brauzer
//! `gzip` ni qabul qilsa — u beriladi: 13 MB o'rniga ~4 MB.

use axum::extract::Path as UrlPath;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use std::path::{Path, PathBuf};

/// Ilova fayllari papkasi.
pub fn web_dir() -> Option<PathBuf> {
    if let Ok(d) = std::env::var("QURAI_WEB_DIR") {
        let p = PathBuf::from(d);
        if p.join("index.html").exists() {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("web");
            if p.join("index.html").exists() {
                return Some(p);
            }
        }
    }
    let p = PathBuf::from("dist/web");
    p.join("index.html").exists().then_some(p)
}

/// Fayl turi kengaytmadan.
fn mime(name: &str) -> &'static str {
    match Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
    {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "json" => "application/manifest+json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "css" => "text/css; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// `/app` — oxiriga `/` qo'yiladi: sahifadagi nisbiy yo'llar (`./qurai.js`)
/// shundagina `/app/` ichidan o'qiladi.
pub async fn root() -> Redirect {
    Redirect::permanent("/app/")
}

pub async fn index(headers: HeaderMap) -> Response {
    send("index.html", &headers)
}

pub async fn file(UrlPath(name): UrlPath<String>, headers: HeaderMap) -> Response {
    send(&name, &headers)
}

fn send(name: &str, headers: &HeaderMap) -> Response {
    // Faqat papka ichidagi oddiy fayl nomi: `..` va pastki papkalar yo'q.
    if name.is_empty()
        || name.contains("..")
        || name.contains('/')
        || name.contains('\\')
        || name.starts_with('.')
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(dir) = web_dir() else {
        return (
            StatusCode::NOT_FOUND,
            "QURAi web ilovasi yig'ilmagan: tools/wasm/build.sh ni ishga tushiring \
             yoki QURAI_WEB_DIR ni ko'rsating.",
        )
            .into_response();
    };
    let path = dir.join(name);
    let gzip_ok = headers
        .get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("gzip"));
    let gz = dir.join(format!("{name}.gz"));
    // Siqilgan nusxa: brauzer qabul qilsa yoki siqilmagani yo'q bo'lsa
    // (o'rnatuvchi faqat `.gz` ni qo'yadi — barcha zamonaviy brauzerlar
    // gzip ni tushunadi).
    let (file, gzipped) = if gz.exists() && (gzip_ok || !path.exists()) {
        (gz, true)
    } else {
        (path, false)
    };
    let Ok(meta) = std::fs::metadata(&file) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    // Versiya belgisi: o'lcham va o'zgartirilgan vaqt. Brauzer har safar
    // so'raydi, fayl o'zgarmagan bo'lsa 304 oladi (yuklamaydi) — yangi
    // versiya esa darhol chiqadi.
    let modified = meta
        .modified()
        .ok()
        .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let etag = format!("\"{:x}-{:x}-{}\"", meta.len(), modified, u8::from(gzipped));
    let fresh = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == etag);
    let mut resp = if fresh {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        match std::fs::read(&file) {
            Ok(b) => b.into_response(),
            Err(_) => return StatusCode::NOT_FOUND.into_response(),
        }
    };
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(mime(name)));
    if gzipped {
        h.insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
    }
    h.insert(header::VARY, HeaderValue::from_static("Accept-Encoding"));
    if let Ok(v) = HeaderValue::from_str(&etag) {
        h.insert(header::ETAG, v);
    }
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_and_unsafe_names() {
        assert_eq!(mime("qurai_bg.wasm"), "application/wasm");
        assert_eq!(mime("index.html"), "text/html; charset=utf-8");
        let h = HeaderMap::new();
        assert_eq!(send("../secret", &h).status(), StatusCode::NOT_FOUND);
        assert_eq!(send("a/b.js", &h).status(), StatusCode::NOT_FOUND);
        assert_eq!(send(".env", &h).status(), StatusCode::NOT_FOUND);
    }
}
