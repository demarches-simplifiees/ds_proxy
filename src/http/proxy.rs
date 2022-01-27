use super::super::config::Config;
use actix_web::guard;
use actix_web::{middleware, web, App, HttpServer};

use super::handlers::*;
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

#[actix_web::main]
pub async fn main(config: Config) -> std::io::Result<()> {
    let address = config.address.unwrap();
    let max_conn = config.max_connections;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(
                awc::Client::builder()
                    .connector(
                        awc::Connector::new().timeout(CONNECT_TIMEOUT), // max time to connect to remote host including dns name resolution
                    )
                    .timeout(RESPONSE_TIMEOUT) // the total time before a response must be received
                    .finish(),
            ))
            .app_data(web::Data::new(config.clone()))
            .wrap(middleware::Logger::default())
            .service(web::resource("/ping").guard(guard::Get()).to(ping))
            .service(
                web::scope("/backend")
                    .service(web::resource("{name}*").guard(guard::Get()).to(fetch))
                    .service(web::resource("{name}*").guard(guard::Put()).to(forward))
                    .service(web::resource("{name}*").to(simple_proxy)),
            )
            .service(web::resource("{name}*").guard(guard::Get()).to(fetch))
            .service(web::resource("{name}*").guard(guard::Put()).to(forward))
            .service(web::resource("{name}*").to(simple_proxy))
    })
    .max_connections(max_conn)
    .keep_alive(actix_http::KeepAlive::Disabled)
    .bind_uds("/tmp/actix-uds.socket")?
    .bind(address)?
    .run()
    .await
}
