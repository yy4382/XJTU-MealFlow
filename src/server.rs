//! # Web服务器模块
//!
//! 该模块提供HTTP Web服务器功能，支持前端资产服务和SPA路由。
//! 使用 [actix-web](https://actix.rs/) 框架构建高性能的Web服务。
//!
//! ## 主要功能
//!
//! - **静态资产服务**: 提供前端构建产物（HTML、CSS、JS等）
//! - **SPA支持**: 支持单页应用的客户端路由
//! - **嵌入式资源**: 前端资源编译时嵌入到二进制文件中
//! - **MIME类型检测**: 自动检测文件类型并设置正确的Content-Type
//!
//! ## 路由规则
//!
//! - `/`: 默认路由，返回index.html
//! - `/api/*`: API路由（在api模块中定义）
//! - `/*`: 其他所有路径，尝试匹配静态文件，否则返回index.html（SPA支持）
//!
//! ## 使用示例
//!
//! ```rust
//! use actix_web::{web, App, HttpServer};
//! 
//! #[actix_web::main]
//! async fn main() -> std::io::Result<()> {
//!     HttpServer::new(|| {
//!         App::new()
//!             .default_service(web::route().to(serve_frontend))
//!     })
//!     .bind("127.0.0.1:8080")?
//!     .run()
//!     .await
//! }
//! ```

use actix_web::{HttpRequest, HttpResponse, Responder};
use rust_embed::RustEmbed;

/// API路由和处理器模块
///
/// 包含所有REST API的路由定义和请求处理器。
pub mod api;

/// 前端嵌入式资源
///
/// 使用 `rust-embed` 将前端构建产物嵌入到二进制文件中，
/// 实现单一可执行文件部署。
#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct FrontendAssets;

/// 前端静态资源服务处理器
///
/// 这是一个通用的请求处理器，用于服务前端静态文件并支持SPA路由。
/// 
/// ## 处理逻辑
///
/// 1. 从请求路径中提取文件路径
/// 2. 如果路径为空（根路径），默认返回index.html
/// 3. 尝试从嵌入资源中查找对应文件
/// 4. 如果找到文件，返回文件内容和正确的MIME类型
/// 5. 如果未找到文件，返回index.html（支持SPA客户端路由）
/// 6. 如果连index.html都不存在，返回404错误
///
/// ## SPA支持
///
/// 对于单页应用，所有未匹配到静态文件的路径都会返回index.html，
/// 让前端路由器处理页面导航。这是SPA部署的标准做法。
///
/// # 参数
///
/// * `req` - HTTP请求对象，包含请求路径等信息
///
/// # 返回值
///
/// 返回HTTP响应，包含：
/// - 静态文件内容（如果找到对应文件）
/// - index.html内容（用于SPA路由支持）
/// - 404错误（如果连index.html都不存在）
///
/// # 示例
///
/// 
/// - 请求 / -> 返回 index.html
/// - 请求 /app.js -> 返回 app.js文件
/// - 请求 /user/profile -> 返回 index.html（SPA路由）
/// - 请求 /api/data -> 由API路由处理（不会到达这里）
/// 
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
