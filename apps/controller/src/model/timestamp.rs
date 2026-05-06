pub(crate) fn timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}-{:03}", now.as_secs(), now.subsec_millis())
}
