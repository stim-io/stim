use std::fmt::Write as _;

use crate::{
    cli::OutputFormat,
    model::{Report, Severity},
};

pub(crate) fn print_report(report: &Report, format: OutputFormat) -> Result<(), String> {
    match format {
        OutputFormat::Text => {
            print_text_report(report);
            Ok(())
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(report)
                .map_err(|error| format!("failed to serialize guard report: {error}"))?;
            println!("{json}");
            Ok(())
        }
    }
}

fn print_text_report(report: &Report) {
    print!("{}", text_report(report));
}

pub(crate) fn text_report(report: &Report) -> String {
    let deny_count = report.deny_count();
    let warning_count = report.warning_count();

    if report.issues.is_empty() {
        return "stim-guard passed: no issues\n".to_string();
    }

    let mut text =
        format!("stim-guard found {deny_count} deny issue(s) and {warning_count} warning(s)\n");

    if !report.guidance.is_empty() {
        text.push_str("\nguidance:\n");
        for guide in &report.guidance {
            writeln!(&mut text, "- {}", guide.rule).expect("write string");
            writeln!(&mut text, "  bad flavor: {}", guide.bad_flavor).expect("write string");
            writeln!(&mut text, "  action hint: {}", guide.action_hint).expect("write string");
        }
    }

    text.push_str("\nissues:\n");
    for issue in &report.issues {
        let location = match issue.line {
            Some(line) => format!("{}:{line}", issue.path),
            None => issue.path.clone(),
        };
        writeln!(
            &mut text,
            "{} {} {location} - {}",
            severity_label(issue.severity),
            issue.rule,
            issue.message
        )
        .expect("write string");
    }

    text
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Deny => "deny",
        Severity::Warning => "warning",
    }
}
