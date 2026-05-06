mod assertions;
mod renderer;

pub(crate) fn smoke(args: Vec<String>) -> Result<(), String> {
    match args.as_slice() {
        [target, leaf] if target == "renderer" && leaf == "continuation" => {
            renderer::smoke_continuation(None)
        }
        [target, leaf, text @ ..] if target == "renderer" && leaf == "continuation" => {
            renderer::smoke_continuation(Some(text.join(" ")))
        }
        [target, leaf] if target == "renderer" && leaf == "messaging" => {
            renderer::smoke_messaging(None)
        }
        [target, leaf, text @ ..] if target == "renderer" && leaf == "messaging" => {
            renderer::smoke_messaging(Some(text.join(" ")))
        }
        [] | [_] => Err("smoke requires '<target> <leaf>'; supported leaves: renderer messaging [text], renderer continuation [text]".into()),
        [target, ..] => Err(format!(
            "unsupported smoke leaf under target '{target}'; supported leaves: renderer messaging [text], renderer continuation [text]"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::smoke;

    #[test]
    fn smoke_rejects_unknown_or_incomplete_leaves() {
        assert!(smoke(Vec::new()).unwrap_err().contains("smoke requires"));
        assert!(smoke(vec!["renderer".into()])
            .unwrap_err()
            .contains("smoke requires"));
        assert!(smoke(vec!["tauri".into(), "messaging".into()])
            .unwrap_err()
            .contains("unsupported smoke leaf"));
    }
}
