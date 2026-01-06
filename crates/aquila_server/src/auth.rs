use crate::jwt::JwtService;
use crate::state::AppState;
use aquila_core::prelude::*;
use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};

/// A wrapper struct indicating a request has been authenticated.
#[derive(Clone, Debug)]
pub struct AuthenticatedUser(pub User);

impl<S, A> FromRequestParts<AppState<S, A>> for AuthenticatedUser
where
    S: StorageBackend,
    A: AuthProvider,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState<S, A>,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("Authorization")
            .and_then(|auth_header| {
                auth_header
                    .to_str()
                    .map(|header_str| {
                        header_str
                            .strip_prefix("Bearer ")
                            .unwrap_or(header_str)
                            .trim()
                    })
                    .ok()
            })
            .unwrap_or("");

        match state.auth.verify(token).await {
            Ok(user) => Ok(AuthenticatedUser(user)),
            Err(_) => Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string())),
        }
    }
}

#[derive(Clone)]
pub struct JWTServiceAuthProvider<P: AuthProvider> {
    jwt_service: JwtService,
    provider: P,
}

impl<P: AuthProvider> JWTServiceAuthProvider<P> {
    pub fn new(jwt_service: JwtService, provider: P) -> Self {
        Self {
            jwt_service,
            provider,
        }
    }
}

impl<P: AuthProvider> AuthProvider for JWTServiceAuthProvider<P> {
    async fn verify(&self, token: &str) -> Result<User, AuthError> {
        if let Ok(user) = self.jwt_service.verify(token) {
            return Ok(user);
        }

        self.provider.verify(token).await
    }

    fn get_login_url(&self) -> Option<String> {
        self.provider.get_login_url()
    }

    async fn exchange_code(&self, code: &str) -> Result<User, AuthError> {
        self.provider.exchange_code(code).await
    }
}
