mod fetch;
mod forward;
mod ping;
mod simple_proxy;

pub use fetch::fetch;
pub use forward::forward;
pub use ping::ping;
pub use simple_proxy::simple_proxy;

use actix_web::http::header;

pub static FETCH_RESPONSE_HEADERS_TO_REMOVE: [header::HeaderName; 2] = [
    // Connection settings (keepalived) must not be resend
    header::CONNECTION,
    // Encryption changes the length of the content
    // and we use chunk transfert-encoding
    header::CONTENT_LENGTH,
];

pub static FETCH_REQUEST_HEADERS_TO_REMOVE: [header::HeaderName; 2] = [
    // Connection settings (keepalived) must not be resend
    header::CONNECTION,
    header::RANGE,
];
