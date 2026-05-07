use std::path::PathBuf;

use crate::model::{issue, Report, Severity};

#[test]
fn report_guides_rules() {
    let issues = vec![
        issue(
            Severity::Deny,
            "name-too-many-words",
            "sample.rs",
            Some(1),
            "long name",
        ),
        issue(
            Severity::Deny,
            "name-too-many-words",
            "sample.rs",
            Some(2),
            "another long name",
        ),
    ];
    let report = Report::new(PathBuf::from("root"), issues);

    assert_eq!(report.guidance.len(), 1);
    assert_eq!(report.guidance[0].rule, "name-too-many-words");
    assert!(report.guidance[0].bad_flavor.contains("scenario"));
    assert!(report.guidance[0].action_hint.contains("namespace"));
}
