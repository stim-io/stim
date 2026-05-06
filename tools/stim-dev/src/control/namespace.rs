use stim_sidecar::identity::namespace_or_default;

pub(crate) fn current_namespace() -> String {
    namespace_or_default(
        std::env::var(stim_sidecar::identity::SIDECAR_NAMESPACE_ENV)
            .ok()
            .as_deref(),
    )
}
