use std::path::Path;

use swc_common::{sync::Lrc, FileName, SourceMap, Span};
use swc_ecma_ast::{
    ClassMethod, FnDecl, FnExpr, Ident, MethodProp, Param, Pat, PropName, VarDeclarator,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};

use crate::{model::Issue, naming::check_name};

pub(crate) fn check_ts_names(path: &str, source: &str, issues: &mut Vec<Issue>) {
    let extension = Path::new(path).extension().and_then(|value| value.to_str());
    let scripts = if extension == Some("vue") {
        extract_vue_scripts(source)
    } else {
        vec![ScriptBlock {
            content: source.to_string(),
            start_line: 0,
        }]
    };

    for script in scripts {
        check_script(path, extension == Some("tsx"), script, issues);
    }
}

struct ScriptBlock {
    content: String,
    start_line: usize,
}

fn check_script(path: &str, tsx: bool, script: ScriptBlock, issues: &mut Vec<Issue>) {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom(path.to_string()).into(), script.content);
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx,
            decorators: true,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let module = match parser.parse_module() {
        Ok(module) => module,
        Err(error) => {
            issues.push(crate::model::issue(
                crate::model::Severity::Deny,
                "source-parse-error",
                path,
                None,
                format!("failed to parse TypeScript source: {error:?}"),
            ));
            return;
        }
    };

    for error in parser.take_errors() {
        issues.push(crate::model::issue(
            crate::model::Severity::Deny,
            "source-parse-error",
            path,
            None,
            format!("failed to parse TypeScript source: {error:?}"),
        ));
    }

    let mut visitor = TsNameVisitor {
        path,
        issues,
        cm,
        line_offset: script.start_line,
    };
    module.visit_with(&mut visitor);
}

struct TsNameVisitor<'a> {
    path: &'a str,
    issues: &'a mut Vec<Issue>,
    cm: Lrc<SourceMap>,
    line_offset: usize,
}

impl Visit for TsNameVisitor<'_> {
    fn visit_fn_decl(&mut self, node: &FnDecl) {
        self.check_ident("function", &node.ident);
        node.visit_children_with(self);
    }

    fn visit_fn_expr(&mut self, node: &FnExpr) {
        if let Some(ident) = &node.ident {
            self.check_ident("function", ident);
        }
        node.visit_children_with(self);
    }

    fn visit_var_declarator(&mut self, node: &VarDeclarator) {
        self.check_pat("binding", &node.name);
        node.visit_children_with(self);
    }

    fn visit_param(&mut self, node: &Param) {
        self.check_pat("parameter", &node.pat);
        node.visit_children_with(self);
    }

    fn visit_class_method(&mut self, node: &ClassMethod) {
        self.check_prop_name("method", &node.key);
        node.visit_children_with(self);
    }

    fn visit_method_prop(&mut self, node: &MethodProp) {
        self.check_prop_name("method", &node.key);
        node.visit_children_with(self);
    }
}

impl TsNameVisitor<'_> {
    fn check_pat(&mut self, kind: &str, pat: &Pat) {
        match pat {
            Pat::Ident(binding) => self.check_ident(kind, &binding.id),
            Pat::Array(array) => {
                for elem in array.elems.iter().flatten() {
                    self.check_pat(kind, elem);
                }
            }
            Pat::Rest(rest) => self.check_pat(kind, &rest.arg),
            Pat::Object(object) => {
                for prop in &object.props {
                    prop.visit_with(self);
                }
            }
            Pat::Assign(assign) => self.check_pat(kind, &assign.left),
            Pat::Invalid(_) | Pat::Expr(_) => {}
        }
    }

    fn check_ident(&mut self, kind: &str, ident: &Ident) {
        check_name(
            self.issues,
            self.path,
            self.line_for(ident.span),
            kind,
            ident.sym.as_ref(),
        );
    }

    fn check_prop_name(&mut self, kind: &str, prop: &PropName) {
        if let PropName::Ident(ident) = prop {
            check_name(
                self.issues,
                self.path,
                self.line_for(ident.span),
                kind,
                ident.sym.as_ref(),
            );
        }
    }

    fn line_for(&self, span: Span) -> usize {
        self.cm.lookup_char_pos(span.lo()).line + self.line_offset
    }
}

fn extract_vue_scripts(source: &str) -> Vec<ScriptBlock> {
    let mut scripts = Vec::new();
    let mut search_start = 0;

    while let Some(open_offset) = source[search_start..].find("<script") {
        let open_start = search_start + open_offset;
        let Some(open_end_offset) = source[open_start..].find('>') else {
            break;
        };
        let content_start = open_start + open_end_offset + 1;
        let Some(close_offset) = source[content_start..].find("</script>") else {
            break;
        };
        let content_end = content_start + close_offset;
        scripts.push(ScriptBlock {
            content: source[content_start..content_end].to_string(),
            start_line: source[..content_start].lines().count().saturating_sub(1),
        });
        search_start = content_end + "</script>".len();
    }

    scripts
}
