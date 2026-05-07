pub(crate) mod assertions;
pub(crate) mod renderer;

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
