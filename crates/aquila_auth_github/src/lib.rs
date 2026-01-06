//! # Aquila GitHub Auth
//! [![Crates.io](https://img.shields.io/crates/v/aquila_auth_github.svg)](https://crates.io/crates/aquila_auth_github)
//! [![Downloads](https://img.shields.io/crates/d/aquila_auth_github.svg)](https://crates.io/crates/aquila_auth_github)
//! [![Docs](https://docs.rs/aquila_auth_github/badge.svg)](https://docs.rs/aquila_auth_github/)
//!
//! OAuth2 authentication provider using GitHub.
//!
//! Allows users to log in using their GitHub accounts.
//! Supports restricting access to members of a specific GitHub Organization.
//!
//! ## Usage
//!
//! ```no_run
//! # use aquila_auth_github::{GithubAuthProvider, GithubConfig};
//! let config = GithubConfig {
//!     client_id: "Iv1...".to_string(),
//!     client_secret: "secret".to_string(),
//!     redirect_uri: "http://localhost:3000/auth/callback".to_string(),
//!     required_org: Some("MyGameStudio".to_string()),
//! };
//!
//! let auth = GithubAuthProvider::new(Some(config));
//! ```

use aquila_core::prelude::*;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Deserialize, Debug, Clone)]
struct GithubUser {
    login: String,
}

struct CachedUser {
    user: User,
    expires_at: Instant,
}

#[derive(Clone, Debug, Default)]
pub struct GithubConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub required_org: Option<String>,
}

#[derive(Clone)]
pub struct GithubAuthProvider {
    client: Client,
    config: Option<GithubConfig>,
    cache: Arc<Mutex<HashMap<String, CachedUser>>>,
}

impl GithubAuthProvider {
    pub fn new(config: Option<GithubConfig>) -> Self {
        let client = Client::builder()
            .user_agent("BevyAquila/0.1")
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            config,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn fetch_access_token(&self, code: &str) -> Result<String, AuthError> {
        let config = self
            .config
            .as_ref()
            .ok_or(AuthError::Generic("OAuth not configured".into()))?;

        let params = [
            ("client_id", &config.client_id),
            ("client_secret", &config.client_secret),
            ("code", &code.to_string()),
            ("redirect_uri", &config.redirect_uri),
        ];

        let res = self
            .client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| AuthError::Generic(format!("Network error: {}", e)))?;

        #[derive(Deserialize)]
        struct TokenRes {
            access_token: String,
        }

        let token_res: TokenRes = res
            .json()
            .await
            .map_err(|_| AuthError::Generic("Failed to parse GitHub token response".into()))?;

        Ok(token_res.access_token)
    }

    fn hash_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    }

    async fn fetch_user(&self, token: &str) -> Result<GithubUser, AuthError> {
        let res = self
            .client
            .get("https://api.github.com/user")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| AuthError::Generic(format!("GitHub API error: {}", e)))?;

        if res.status() == StatusCode::UNAUTHORIZED {
            return Err(AuthError::InvalidToken);
        }

        if !res.status().is_success() {
            return Err(AuthError::Generic(format!(
                "GitHub returned {}",
                res.status()
            )));
        }

        res.json::<GithubUser>()
            .await
            .map_err(|_| AuthError::Generic("Failed to parse GitHub response".into()))
    }

    async fn check_org_membership(
        &self,
        token: &str,
        username: &str,
        org: &str,
    ) -> Result<(), AuthError> {
        let url = format!("https://api.github.com/orgs/{}/members/{}", org, username);
        let res = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| AuthError::Generic(format!("Membership check failed: {}", e)))?;

        if res.status() == StatusCode::NO_CONTENT {
            Ok(())
        } else {
            Err(AuthError::Forbidden(format!(
                "User {} is not a member of {}",
                username, org
            )))
        }
    }
}

impl AuthProvider for GithubAuthProvider {
    async fn verify(&self, token: &str) -> Result<User, AuthError> {
        let token_hash = self.hash_token(token);

        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.get(&token_hash) {
                if Instant::now() < entry.expires_at {
                    return Ok(entry.user.clone());
                } else {
                    cache.remove(&token_hash);
                }
            }
        }

        let gh_user = self.fetch_user(token).await?;

        if let Some(cfg) = &self.config
            && let Some(org) = &cfg.required_org
        {
            self.check_org_membership(token, &gh_user.login, org)
                .await?;
        }

        let user = User {
            id: gh_user.login,
            scopes: vec!["read".to_string(), "write".to_string()],
        };

        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(
                token_hash,
                CachedUser {
                    user: user.clone(),
                    expires_at: Instant::now() + Duration::from_secs(300),
                },
            );
        }

        Ok(user)
    }

    fn get_login_url(&self) -> Option<String> {
        self.config.as_ref().map(|c| {
            format!(
                "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=read:user,read:org",
                c.client_id, c.redirect_uri
            )
        })
    }

    async fn exchange_code(&self, code: &str) -> Result<User, AuthError> {
        let token = self.fetch_access_token(code).await?;

        self.verify(&token).await
    }
}
