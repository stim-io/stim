use std::path::{Path, PathBuf};

use crate::path_match::PathPattern;

pub(crate) const MAX_NAME_WORDS: usize = 4;
pub(crate) const MAX_DIR_CHILDREN: usize = 10;
pub(crate) const MAX_SOURCE_LINES: usize = 500;
pub(crate) const WARN_SOURCE_DEPTH: usize = 4;

#[derive(Debug, Clone)]
pub(crate) struct GuardConfig {
    pub(crate) root: PathBuf,
    pub(crate) include: Vec<PathPattern>,
    pub(crate) exclude: Vec<PathPattern>,
}

impl GuardConfig {
    pub(crate) fn core(root: PathBuf) -> Self {
        Self {
            root,
            include: core_include_patterns(),
            exclude: core_exclude_patterns(),
        }
    }

    pub(crate) fn is_included(&self, relative: &Path) -> bool {
        self.include.iter().any(|pattern| pattern.matches(relative))
            && !self.exclude.iter().any(|pattern| pattern.matches(relative))
    }

    pub(crate) fn is_excluded(&self, relative: &Path) -> bool {
        self.exclude.iter().any(|pattern| pattern.matches(relative))
    }
}

fn core_include_patterns() -> Vec<PathPattern> {
    [
        "apps/*/src/**",
        "apps/*/tests/**",
        "apps/renderer/server/src/**",
        "apps/renderer/vite/src/**",
        "apps/tauri/src-tauri/src/**",
        "apps/tauri/src-tauri/tests/**",
        "crates/*/src/**",
        "crates/*/tests/**",
        "tools/*/src/**",
        "tools/*/tests/**",
    ]
    .into_iter()
    .map(PathPattern::new)
    .collect()
}

fn core_exclude_patterns() -> Vec<PathPattern> {
    [
        "**/node_modules/**",
        "**/target/**",
        "**/dist/**",
        "**/.vite/**",
        "**/.vite-temp/**",
        ".tmp/**",
        "apps/tauri/src-tauri/gen/**",
        "packages/client/src/gen/**",
    ]
    .into_iter()
    .map(PathPattern::new)
    .collect()
}

pub(crate) fn source_file_kind(path: &Path) -> Option<SourceKind> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") => Some(SourceKind::Rust),
        Some("ts" | "tsx" | "vue") => Some(SourceKind::TypeScript),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum SourceKind {
    Rust,
    TypeScript,
}
