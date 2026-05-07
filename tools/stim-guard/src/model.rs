use std::{collections::BTreeSet, path::PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Severity {
    Deny,
    Warning,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub(crate) struct Issue {
    pub(crate) severity: Severity,
    pub(crate) rule: &'static str,
    pub(crate) path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) line: Option<usize>,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
pub(crate) struct RuleGuide {
    pub(crate) rule: &'static str,
    pub(crate) bad_flavor: &'static str,
    pub(crate) action_hint: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct Report {
    pub(crate) root: String,
    pub(crate) guidance: Vec<RuleGuide>,
    pub(crate) issues: Vec<Issue>,
}

impl Report {
    pub(crate) fn new(root: PathBuf, issues: Vec<Issue>) -> Self {
        let guidance = guides_for(&issues);
        Self {
            root: root.display().to_string(),
            guidance,
            issues,
        }
    }

    pub(crate) fn deny_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == Severity::Deny)
            .count()
    }

    pub(crate) fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == Severity::Warning)
            .count()
    }
}

fn guides_for(issues: &[Issue]) -> Vec<RuleGuide> {
    issues
        .iter()
        .map(|issue| issue.rule)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter_map(rule_guide)
        .collect()
}

fn rule_guide(rule: &str) -> Option<RuleGuide> {
    match rule {
        "name-too-many-words" => Some(RuleGuide {
            rule: "name-too-many-words",
            bad_flavor: "Names may be carrying scenario, path, or assertion context that belongs near an owner boundary.",
            action_hint: "Consider lifting repeated context into a namespace, object, class, module, impl block, or test module before shortening names.",
        }),
        "directory-too-many-children" => Some(RuleGuide {
            rule: "directory-too-many-children",
            bad_flavor: "The directory may be acting as a mixed ownership shelf instead of a clear boundary.",
            action_hint: "Look for real owner or runtime-boundary groups before adding utility buckets or thin routing folders.",
        }),
        "source-file-too-long" => Some(RuleGuide {
            rule: "source-file-too-long",
            bad_flavor: "The file may be carrying multiple concepts, fixture weight, flow stages, or view/model pressure.",
            action_hint: "Look for concept, fixture, flow, model, or view boundaries; avoid mechanical line-count cuts.",
        }),
        "source-directory-too-deep" => Some(RuleGuide {
            rule: "source-directory-too-deep",
            bad_flavor: "Path depth may be explaining ownership that belongs at module or package level.",
            action_hint: "Use this as boundary-review pressure; module/package changes should wait until ownership is stable.",
        }),
        "rust-test-in-src" => Some(RuleGuide {
            rule: "rust-test-in-src",
            bad_flavor: "Production source may be carrying test-only modules, fixtures, or private-shape pressure.",
            action_hint: "Consider moving test cases into sibling tests paths and exposing only intentional test seams.",
        }),
        "source-parse-error" => Some(RuleGuide {
            rule: "source-parse-error",
            bad_flavor: "The source could not be parsed, so AST checks cannot be trusted.",
            action_hint: "Check syntax or parser coverage before treating downstream style results as complete.",
        }),
        _ => None,
    }
}

pub(crate) fn issue(
    severity: Severity,
    rule: &'static str,
    path: impl Into<String>,
    line: Option<usize>,
    message: impl Into<String>,
) -> Issue {
    Issue {
        severity,
        rule,
        path: path.into(),
        line,
        message: message.into(),
    }
}
