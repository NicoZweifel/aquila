use aquila::prelude::*;
use std::env;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Config
    let jwt_secret = env::var("AQUILA_JWT_SECRET").expect("JWT Secret required");
    let required_org = env::var("AQUILA_GITHUB_ORG").ok();

    // Must match the callback route in the GitHub app and the server config callback, see below.
    let redirect_uri = "http://localhost:3000/auth/callback".to_string();
    let gh_cfg = env::var("GITHUB_CLIENT_ID")
        .and_then(|client_id| {
            env::var("GITHUB_CLIENT_SECRET").map(|client_secret| GithubConfig {
                redirect_uri,
                client_id,
                client_secret,
                required_org,
            })
        })
        .ok();

    // Providers
    let storage = FileSystemStorage::new("./aquila_data");
    let github_auth = GithubAuthProvider::new(gh_cfg);
    let jwt_service = JwtService::new(&jwt_secret);
    let auth = JWTServiceAuthProvider::new(jwt_service, github_auth);

    // Build
    let app = AquilaServer::new(AquilaSeverConfig {
        jwt_secret,
        // this is the default but just to be explicit, see above.
        callback: "/auth/callback".to_string(),
    })
    .build(storage, auth);

    // Serve
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    println!("Server listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
