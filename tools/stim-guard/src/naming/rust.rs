use syn::{
    visit::{self, Visit},
    ImplItemFn, ItemFn, PatIdent, TraitItemFn,
};

use crate::{model::Issue, naming::check_name};

pub(crate) fn check_rust_names(path: &str, source: &str, issues: &mut Vec<Issue>) {
    match syn::parse_file(source) {
        Ok(file) => {
            let mut visitor = RustNameVisitor { path, issues };
            visitor.visit_file(&file);
        }
        Err(error) => issues.push(crate::model::issue(
            crate::model::Severity::Deny,
            "source-parse-error",
            path,
            Some(error.span().start().line),
            format!("failed to parse Rust source: {error}"),
        )),
    }
}

struct RustNameVisitor<'a> {
    path: &'a str,
    issues: &'a mut Vec<Issue>,
}

impl<'ast> Visit<'ast> for RustNameVisitor<'_> {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        check_name(
            self.issues,
            self.path,
            node.sig.ident.span().start().line,
            "function",
            &node.sig.ident.to_string(),
        );
        visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        check_name(
            self.issues,
            self.path,
            node.sig.ident.span().start().line,
            "method",
            &node.sig.ident.to_string(),
        );
        visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast TraitItemFn) {
        check_name(
            self.issues,
            self.path,
            node.sig.ident.span().start().line,
            "method",
            &node.sig.ident.to_string(),
        );
        visit::visit_trait_item_fn(self, node);
    }

    fn visit_pat_ident(&mut self, node: &'ast PatIdent) {
        let name = node.ident.to_string();
        if name != "self" {
            check_name(
                self.issues,
                self.path,
                node.ident.span().start().line,
                "binding",
                &name,
            );
        }
        visit::visit_pat_ident(self, node);
    }
}
