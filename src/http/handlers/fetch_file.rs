use super::*;
use actix_web::{HttpRequest, HttpResponse};

pub async fn fetch_file(req: HttpRequest, config: web::Data<Config>) -> HttpResponse {
    let filepath = config.local_encryption_path_for(&req);

    match actix_files::NamedFile::open(filepath) {
        Ok(named_file) => named_file.into_response(&req),
        _ => HttpResponse::NotFound().finish(),
    }
}
