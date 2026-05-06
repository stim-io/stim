use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn timestamp_now() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch");

    format!("{}-{:03}", duration.as_secs(), duration.subsec_millis())
}

pub(crate) fn create_request_id() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch");

    format!(
        "{}-{}-{}",
        duration.as_secs(),
        duration.subsec_nanos(),
        std::process::id()
    )
}
