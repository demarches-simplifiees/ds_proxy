use super::super::config::HttpConfig;
use actix_web::dev::ServiceResponse;
use actix_web::web;
use actix_web::Error;
use std::path::Path;

pub fn erase_file(res: Result<ServiceResponse, Error>) -> Result<ServiceResponse, Error> {
    let response = res.unwrap();
    let request = response.request();

    let filepath = request
        .app_data::<web::Data<HttpConfig>>()
        .unwrap()
        .local_encryption_path_for(request);

    if Path::new(&filepath).exists() {
        std::fs::remove_file(filepath).unwrap();
    }

    Ok(response)
}
