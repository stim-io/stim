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

#[path = "service_cases/roundtrip.rs"]
mod service_cases;
