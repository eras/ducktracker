use actix_web::{Scope, web};

use mime_guess::from_path;

use actix_web::{HttpResponse, Responder};

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "./frontend/dist/"] // Path to your assets directory
struct EmbeddedAssets;

async fn embedded_assets_handler(
    path: web::Path<String>,
    index_file_name_data: web::Data<String>,
) -> impl Responder {
    let file_path = if path.is_empty() {
        index_file_name_data.as_str()
    } else {
        path.as_str()
    };

    if let Some(embedded_file) = EmbeddedAssets::get(file_path) {
        let mime_type = from_path(file_path).first_or_octet_stream(); // Fallback to application/octet-stream

        HttpResponse::Ok()
            .content_type(mime_type.to_string())
            .body(embedded_file.data)
    } else {
        HttpResponse::NotFound().body("File not found")
    }
}

pub fn assets(path_prefix: &str, index_file_name: &str) -> Scope {
    let mut scope = web::scope(path_prefix);

    let index_data = web::Data::new(index_file_name.to_string());
    scope = scope
        .app_data(index_data)
        .route("/{path:.*}", web::get().to(embedded_assets_handler));

    scope
}
