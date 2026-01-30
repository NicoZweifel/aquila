use crate::api::ApiError;
use crate::jwt::JwtService;
use crate::state::AppState;

use aquila_core::prelude::*;
use axum::{extract::FromRequestParts, http::request::Parts};

/// A wrapper struct indicating a request has been authenticated.
#[derive(Clone, Debug)]
pub struct AuthenticatedUser(pub User);

impl<S> FromRequestParts<AppState<S>> for AuthenticatedUser
where
    S: AquilaServices,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState<S>,
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

        let auth = state.auth();
        let permissions = state.permissions();

        let user = auth.verify(token).await.map_err(ApiError::from)?;

        permissions
            .elevate(user)
            .await
            .map(AuthenticatedUser)
            .map_err(ApiError::from)
    }
}

/// Trait to define the required scope for an extractor.
pub trait ScopeRequirement: Send + Sync + 'static {
    const SCOPE: &'static str;
}

/// An extractor that requires the user to have a specific scope (or ADMIN).
/// Fails with 403 Forbidden if the scope is missing.
pub struct ScopedUser<T: ScopeRequirement> {
    pub user: User,
    _marker: std::marker::PhantomData<T>,
}

impl<T: ScopeRequirement> ScopedUser<T> {
    pub fn new(user: User) -> Self {
        Self {
            user,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T, S> FromRequestParts<AppState<S>> for ScopedUser<T>
where
    T: ScopeRequirement,
    S: AquilaServices,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState<S>,
    ) -> Result<Self, Self::Rejection> {
        let AuthenticatedUser(user) = AuthenticatedUser::from_request_parts(parts, state).await?;

        let required = T::SCOPE;
        if user
            .scopes
            .iter()
            .any(|s| s == scopes::ADMIN || s == required)
        {
            Ok(ScopedUser::new(user))
        } else {
            Err(ApiError::from(AuthError::Forbidden(format!(
                "Missing permission: '{}' scope required.",
                required
            ))))
        }
    }
}

pub struct WriteScope;
impl ScopeRequirement for WriteScope {
    const SCOPE: &'static str = scopes::WRITE;
}

pub struct AssetUpload;
impl ScopeRequirement for AssetUpload {
    const SCOPE: &'static str = scopes::ASSET_UPLOAD;
}

pub struct AssetDownload;
impl ScopeRequirement for AssetDownload {
    const SCOPE: &'static str = scopes::ASSET_DOWNLOAD;
}

pub struct ManifestPublish;
impl ScopeRequirement for ManifestPublish {
    const SCOPE: &'static str = scopes::MANIFEST_PUBLISH;
}

pub struct ManifestRead;
impl ScopeRequirement for ManifestRead {
    const SCOPE: &'static str = scopes::MANIFEST_READ;
}

pub struct JobRun;
impl ScopeRequirement for JobRun {
    const SCOPE: &'static str = scopes::JOB_RUN;
}

pub struct JobAttach;
impl ScopeRequirement for JobAttach {
    const SCOPE: &'static str = scopes::JOB_ATTACH;
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
        if token.is_empty() {
            return Err(AuthError::Missing);
        }

        match self.jwt_service.verify(token) {
            Ok(user) => Ok(user),
            Err(AuthError::Expired) => Err(AuthError::Expired),
            Err(_) => self.provider.verify(token).await,
        }
    }

    fn get_login_url(&self) -> Option<String> {
        self.provider.get_login_url()
    }

    async fn exchange_code(&self, code: &str) -> Result<User, AuthError> {
        self.provider.exchange_code(code).await
    }
}
