use std::path::PathBuf;

use crate::{
    model::{issue, Report, Severity},
    output::text_report,
};

#[test]
fn text_groups_guidance() {
    let report = Report::new(
        PathBuf::from("root"),
        vec![issue(
            Severity::Deny,
            "name-too-many-words",
            "sample.rs",
            Some(1),
            "long name",
        )],
    );
    let text = text_report(&report);

    assert!(text.contains("guidance:\n- name-too-many-words"));
    assert!(text.contains("bad flavor:"));
    assert!(text.contains("action hint:"));
    assert!(text.contains("\nissues:\n"));
}
