use std::{fs, path::PathBuf};

use crate::{config::GuardConfig, path_match::PathPattern, scan::run_checks};

#[test]
fn scans_structure_limits() {
    let root = test_root("structure");
    let source_dir = root.join("tools/demo/src");
    fs::create_dir_all(&source_dir).unwrap();
    for index in 0..11 {
        fs::write(source_dir.join(format!("file_{index}.rs")), "fn ok() {}\n").unwrap();
    }
    fs::write(source_dir.join("large.rs"), "fn ok() {}\n".repeat(501)).unwrap();

    let config = GuardConfig {
        root: root.clone(),
        include: vec![PathPattern::new("tools/*/src/**")],
        exclude: Vec::new(),
    };
    let issues = run_checks(&config).unwrap();

    assert!(issues
        .iter()
        .any(|issue| issue.rule == "source-file-too-long"));
    assert!(issues
        .iter()
        .any(|issue| issue.rule == "directory-too-many-children"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn warns_deep_source_dirs() {
    let root = test_root("depth");
    fs::create_dir_all(root.join("tools/demo/src/a/b/c/d/e")).unwrap();
    fs::write(
        root.join("tools/demo/src/a/b/c/d/e/file.rs"),
        "fn ok() {}\n",
    )
    .unwrap();

    let config = GuardConfig {
        root: root.clone(),
        include: vec![PathPattern::new("tools/*/src/**")],
        exclude: Vec::new(),
    };
    let issues = run_checks(&config).unwrap();

    assert!(issues
        .iter()
        .any(|issue| issue.rule == "source-directory-too-deep"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn counts_depth_after_src() {
    let root = test_root("src-depth");
    fs::create_dir_all(root.join("apps/renderer/vite/src/agents")).unwrap();
    fs::write(
        root.join("apps/renderer/vite/src/agents/model.ts"),
        "const ok = 1;\n",
    )
    .unwrap();

    let config = GuardConfig {
        root: root.clone(),
        include: vec![PathPattern::new("apps/renderer/vite/src/**")],
        exclude: Vec::new(),
    };
    let issues = run_checks(&config).unwrap();

    assert!(!issues
        .iter()
        .any(|issue| issue.rule == "source-directory-too-deep"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn denies_src_rust_tests() {
    let root = test_root("rust-tests");
    fs::create_dir_all(root.join("tools/demo/src")).unwrap();
    fs::write(
        root.join("tools/demo/src/lib.rs"),
        "#[cfg(test)] mod tests { #[test] fn sample() {} }\n",
    )
    .unwrap();

    let config = GuardConfig {
        root: root.clone(),
        include: vec![PathPattern::new("tools/*/src/**")],
        exclude: Vec::new(),
    };
    let issues = run_checks(&config).unwrap();

    assert!(issues.iter().any(|issue| issue.rule == "rust-test-in-src"));

    let _ = fs::remove_dir_all(root);
}

fn test_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("stim-guard-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}
