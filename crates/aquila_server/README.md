## Aquila Server
[![Crates.io](https://img.shields.io/crates/v/aquila_server.svg)](https://crates.io/crates/aquila_server)
[![Downloads](https://img.shields.io/crates/d/aquila_server.svg)](https://crates.io/crates/bevy_aquila)
[![Docs](https://docs.rs/aquila_server/badge.svg)](https://docs.rs/aquila_server/)

A modular, Axum-based asset server implementation.

Provides the [`AquilaServer`] builder, which ties together a storage backend and an authentication provider
to serve assets.

### Permissions

Enforces a scoped permission system. Authentication providers must grant
the following scopes in their `User` object:

* **`read`**: to download assets, fetch manifests.
* **`write`**: to upload assets, publish manifests.
* **`admin`**: Full access. (Note: Admin tokens cannot be minted via the API and only Admins can mint tokens).

### Example

```rust
use aquila_server::prelude::*;
use aquila_fs::FileSystemStorage;
use aquila_auth_mock::AllowAllAuth;

let storage = FileSystemStorage::new("./assets");
let auth = AllowAllAuth;

let app = AquilaServer::default().build(storage, auth);
```

License: MIT OR Apache-2.0
