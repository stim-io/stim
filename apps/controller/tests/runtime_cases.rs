#![allow(dead_code, unused_imports)]

#[path = "../src/client/mod.rs"]
mod client;
#[path = "../src/config.rs"]
mod config;
#[path = "../src/factory/mod.rs"]
mod factory;
#[path = "../src/fetch.rs"]
pub mod fetch;
#[path = "../src/handler/mod.rs"]
mod handler;
#[path = "../src/model/mod.rs"]
mod model;
#[path = "../src/runtime/mod.rs"]
mod runtime;
#[path = "../src/service/mod.rs"]
mod service;

#[path = "runtime_cases/fetch.rs"]
mod fetch_cases;
#[path = "runtime_cases/http.rs"]
mod http_cases;
#[path = "runtime_cases/sidecar.rs"]
mod sidecar_cases;
#[path = "runtime_cases/status.rs"]
mod status_cases;
#[path = "runtime_cases/support.rs"]
mod support;
#[path = "runtime_cases/ws.rs"]
mod ws_cases;
