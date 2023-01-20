use rocket::get;
use rocket::serde::json::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct VersionInformation {
    /// The version of the backend which is currently running.
    pub backend_version: &'static str,
    /// The architecture the backend was build for.
    pub backend_arch: &'static str,
    /// The version of the rustc compiler used to compile the backend.
    pub rustc_version: &'static str,
    /// The date on which the backend was build.
    pub build_date: &'static str,
    /// The time on which the backend was build.
    pub build_time: &'static str,
}

#[get("/version")]
pub async fn get_backend_version() -> Json<VersionInformation> {
    Json(VersionInformation {
        backend_version: env!("VERGEN_GIT_SEMVER"),
        backend_arch: env!("VERGEN_CARGO_TARGET_TRIPLE"),
        rustc_version: env!("VERGEN_RUSTC_SEMVER"),
        build_date: env!("VERGEN_BUILD_DATE"),
        build_time: env!("VERGEN_BUILD_TIME"),
    })
}
