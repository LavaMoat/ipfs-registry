//! Custom typed headers.
use axum::headers::{self, Header, HeaderName, HeaderValue};

use once_cell::sync::Lazy;

const X_SIGNATURE_NAME: &str = "x-signature";

pub static X_SIGNATURE: Lazy<HeaderName> =
    Lazy::new(|| HeaderName::from_static(X_SIGNATURE_NAME));

/// Represents the `x-signature` header.
pub struct Signature([u8; 65]);

impl Header for Signature {
    fn name() -> &'static HeaderName {
        &X_SIGNATURE
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(headers::Error::invalid)?;
        let value = value.to_str().map_err(|_| headers::Error::invalid())?;
        let value =
            bs58::decode(value)
                .into_vec()
                .map_err(|_| headers::Error::invalid())?;
        let value: [u8; 65] = value.as_slice().try_into()
            .map_err(|_| headers::Error::invalid())?;
        Ok(Signature(value))
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        let s = bs58::encode(self.0).into_string();
        let value = HeaderValue::from_str(&s)
            .expect("failed to create signature header");
        values.extend(std::iter::once(value));
    }
}
