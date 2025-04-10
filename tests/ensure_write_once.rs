extern crate ds_proxy;

use actix_web::guard::Get;
use actix_web::web::resource;
use actix_web::HttpResponse;
use ds_proxy::http::middlewares::{ensure_write_once, hash_key};
use ds_proxy::redis_utils::create_redis_pool;
use std::env;
use url::Url;

pub async fn mock_service() -> HttpResponse {
    let mut response = HttpResponse::Ok();

    response.body("Hello, world!")
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{middleware::from_fn, test, web, App};
    use redis::AsyncCommands;

    #[actix_web::test]
    async fn test_ensure_write_once() {
        let redis_url = env::var("REDIS_URL")
            .ok()
            .and_then(|url| Url::parse(&url).ok())
            .unwrap_or_else(|| Url::parse("redis://127.0.0.1").unwrap());
        let redis_pool = create_redis_pool(Some(redis_url)).await;

        // Prépare une application Actix Web avec le middleware
        let mut actix_app = App::new()
            .app_data(web::Data::new(redis_pool.clone()))
            .service(
                resource("/test-path")
                    .guard(Get())
                    .wrap(from_fn(ensure_write_once))
                    .to(mock_service),
            );

        if let Some(ref redis_pool) = redis_pool {
            log::info!("Redis pool available.");
            actix_app = actix_app.app_data(web::Data::new(redis_pool.clone()));
            // on clean la clé
            match redis_pool.get().await {
                Ok(mut conn) => conn.del(hash_key("/test-path")).await.unwrap(),
                Err(_err) => {}
            }
        }

        let app = test::init_service(actix_app).await;

        // Effectue une première requête (devrait passer, et de maniere sous-jacente, écrire dans Redis)
        let req = test::TestRequest::get().uri("/test-path").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Effectue une seconde requête (devrait être refusée)
        let req = test::TestRequest::get().uri("/test-path").to_request();
        let resp2 = test::try_call_service(&app, req).await;
        match resp2 {
            Ok(_) => panic!("Expected an error, but got a response. do you have a redis running on 127.0.0.1:6379 ?"),
            Err(err) => {
                assert_eq!(err.error_response().status(), 403);
            }
        }
    }
}
