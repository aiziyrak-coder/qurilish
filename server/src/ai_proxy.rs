//! AI so'rovlari uchun o'tkazgich: brauzerdagi ilova → server → OpenAI.
//!
//! Nega kerak: brauzer boshqa saytga so'rov yuborishda CORS qoidalariga
//! bo'ysunadi va ba'zi tarmoqlar (korporativ proksi, brauzer siyosati)
//! tashqi POST ni to'sadi. Ilova o'z serveriga murojaat qiladi — bu har
//! doim ochiq.
//!
//! **Xavfsizlik chegarasi:** server faqat bitta manzilga uzatadi —
//! `QURAI_AI_ENDPOINT` (sukut: OpenAI). Brauzer manzilni tanlay olmaydi,
//! shuning uchun server ichki tarmoqqa «eshik» bo'lib qolmaydi. Kalit
//! brauzerdan `Authorization` sarlavhasida keladi va o'zgarmay uzatiladi;
//! server uni saqlamaydi va jurnalga yozmaydi.

use axum::body::Bytes;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

/// So'rov tanasi chegarasi: varaq PDF lari bilan bir necha MB bo'ladi.
pub const MAX_BODY: usize = 40 * 1024 * 1024;

fn endpoint() -> String {
    std::env::var("QURAI_AI_ENDPOINT")
        .ok()
        .filter(|s| s.starts_with("https://"))
        .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string())
}

pub async fn forward(headers: HeaderMap, body: Bytes) -> Response {
    let Some(auth) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .filter(|v| v.starts_with("Bearer ") && v.len() > 12)
        .map(str::to_string)
    else {
        return (StatusCode::UNAUTHORIZED, "kalit yo'q").into_response();
    };
    let url = endpoint();
    let result = tokio::task::spawn_blocking(move || {
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(600))
            .build();
        match agent
            .post(&url)
            .set("Authorization", &auth)
            .set("Content-Type", "application/json")
            .send_bytes(&body)
        {
            Ok(r) => (r.status(), r.into_string().unwrap_or_default()),
            Err(ureq::Error::Status(code, r)) => (code, r.into_string().unwrap_or_default()),
            // Tarmoq xatosi: matnda kalit yo'q (u faqat sarlavhada edi).
            Err(e) => (
                502,
                format!("{{\"error\":{{\"message\":\"{}\"}}}}", e.kind()),
            ),
        }
    })
    .await;
    let (code, text) = result.unwrap_or((502, String::new()));
    let mut resp = (
        StatusCode::from_u16(code).unwrap_or(StatusCode::BAD_GATEWAY),
        text,
    )
        .into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_must_be_https() {
        std::env::remove_var("QURAI_AI_ENDPOINT");
        assert!(endpoint().starts_with("https://api.openai.com"));
    }

    #[tokio::test]
    async fn no_key_is_refused_without_network() {
        let r = forward(HeaderMap::new(), Bytes::from_static(b"{}")).await;
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    }
}
