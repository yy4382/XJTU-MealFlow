use actix_web::{HttpRequest, HttpResponse, Responder};
use rust_embed::RustEmbed;

pub mod api;

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct FrontendAssets;

pub async fn serve_frontend(req: HttpRequest) -> impl Responder {
    let mut path = req.path().trim_start_matches('/').to_string();

    // 如果路径为空 (根路径)，则默认为 index.html
    if path.is_empty() {
        path = "index.html".to_string();
    }

    // 尝试从嵌入资源中获取文件
    match FrontendAssets::get(&path) {
        Some(asset) => {
            let mime_type = mime_guess::from_path(&path).first_or_octet_stream();
            HttpResponse::Ok()
                .content_type(mime_type.as_ref())
                .body(asset.data)
        }
        None => {
            // 如果找不到特定文件，则对于 SPA，总是返回 index.html
            // 这样客户端路由才能工作
            if let Some(index_html) = FrontendAssets::get("index.html") {
                HttpResponse::Ok()
                    .content_type("text/html; charset=utf-8")
                    .body(index_html.data)
            } else {
                // 理论上 index.html 应该总是存在于 build 产物中
                HttpResponse::NotFound().body("404: index.html not found in embedded assets")
            }
        }
    }
}
