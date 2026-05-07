pub(crate) mod controller;

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
        [target, leaf] if target == "controller" && leaf == "participant-routing" => {
            controller::accept_participant_routing(None)
        }
        [target, leaf, text @ ..] if target == "controller" && leaf == "participant-routing" => {
            controller::accept_participant_routing(Some(text.join(" ")))
        }
        [] | [_] => Err(
            "accept requires '<target> <leaf>'; supported leaves: controller messaging [text], controller tool-activity [text], controller participant-routing [text]"
                .into(),
        ),
        [target, ..] => Err(format!(
            "unsupported accept leaf under target '{target}'; supported leaves: controller messaging [text], controller tool-activity [text], controller participant-routing [text]"
        )),
    }
}
