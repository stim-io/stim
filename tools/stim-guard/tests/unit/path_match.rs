use crate::path_match::PathPattern;

#[test]
fn star_matches_segment() {
    let pattern = PathPattern::new("apps/*/src/**");

    assert!(pattern.matches("apps/controller/src/main.rs".as_ref()));
    assert!(!pattern.matches("apps/renderer/vite/src/App.vue".as_ref()));
}

#[test]
fn globstar_matches_nested() {
    let pattern = PathPattern::new("apps/renderer/vite/src/**");

    assert!(pattern.matches("apps/renderer/vite/src/App.vue".as_ref()));
    assert!(pattern.matches("apps/renderer/vite/src/components/im/model.ts".as_ref()));
    assert!(!pattern.matches("apps/renderer/src/main.rs".as_ref()));
}
