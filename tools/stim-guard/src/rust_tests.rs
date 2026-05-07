use std::path::Path;

use syn::{
    spanned::Spanned,
    visit::{self, Visit},
    Attribute, Meta,
};

use crate::{
    model::{issue, Issue, Severity},
    scan::path_string,
};

pub(crate) fn check_rust_test_home(relative: &Path, source: &str, issues: &mut Vec<Issue>) {
    if !has_src_component(relative) {
        return;
    }

    let Ok(file) = syn::parse_file(source) else {
        return;
    };

    let path = path_string(relative);
    let mut visitor = TestVisit {
        path: &path,
        issues,
    };
    visitor.visit_file(&file);
}

fn has_src_component(relative: &Path) -> bool {
    relative
        .components()
        .any(|component| component.as_os_str() == "src")
}

struct TestVisit<'a> {
    path: &'a str,
    issues: &'a mut Vec<Issue>,
}

impl<'ast> Visit<'ast> for TestVisit<'_> {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if is_test_attr(attr) {
            self.issues.push(issue(
                Severity::Deny,
                "rust-test-in-src",
                self.path,
                Some(attr.meta.path().span().start().line),
                "Rust test code belongs under a tests directory sibling to src",
            ));
        }

        visit::visit_attribute(self, attr);
    }
}

fn is_test_attr(attr: &Attribute) -> bool {
    let path = attr.meta.path();
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == "test")
        || is_cfg_test_attr(attr)
}

fn is_cfg_test_attr(attr: &Attribute) -> bool {
    if !attr.meta.path().is_ident("cfg") && !attr.meta.path().is_ident("cfg_attr") {
        return false;
    }

    let Meta::List(list) = &attr.meta else {
        return false;
    };

    list.tokens
        .to_string()
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .any(|word| word == "test")
}
