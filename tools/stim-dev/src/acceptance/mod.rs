mod controller;

pub(crate) fn accept(args: Vec<String>) -> Result<(), String> {
    match args.as_slice() {
        [target, leaf] if target == "controller" && leaf == "messaging" => {
            controller::accept_messaging(None)
        }
        [target, leaf, text @ ..] if target == "controller" && leaf == "messaging" => {
            controller::accept_messaging(Some(text.join(" ")))
        }
        [target, leaf] if target == "controller" && leaf == "tool-activity" => {
            controller::accept_tool_activity(None)
        }
        [target, leaf, text @ ..] if target == "controller" && leaf == "tool-activity" => {
            controller::accept_tool_activity(Some(text.join(" ")))
        }
        [] | [_] => Err(
            "accept requires '<target> <leaf>'; supported leaves: controller messaging [text], controller tool-activity [text]"
                .into(),
        ),
        [target, ..] => Err(format!(
            "unsupported accept leaf under target '{target}'; supported leaves: controller messaging [text], controller tool-activity [text]"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::accept;

    #[test]
    fn accept_rejects_unknown_or_incomplete_leaves() {
        assert!(accept(Vec::new()).unwrap_err().contains("accept requires"));
        assert!(accept(vec!["controller".into()])
            .unwrap_err()
            .contains("accept requires"));
        assert!(accept(vec!["renderer".into(), "messaging".into()])
            .unwrap_err()
            .contains("unsupported accept leaf"));
    }
}
