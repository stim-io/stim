use crate::naming::{check_rust_names, check_ts_names, count_name_words};

#[test]
fn counts_name_words() {
    assert_eq!(count_name_words("controller_operation_event"), 3);
    assert_eq!(count_name_words("controllerOperationEvent"), 3);
    assert_eq!(count_name_words("HTTPClient"), 2);
    assert_eq!(count_name_words("guard_sample_over_limit_name"), 5);
}

#[test]
fn rust_detects_long_names() {
    let mut issues = Vec::new();
    check_rust_names(
        "sample.rs",
        "fn guard_sample_over_limit_name() { let guard_sample_value_over_limit = 1; }",
        &mut issues,
    );

    assert_eq!(issues.len(), 2);
    assert!(issues[0].message.contains("guard_sample_over_limit_name"));
    assert!(issues[1].message.contains("guard_sample_value_over_limit"));
}

#[test]
fn ts_detects_long_names() {
    let mut issues = Vec::new();
    check_ts_names(
        "sample.ts",
        "function rendererOperationEventHandlerName(inputValue: string) { const controllerRuntimeResultValueText = inputValue; }",
        &mut issues,
    );

    assert_eq!(issues.len(), 2);
    assert!(issues[0]
        .message
        .contains("rendererOperationEventHandlerName"));
    assert!(issues[1]
        .message
        .contains("controllerRuntimeResultValueText"));
}

#[test]
fn vue_offsets_lines() {
    let mut issues = Vec::new();
    check_ts_names(
        "Sample.vue",
        "<template></template>\n<script setup lang=\"ts\">\nconst controllerRuntimeResultValueText = 1;\n</script>",
        &mut issues,
    );

    assert_eq!(issues[0].line, Some(3));
}
