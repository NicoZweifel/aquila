//! # Aquila Auth Mock
//! [![Crates.io](https://img.shields.io/crates/v/aquila_auth_mock.svg)](https://crates.io/crates/aquila_auth_mock)
//! [![Downloads](https://img.shields.io/crates/d/aquila_auth_mock.svg)](https://crates.io/crates/aquila_auth_mock)
//! [![Docs](https://docs.rs/aquila_auth_mock/badge.svg)](https://docs.rs/aquila_auth_mock/)
//!
//! A dummy authentication provider for development and testing.
//!
//! **WARNING**: This provider allows ANY token to pass as a valid user with full admin permissions.
//!
//! **DO NOT use this in production!!!**
//!
//! ## Usage
//!
//! ```rust
//! # use aquila_auth_mock::AllowAllAuth;
//! # fn main() {
//! let auth = AllowAllAuth;
//! # }
//! ```

use aquila_core::prelude::*;

#[derive(Clone)]
pub struct AllowAllAuth;

impl AuthProvider for AllowAllAuth {
    async fn verify(&self, _token: &str) -> Result<User, AuthError> {
        Ok(User {
            id: "dev_user".to_string(),
            scopes: vec!["admin".to_string(), "read".to_string(), "write".to_string()],
        })
    }
}
