# 🦅 Aquila
[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/NicoZweifel/aquila?tab=readme-ov-file#license)
[![Crates.io](https://img.shields.io/crates/v/aquila.svg)](https://crates.io/crates/aquila)
[![Downloads](https://img.shields.io/crates/d/aquila.svg)](https://crates.io/crates/aquila)
[![Docs](https://docs.rs/aquila/badge.svg)](https://docs.rs/aquila/)

> *Your personal flying courier*

A modular asset server with support for OAuth and multiple file backends, meant for serving game assets but could probably be used for other things too.

I'll write more here soon!

> [!CAUTION]
> This package is in early development!

## What is this for?

During game development a way to serve assets remotely is often desired.
This can be either to fetch at build-time in a build environment or to serve them to your users at runtime,
leading to complex setups involving git LFS or Perforce and build servers or worse - manual swapping of files.

This crate aims at simplifying this process by providing a simple server, a client and a cli that can be used to serve versioned assets. 
At the moment, it supports:

- Serve assets to your game clients
- Publish assets and manifests to a server
- Minting (read-only public) tokens
- Authenticate users (custom or OAuth, see [`aquila_auth_mock`](/crates/aquila_auth_mock) and [`aquila_auth_github`](/crates/aquila_auth_github)) 

## Security Notice

This crate is in early development and should not be used in production yet. You are responsible for making sure your assets are safe and secure.
If you ship public read-only tokens to users, make sure you are aware of what that entails, e.g., how to invalidate and ship new ones in the case of abuse.

> [!IMPORTANT]
> Make sure you vet any auth providers and OAuth applications and its permissions that you intend to use thoroughly before using them in production.

## Feature Flags

| Feature | Description |
|---------|-------------|
| **`server`** | Includes the Axum-based server implementation (`aquila_server`). |
| **`client`** | Includes the HTTP client (`aquila_client`) for tooling. |
| **`fs`** | Storage backend for the local filesystem (`aquila_fs`). |
| **`s3`** | Storage backend for AWS S3 (`aquila_s3`). |
| **`opendal`** | Storage backend for OpenDAL (`aquila_opendal`). |
| **`github_auth`** | GitHub OAuth2 provider (`aquila_auth_github`). |
| **`mock_auth`** | Development authentication provider (`aquila_auth_mock`). |

## Examples

### Simple server 

```sh
cargo run --example simple_server --features "server fs mock_auth"
```

### Simple client

Simple client (will publish v1.0 manifest and test.png)
```sh
cargo run --examples simple_client --features "client"
```

### Bevy

Bevy example (uses v1.0 manifest and test.png)

```shell
cargo run --example bevy
```

### Custom Server

```toml
[dependencies]
aquila = { version = "0.1", features = ["server", "fs", "mock_auth"] }
```

```rust
use aquila::prelude::*;

#[tokio::main]
async fn main() {
    let storage = FileSystemStorage::new("./assets");
     let auth = AllowAllAuth;

    // Build
    let app = AquilaServer::default().build(storage, auth);
    
    // Serve
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

The rest of the examples use the [CLI](/crates/aquila_cli)

> [!TIP]
> While not required, it's recommended to install the CLI to make usage easier.

#### Install cli
crates.io:
```bash
cargo install aquila_cli
```
From source: 
```bash
cargo install --path crates/aquila_cli
```

### AWS S3

You need to set the `AWS_REGION`, `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` env vars and/or use the AWS cli (`aws configure`).

Set the bucket name

```shell
set S3_BUCKET=...
```
Run the server
```shell
cargo run --example s3_server --features "server s3 mock_auth"
```
Publish v1.0 manifest and test.png
```shell
aquila publish --dir ./assets --version "v1.0"
```
Bevy example (uses v1.0 manifest and test.png)

```shell
cargo run --example bevy
```

### GitHub auth and JWT Minting (for read-only tokens)

Generate & set JWT secret:

You can use the CLI to generate a secret or provide your own:

```sh
aquila generate-secret   
set AQUILA_SECRET=...
```

Create a [GitHub OAuth app](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/creating-an-oauth-app)

The routes are configurable, you're going to have to make sure the callback route matches (in this case `/auth/callback`).

Set the client id and secret env vars

```shell
SET GITHUB_CLIENT_ID=...
SET GITHUB_CLIENT_SECRET=...
```

Run the server

```sh
cargo run --example github_auth_server --features "server fs github_auth"
 ```

You should now be able to log in using a second terminal:

```shell
aquila login      
```

Now set the token that you get after you've been redirected back to the callback route:

```shell
set AQUILA_TOKEN=...
```

You should have full access now! To mint a read-only public token:

```sh
aquila mint-token --subject "release-build-v1.0"  
```

To publish all assets and a manifest:

```shell
aquila publish --dir ./assets --version "v1.0"     
```
Bevy example (uses v1.0 manifest and test.png)

```shell
cargo run --example bevy
```

### CLI commands

single file test
```sh
aquila upload ./assets/test.png
```

publish manifest and assets
```sh
aquila publish --dir ./assets --version v1.0`
```

### Bevy

As shown above in the other examples, after publishing a manifest version, you can use the assets in bevy:

```sh
cargo run --example bevy
```

### Server

test manually

upload
```sh
curl -X POST --data-binary @./assets/test.png http://localhost:3000/assets
```

fetch
```sh
curl http://localhost:3000/assets/{hash} --output test_down.png
```

## Notes

Using generics to be able to use native async traits and avoiding dyn + `async_trait` or `Box` etc.
I'd be willing to revisit this though if there's a better alternative.

## TODO

- add some tests
- add some convenience features like `latest` etc.
- docker images, nix flakes (a simple server example should be enough)
- meta file support and other bevy asset reader functionality (folders) 
- readmes in crate folders
- multiple scopes, not just read/write/admin
- streaming large files (chunked encoding)
- I experimented with a VCSProvider trait to verify the version of the manifest against the VCS, 
but decided against it for now, but it definitely could be useful.

## License

Dual-licensed:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))
