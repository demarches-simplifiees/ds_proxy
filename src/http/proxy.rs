use super::super::config::{HttpConfig, RedisConfig};
use super::handlers::*;
use super::middlewares::*;
use crate::redis_utils::configure_redis_pool;
use crate::write_once_service::WriteOnceService;
use actix_web::dev::Service;
use actix_web::guard::{Get, Put};
use actix_web::{
    middleware,
    middleware::from_fn,
    web::{resource, scope, Data},
    App, HttpServer,
};
use futures::FutureExt;
use std::time::Duration;

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

#[actix_web::main]
pub async fn main(config: HttpConfig, redis_config: RedisConfig) -> std::io::Result<()> {
    let address = config.address;

    HttpServer::new(move || {
        let mut app = App::new()
            .app_data(Data::new(
                awc::Client::builder()
                    .connector(
                        awc::Connector::new().timeout(config.backend_connection_timeout), // max time to connect to remote host including dns name resolution
                    )
                    .timeout(RESPONSE_TIMEOUT) // the total time before a response must be received
                    .finish(),
            ))
            .app_data(Data::new(config.clone()))
            .wrap(middleware::Logger::default())
            .service(resource("/ping").guard(Get()).to(ping))
            .service({
                let scope = scope("/upstream").service(resource("{name}*").guard(Get()).to(fetch));

                let upstream_put = resource("{name}*").guard(Put()).to(forward);

                if config.write_once {
                    scope.service(upstream_put.wrap(from_fn(ensure_write_once)))
                } else {
                    scope.service(upstream_put)
                }
                .service(resource("{name}*").to(simple_proxy))
            })
            .service(
                scope("/local")
                    .service(resource("encrypt/{name}").guard(Put()).to(encrypt_to_file))
                    .service(
                        resource("encrypt/{name}")
                            .guard(Get())
                            .wrap_fn(|req, srv| srv.call(req).map(erase_file))
                            .to(fetch_file),
                    ),
            );

        if config.write_once {
            let redis_pool =
                configure_redis_pool(&config, &redis_config).expect("Failed to create Redis pool");

            app = app.app_data(Data::new(WriteOnceService::new(redis_pool)))
        }

        app
    })
    .keep_alive(actix_http::KeepAlive::Disabled)
    .bind_uds("/tmp/actix-uds.socket")?
    .bind(address)?
    .run()
    .await
}
