extern crate ds_proxy;

use actix_web::guard::Get;
use actix_web::web::resource;
use actix_web::HttpResponse;
use ds_proxy::config::RedisConfig;
use ds_proxy::http::middlewares::ensure_write_once;
use ds_proxy::redis_utils::create_redis_pool;

use std::env;
use url::Url;

pub async fn mock_success() -> HttpResponse {
    let mut response = HttpResponse::Ok();

    response.body("Hello, world!")
}

pub async fn mock_found() -> HttpResponse {
    let mut response = HttpResponse::Found();
    response.insert_header(("Location", "http://example.com"));

    response.body("Redirecting...")
}

fn redis_url() -> Url {
    env::var("REDIS_URL")
        .ok()
        .and_then(|url| Url::parse(&url).ok())
        .unwrap_or_else(|| {
            Url::parse("redis://127.0.0.1").expect("Failed to parse default Redis URL")
        })
}

fn redis_config() -> RedisConfig {
    RedisConfig {
        redis_url: Some(redis_url()),
        redis_timeout_wait: Some(std::time::Duration::from_secs(5)),
        redis_timeout_create: Some(std::time::Duration::from_secs(5)),
        redis_timeout_recycle: Some(std::time::Duration::from_secs(5)),
        redis_pool_max_size: Some(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{middleware::from_fn, test, web, App};
    use ds_proxy::write_once_service::WriteOnceService;
    use redis::AsyncCommands;

    #[actix_web::test]
    async fn test_ensure_write_once_with_success() {
        let redis_pool = create_redis_pool(&redis_config()).await;

        // Prépare une application Actix Web avec le middleware
        let mut actix_app = App::new().service(
            resource("/test-success-path")
                .guard(Get())
                .wrap(from_fn(ensure_write_once))
                .to(mock_success),
        );

        if let Some(ref redis_pool) = redis_pool {
            log::info!("Redis pool available.");
            actix_app = actix_app.app_data(web::Data::new(redis_pool.clone()));
            // on clean la clé
            match redis_pool.get().await {
                Ok(mut conn) => conn
                    .del(WriteOnceService::hash_key("/test-success-path"))
                    .await
                    .unwrap(),
                Err(_err) => {}
            }
        }

        let app = test::init_service(actix_app).await;

        // Effectue une première requête (devrait passer, et de maniere sous-jacente, écrire dans Redis)
        let req = test::TestRequest::get()
            .uri("/test-success-path")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Effectue une seconde requête (devrait être refusée)
        let req = test::TestRequest::get()
            .uri("/test-success-path")
            .to_request();
        let resp2 = test::try_call_service(&app, req).await;
        match resp2 {
            Ok(_) => panic!("Expected an error, but got a response. do you have a redis running on 127.0.0.1:6379 ?"),
            Err(err) => {
                assert_eq!(err.error_response().status(), 403);
            }
        }
    }

    #[actix_web::test]
    async fn test_ensure_write_once_with_found() {
        let redis_pool = create_redis_pool(&redis_config()).await;

        // Prépare une application Actix Web avec le middleware
        let mut actix_app = App::new()
            .service(
                resource("/test-success-path")
                    .guard(Get())
                    .wrap(from_fn(ensure_write_once))
                    .to(mock_success),
            )
            .service(
                resource("/test-not-success-path")
                    .guard(Get())
                    .wrap(from_fn(ensure_write_once))
                    .to(mock_found),
            );
        if let Some(ref redis_pool) = redis_pool {
            log::info!("Redis pool available.");
            actix_app = actix_app.app_data(web::Data::new(redis_pool.clone()));
        }

        let app = test::init_service(actix_app).await;

        let req = test::TestRequest::get()
            .uri("/test-not-success-path")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 302);

        let req = test::TestRequest::get()
            .uri("/test-not-success-path")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 302);
    }
}
