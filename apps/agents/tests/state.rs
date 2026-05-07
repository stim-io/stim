use stim_agents::state::{AppState, SantiInstanceConfig};

#[test]
fn rejects_dup_santi_ids() {
    let result = AppState::santi_instances(
        "test",
        vec![
            SantiInstanceConfig {
                agent_id: None,
                participant_id: None,
                delivery_endpoint_id: None,
                id: "dup".into(),
                label: "First".into(),
                endpoint: "http://127.0.0.1:18081".into(),
                profile: None,
                managed: false,
                launch: None,
            },
            SantiInstanceConfig {
                agent_id: None,
                participant_id: None,
                delivery_endpoint_id: None,
                id: "dup".into(),
                label: "Second".into(),
                endpoint: "http://127.0.0.1:18082".into(),
                profile: None,
                managed: false,
                launch: None,
            },
        ],
    );

    assert!(matches!(
        result.as_ref().map_err(String::as_str),
        Err(error) if error.contains("duplicate Santi instance id")
    ));
}
