fn main() {
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
        }
        Err(error) => {
            eprintln!("stim-controller controller proof failed: {error:?}");
            std::process::exit(1);
        }
    }
}
