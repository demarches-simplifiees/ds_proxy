use super::super::config::Config;
use actix_web::guard::{Get, Put};
use actix_web::{
    middleware,
    web::{resource, scope, Data},
    App, HttpServer,
};

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
            .app_data(Data::new(
                awc::Client::builder()
                    .connector(
                        awc::Connector::new().timeout(CONNECT_TIMEOUT), // max time to connect to remote host including dns name resolution
                    )
                    .timeout(RESPONSE_TIMEOUT) // the total time before a response must be received
                    .finish(),
            ))
            .app_data(Data::new(config.clone()))
            .wrap(middleware::Logger::default())
            .service(resource("/ping").guard(Get()).to(ping))
            .service(
                scope("/upstream")
                    .service(resource("{name}*").guard(Get()).to(fetch))
                    .service(resource("{name}*").guard(Put()).to(forward))
                    .service(resource("{name}*").to(simple_proxy)),
            )
    })
    .max_connections(max_conn)
    .keep_alive(actix_http::KeepAlive::Disabled)
    .bind_uds("/tmp/actix-uds.socket")?
    .bind(address)?
    .run()
    .await
}
