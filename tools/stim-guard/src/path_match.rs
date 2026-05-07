use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct PathPattern {
    segments: Vec<Segment>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum Segment {
    AnyMany,
    AnyOne,
    Literal(String),
}

impl PathPattern {
    pub(crate) fn new(pattern: &str) -> Self {
        let segments = pattern
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(|segment| match segment {
                "**" => Segment::AnyMany,
                "*" => Segment::AnyOne,
                other => Segment::Literal(other.to_string()),
            })
            .collect();

        Self { segments }
    }

    pub(crate) fn matches(&self, path: &Path) -> bool {
        let path_segments = path_segments(path);
        matches_segments(&self.segments, &path_segments)
    }
}

pub(crate) fn relative_path(root: &Path, path: &Path) -> Result<PathBuf, String> {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .map_err(|error| format!("failed to strip root from {}: {error}", path.display()))
}

fn path_segments(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| component.as_os_str().to_str())
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

fn matches_segments(pattern: &[Segment], path: &[String]) -> bool {
    match (pattern.split_first(), path.split_first()) {
        (None, None) => true,
        (None, Some(_)) => false,
        (Some((Segment::AnyMany, rest)), _) => {
            matches_segments(rest, path)
                || path
                    .split_first()
                    .is_some_and(|(_, path_rest)| matches_segments(pattern, path_rest))
        }
        (Some((Segment::AnyOne, rest)), Some((_, path_rest))) => matches_segments(rest, path_rest),
        (Some((Segment::Literal(expected), rest)), Some((actual, path_rest))) => {
            expected == actual && matches_segments(rest, path_rest)
        }
        (Some(_), None) => false,
    }
}
