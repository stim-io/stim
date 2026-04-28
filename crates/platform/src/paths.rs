use std::path::PathBuf;

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("failed to resolve stim workspace root")
}

pub fn dev_root() -> PathBuf {
    workspace_root().join(".tmp/dev")
}
