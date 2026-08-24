//! Front-end assets (CSS, JS) embedded into the binary at compile time via `include_str!`,
//! so the app ships as a single self-contained executable with no external files or
//! internet access needed at runtime.

use axum::http::header;
use axum::response::IntoResponse;

const APP_CSS: &str = include_str!("../static/css/app.css");
const HTMX_JS: &str = include_str!("../static/js/htmx.min.js");

pub async fn app_css() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], APP_CSS)
}

pub async fn htmx_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        HTMX_JS,
    )
}
