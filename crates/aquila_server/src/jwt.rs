use aquila_core::prelude::*;

use jsonwebtoken::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtBackend for JwtService {
    fn mint(
        &self,
        subject: String,
        scopes: Vec<String>,
        duration_seconds: u64,
    ) -> Result<String, AuthError> {
        let expiration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AuthError::System(e.to_string()))?
            .as_secs()
            + duration_seconds;

        let claims = Claims {
            sub: subject,
            exp: expiration,
            scopes,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::System(e.to_string()))
    }

    fn verify(&self, token: &str) -> Result<User, AuthError> {
        if token.is_empty() {
            return Err(AuthError::Missing);
        }

        let validation = Validation::default();
        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|_| AuthError::Invalid)?;

        Ok(User {
            id: token_data.claims.sub,
            scopes: token_data.claims.scopes,
        })
    }
}

impl JwtService {
    /// - `secret`: The secret used to for JWT tokens.
    ///
    /// **NOTE:** This should be set to a secure value!
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }
}
