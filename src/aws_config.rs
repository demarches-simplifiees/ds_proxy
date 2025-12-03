use std::time::SystemTime;

use aws_sdk_s3::config::Credentials;
use aws_sigv4::http_request::{
    sign, PercentEncodingMode, SignableRequest, SigningInstructions, SigningSettings,
};
use aws_sigv4::sign::v4::SigningParams;
use aws_sigv4::SigningOutput;

#[derive(Debug, Clone)]
pub struct AwsConfig {
    credentials: Credentials,
    region: String,
}

impl AwsConfig {
    pub fn new(credentials: Credentials, region: String) -> Self {
        AwsConfig {
            credentials,
            region,
        }
    }

    pub fn sign<'a>(
        self,
        time: SystemTime,
        request: SignableRequest<'a>,
    ) -> SigningOutput<SigningInstructions> {
        let mut settings = SigningSettings::default();
        settings.percent_encoding_mode = PercentEncodingMode::Single;

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

        sign(request, &signing_params).unwrap()
    }
}
