#![allow(dead_code, private_interfaces, unused_imports)]

#[path = "../src/acceptance/mod.rs"]
mod acceptance;
#[path = "../src/chat/mod.rs"]
mod chat;
#[path = "../src/cli.rs"]
mod cli;
#[path = "../src/control/mod.rs"]
mod control;
#[path = "../src/detect/mod.rs"]
mod detect;
#[path = "../src/dev_loop.rs"]
mod dev_loop;
#[path = "../src/shared/mod.rs"]
mod shared;
#[path = "../src/sidecars/mod.rs"]
mod sidecars;
#[path = "../src/smoke/mod.rs"]
mod smoke;

#[path = "unit/acceptance.rs"]
mod acceptance_cases;
#[path = "unit/chat.rs"]
mod chat_cases;
#[path = "unit/cli.rs"]
mod cli_cases;
#[path = "unit/control.rs"]
mod control_cases;
#[path = "unit/detect.rs"]
mod detect_cases;
#[path = "unit/shared.rs"]
mod shared_cases;
#[path = "unit/smoke.rs"]
mod smoke_cases;
