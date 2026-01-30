pub mod scopes {
    pub const READ: &str = "read";
    pub const WRITE: &str = "write";
    pub const ADMIN: &str = "admin";

    pub const ASSET_UPLOAD: &str = "asset:upload";
    pub const ASSET_DOWNLOAD: &str = "asset:download";
    pub const MANIFEST_PUBLISH: &str = "manifest:publish";
    pub const MANIFEST_READ: &str = "manifest:download";
    pub const JOB_RUN: &str = "job:run";
    pub const JOB_ATTACH: &str = "job:attach";
}

pub mod routes {
    pub const HEALTH: &str = "/health";

    pub const AUTH_LOGIN: &str = "/auth/login";
    pub const AUTH_TOKEN: &str = "/auth/token";
    pub const AUTH_CALLBACK: &str = "/auth/callback";

    pub const ASSETS: &str = "/assets";
    pub const ASSETS_BY_HASH: &str = "/assets/{hash}";
    pub const ASSETS_STREAM_BY_HASH: &str = "/assets/stream/{hash}";

    pub const MANIFEST: &str = "/manifest";
    pub const MANIFEST_BY_VERSION: &str = "/manifest/{version}";

    pub const JOBS_RUN: &str = "/jobs/run";
    pub const JOBS_ATTACH: &str = "/jobs/{id}/attach";
}
