use actix_web::http::header;
use actix_web::HttpResponse;

pub async fn ping() -> HttpResponse {
    let mut response = match std::env::current_dir() {
        Ok(path_buff) => {
            if path_buff.join("maintenance").exists() {
                HttpResponse::NotFound()
            } else {
                HttpResponse::Ok()
            }
        }

        // the server cannot even read a directory
        Err(_) => HttpResponse::InternalServerError(),
    };

    response
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .body("{}")
}
