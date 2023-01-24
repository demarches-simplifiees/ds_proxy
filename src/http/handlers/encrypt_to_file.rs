use super::*;
use std::fs::File;
use std::io::Write;

pub async fn encrypt_to_file(
    req: HttpRequest,
    config: web::Data<HttpConfig>,
    payload: web::Payload,
) -> HttpResponse {
    let filepath = config.local_encryption_path_for(&req);

    let key = config.keyring.get_last_key().expect("no key avalaible for encryption");

    let mut encrypted_stream = Encoder::new(key, config.chunk_size, Box::new(payload));

    // File::create is blocking operation, use threadpool
    let mut f = web::block(move || File::create(filepath))
        .await
        .unwrap()
        .unwrap();

    while let Ok(Some(chunk)) = encrypted_stream.try_next().await {
        f = web::block(move || f.write_all(&chunk).map(|_| f))
            .await
            .unwrap()
            .unwrap();
    }

    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .body("{}")
}
