use std::{env, thread, time::Duration};

use stim_sidecar::{ready::SidecarReadyLine, stamp::read_stamp};

fn main() {
    if let Err(error) = run() {
        eprintln!("stim-controller: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();

    match args.first().map(String::as_str) {
        Some("serve") => {
            args.remove(0);
            serve(args)
        }
        Some("proof") | None => proof(),
        Some("--help") | Some("-h") | Some("help") => {
            print_help();
            Ok(())
        }
        Some(other) => Err(format!("unsupported command: {other}")),
    }
}

fn serve(args: Vec<String>) -> Result<(), String> {
    let stamp = read_stamp(&args).map_err(|error| format!("invalid sidecar stamp: {error}"))?;
    let handle = stim_controller::service::spawn_local_controller(Some(&stamp.namespace))?;
    let snapshot = handle.snapshot();
    let ready_line = SidecarReadyLine::new(
        stamp,
        "controller-runtime".into(),
        snapshot.instance_id.clone(),
        snapshot.http_base_url.clone(),
        snapshot.published_at.clone(),
    );
    let output = serde_json::to_string(&ready_line)
        .map_err(|error| format!("failed to serialize ready line: {error}"))?;

    println!("{output}");

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

fn proof() -> Result<(), String> {
    match stim_controller::controller::run() {
        Ok(summary) => {
            println!(
                "stim-controller controller proof ok: server={} endpoint={} target={} envelope={} response={} receipt={:?}",
                summary.server_base_url,
                summary.endpoint_id,
                summary.listen_address,
                summary.envelope_id,
                summary.response_text,
                summary.receipt_result
            );
            Ok(())
        }
        Err(error) => Err(format!("controller proof failed: {error:?}")),
    }
}

fn print_help() {
    println!(
        "stim-controller commands:\n  serve <stamp args>  run stamped controller sidecar HTTP runtime\n  proof               run controller proof"
    );
}
