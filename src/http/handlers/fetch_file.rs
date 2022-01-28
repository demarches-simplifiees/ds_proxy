use super::*;
use actix_web::{HttpRequest, HttpResponse};

pub async fn fetch_file(req: HttpRequest, config: web::Data<Config>) -> HttpResponse {
    let filepath = config.local_encryption_path_for(&req);

    actix_files::NamedFile::open(filepath)
        .unwrap()
        .into_response(&req)
}
