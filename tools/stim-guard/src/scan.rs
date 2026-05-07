use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use walkdir::{DirEntry, WalkDir};

use crate::{
    config::{
        source_file_kind, GuardConfig, SourceKind, MAX_DIR_CHILDREN, MAX_SOURCE_LINES,
        WARN_SOURCE_DEPTH,
    },
    model::{issue, Issue, Severity},
    naming::{check_rust_names, check_ts_names},
    path_match::relative_path,
    rust_tests::check_rust_test_home,
};

pub(crate) fn run_checks(config: &GuardConfig) -> Result<Vec<Issue>, String> {
    let root = canonical_root(&config.root)?;
    let mut issues = Vec::new();
    let mut child_counts = BTreeMap::<PathBuf, usize>::new();

    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_entry(|entry| should_enter(config, &root, entry))
    {
        let entry = entry.map_err(|error| format!("failed to walk source tree: {error}"))?;
        let path = entry.path();
        let relative = relative_path(&root, path)?;

        if config.is_excluded(&relative) {
            continue;
        }

        if entry.file_type().is_dir() {
            if config.is_included(&relative) {
                check_source_depth(&relative, &mut issues);
                add_child_count(&mut child_counts, &relative);
            }
            continue;
        }

        let Some(kind) = source_file_kind(&relative) else {
            continue;
        };
        if !config.is_included(&relative) {
            continue;
        }

        add_child_count(&mut child_counts, &relative);
        check_source_file(&relative, path, kind, &mut issues)?;
    }

    for (dir, count) in child_counts {
        if count > MAX_DIR_CHILDREN {
            issues.push(issue(
                Severity::Deny,
                "directory-too-many-children",
                path_string(&dir),
                None,
                format!("directory has {count} source children; max is {MAX_DIR_CHILDREN}"),
            ));
        }
    }

    issues.sort_by(|left, right| {
        (left.path.as_str(), left.line.unwrap_or(0), left.rule).cmp(&(
            right.path.as_str(),
            right.line.unwrap_or(0),
            right.rule,
        ))
    });
    Ok(issues)
}

fn canonical_root(root: &Path) -> Result<PathBuf, String> {
    fs::canonicalize(root)
        .map_err(|error| format!("failed to resolve root {}: {error}", root.display()))
}

fn should_enter(config: &GuardConfig, root: &Path, entry: &DirEntry) -> bool {
    let Ok(relative) = relative_path(root, entry.path()) else {
        return true;
    };
    !config.is_excluded(&relative)
}

fn check_source_depth(relative: &Path, issues: &mut Vec<Issue>) {
    let Some(depth) = source_depth(relative) else {
        return;
    };
    if depth <= WARN_SOURCE_DEPTH {
        return;
    }

    issues.push(issue(
        Severity::Warning,
        "source-directory-too-deep",
        path_string(relative),
        None,
        format!("source directory depth is {depth}; warning threshold is {WARN_SOURCE_DEPTH}"),
    ));
}

fn source_depth(relative: &Path) -> Option<usize> {
    let mut depth = None;
    for component in relative.components() {
        if let Some(depth) = depth.as_mut() {
            *depth += 1;
            continue;
        }
        if component.as_os_str() == "src" {
            depth = Some(0);
        }
    }
    depth
}

fn add_child_count(child_counts: &mut BTreeMap<PathBuf, usize>, relative: &Path) {
    let Some(parent) = relative.parent() else {
        return;
    };
    *child_counts.entry(parent.to_path_buf()).or_default() += 1;
}

fn check_source_file(
    relative: &Path,
    path: &Path,
    kind: SourceKind,
    issues: &mut Vec<Issue>,
) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let line_count = source.lines().count();
    let relative_path = path_string(relative);

    if line_count > MAX_SOURCE_LINES {
        issues.push(issue(
            Severity::Deny,
            "source-file-too-long",
            relative_path.clone(),
            None,
            format!("source file has {line_count} lines; max is {MAX_SOURCE_LINES}"),
        ));
    }

    match kind {
        SourceKind::Rust => {
            check_rust_names(&relative_path, &source, issues);
            check_rust_test_home(relative, &source, issues);
        }
        SourceKind::TypeScript => check_ts_names(&relative_path, &source, issues),
    }

    Ok(())
}

pub(crate) fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
