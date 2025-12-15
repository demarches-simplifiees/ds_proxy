use std::time::{Duration, SystemTime};

use aws_sdk_s3::config::Credentials;
use aws_sigv4::http_request::{
    sign, PercentEncodingMode, SignableRequest, SignatureLocation, SigningInstructions,
    SigningSettings,
};
use aws_sigv4::sign::v4::SigningParams;

#[derive(Debug, Clone)]
pub struct AwsConfig {
    pub bypass_signature_check: bool,
    credentials: Credentials,
    region: String,
}

impl AwsConfig {
    pub fn new(credentials: Credentials, region: String, bypass_signature_check: bool) -> Self {
        AwsConfig {
            bypass_signature_check,
            credentials,
            region,
        }
    }

    pub fn sign<'a>(
        self,
        time: SystemTime,
        request: SignableRequest<'a>,
        expires_in: Option<Duration>,
        encoding_mode: PercentEncodingMode,
    ) -> (SigningInstructions, String) {
        let mut settings = SigningSettings::default();
        settings.percent_encoding_mode = encoding_mode;
        settings.expires_in = expires_in;

        if expires_in.is_some() {
            settings.signature_location = SignatureLocation::QueryParams;
        }

        let identity = self.credentials.into();
        let signing_params = SigningParams::builder()
            .identity(&identity)
            .region(&self.region)
            .name("s3")
            .time(time)
            .settings(settings)
            .build()
            .unwrap()
            .into();

        sign(request, &signing_params).unwrap().into_parts()
    }
}
