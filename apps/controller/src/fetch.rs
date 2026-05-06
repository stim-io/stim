use std::{fmt, sync::Arc, thread, time::Duration};

use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderName, HeaderValue},
    Method, StatusCode,
};
use serde::de::DeserializeOwned;

#[derive(Clone)]
pub struct FetchClient {
    base_url: String,
    client: Client,
}

impl FetchClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
        }
    }

    pub fn get_json<T>(
        &self,
        path: impl AsRef<str>,
        options: FetchRequestOptions<T>,
    ) -> Result<FetchOutcome<T>, FetchError>
    where
        T: DeserializeOwned,
    {
        self.request_json(Method::GET, path.as_ref(), options)
    }

    fn request_json<T>(
        &self,
        method: Method,
        path: &str,
        options: FetchRequestOptions<T>,
    ) -> Result<FetchOutcome<T>, FetchError>
    where
        T: DeserializeOwned,
    {
        let started = std::time::Instant::now();
        let mut metadata = FetchMetadata::default();
        let max_attempts = options.retry.max_attempts();

        for attempt in 1..=max_attempts {
            metadata.attempts = attempt;
            let request = self.build_request(&method, path, &options);

            match request.send() {
                Ok(response) => {
                    let status = response.status();
                    metadata.last_status = Some(status.as_u16());

                    if status == StatusCode::NOT_FOUND {
                        if let FetchNotFound::Payload(payload) = &options.not_found {
                            if let Some(delay) =
                                status_retry_delay(&options.retry, attempt, &method, path, status)
                            {
                                metadata.retries += 1;
                                thread::sleep(delay);
                                continue;
                            }

                            metadata.elapsed_ms = started.elapsed().as_millis();
                            return Ok(FetchOutcome {
                                payload: payload(),
                                metadata,
                            });
                        }
                    }

                    if options.status_policy.accepts(status) {
                        let payload = response.json::<T>().map_err(|error| {
                            metadata.elapsed_ms = started.elapsed().as_millis();
                            FetchError::new(format!("fetch JSON decode failed: {error}"), metadata)
                        })?;
                        metadata.elapsed_ms = started.elapsed().as_millis();
                        return Ok(FetchOutcome { payload, metadata });
                    }

                    if let Some(delay) =
                        status_retry_delay(&options.retry, attempt, &method, path, status)
                    {
                        metadata.retries += 1;
                        thread::sleep(delay);
                        continue;
                    }

                    metadata.elapsed_ms = started.elapsed().as_millis();
                    return Err(FetchError::new(
                        format!("fetch status failed: HTTP {status}"),
                        metadata,
                    ));
                }
                Err(error) => {
                    let error = error.to_string();
                    if let Some(delay) = options.retry.retry_delay(FetchRetryContext {
                        attempt,
                        method: &method,
                        path,
                        status: None,
                        error: Some(error.as_str()),
                    }) {
                        metadata.retries += 1;
                        thread::sleep(delay);
                        continue;
                    }

                    metadata.elapsed_ms = started.elapsed().as_millis();
                    return Err(FetchError::new(
                        format!("fetch request failed: {error}"),
                        metadata,
                    ));
                }
            }
        }

        unreachable!("FetchRetry must expose at least one attempt")
    }

    fn build_request<T>(
        &self,
        method: &Method,
        path: &str,
        options: &FetchRequestOptions<T>,
    ) -> reqwest::blocking::RequestBuilder {
        let mut request = self
            .client
            .request(method.clone(), self.request_url(path))
            .headers(options.headers.clone());

        if let Some(timeout) = options.timeout {
            request = request.timeout(timeout);
        }
        if !options.query.is_empty() {
            request = request.query(&options.query);
        }

        request
    }

    fn request_url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            return path.to_string();
        }

        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}

pub struct FetchRequestOptions<T> {
    retry: FetchRetry,
    timeout: Option<Duration>,
    headers: HeaderMap,
    query: Vec<(String, String)>,
    status_policy: FetchStatusPolicy,
    not_found: FetchNotFound<T>,
}

impl<T> Default for FetchRequestOptions<T> {
    fn default() -> Self {
        Self {
            retry: FetchRetry::Off,
            timeout: None,
            headers: HeaderMap::new(),
            query: Vec::new(),
            status_policy: FetchStatusPolicy::Success2xx,
            not_found: FetchNotFound::Error,
        }
    }
}

impl<T> FetchRequestOptions<T> {
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    pub fn with_query_param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.push((name.into(), value.into()));
        self
    }

    pub fn with_status_policy(mut self, status_policy: FetchStatusPolicy) -> Self {
        self.status_policy = status_policy;
        self
    }

    pub fn with_retry(mut self, retry: FetchRetry) -> Self {
        self.retry = retry;
        self
    }

    pub fn with_not_found_payload(
        mut self,
        payload: impl Fn() -> T + Send + Sync + 'static,
    ) -> Self {
        self.not_found = FetchNotFound::Payload(Arc::new(payload));
        self
    }
}

enum FetchNotFound<T> {
    Error,
    Payload(Arc<dyn Fn() -> T + Send + Sync>),
}

pub enum FetchStatusPolicy {
    Success2xx,
    Custom(Arc<dyn Fn(StatusCode) -> bool + Send + Sync>),
}

impl FetchStatusPolicy {
    pub fn custom(accepts: impl Fn(StatusCode) -> bool + Send + Sync + 'static) -> Self {
        Self::Custom(Arc::new(accepts))
    }

    fn accepts(&self, status: StatusCode) -> bool {
        match self {
            Self::Success2xx => status.is_success(),
            Self::Custom(accepts) => accepts(status),
        }
    }
}

#[derive(Clone)]
pub enum FetchRetry {
    Off,
    Policy(FetchRetryPolicy),
    Custom {
        policy: FetchRetryPolicy,
        decide: Arc<dyn Fn(&FetchRetryContext<'_>) -> FetchRetryDecision + Send + Sync>,
    },
}

impl FetchRetry {
    pub fn santi_transient() -> Self {
        Self::Policy(FetchRetryPolicy::santi_transient())
    }

    pub fn custom(
        policy: FetchRetryPolicy,
        decide: impl Fn(&FetchRetryContext<'_>) -> FetchRetryDecision + Send + Sync + 'static,
    ) -> Self {
        Self::Custom {
            policy,
            decide: Arc::new(decide),
        }
    }

    fn max_attempts(&self) -> usize {
        match self {
            Self::Off => 1,
            Self::Policy(policy) | Self::Custom { policy, .. } => policy.max_attempts.max(1),
        }
    }

    fn retry_delay(&self, context: FetchRetryContext<'_>) -> Option<Duration> {
        let (policy, decision) = match self {
            Self::Off => return None,
            Self::Policy(policy) => (policy, default_retry_decision(&context)),
            Self::Custom { policy, decide } => (policy, decide(&context)),
        };

        if context.attempt >= policy.max_attempts.max(1) {
            return None;
        }

        match decision {
            FetchRetryDecision::Fail => None,
            FetchRetryDecision::Retry => Some(policy.delay(context.attempt)),
            FetchRetryDecision::RetryAfter(delay) => Some(delay),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FetchRetryPolicy {
    max_attempts: usize,
    base_delay_ms: u64,
    max_delay_ms: u64,
}

impl FetchRetryPolicy {
    pub fn new(max_attempts: usize, base_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            base_delay_ms,
            max_delay_ms,
        }
    }

    pub fn santi_transient() -> Self {
        Self::new(5, 100, 500)
    }

    fn delay(&self, attempt: usize) -> Duration {
        let delay_ms = (self.base_delay_ms * attempt as u64).min(self.max_delay_ms);
        Duration::from_millis(delay_ms)
    }
}

pub struct FetchRetryContext<'a> {
    pub attempt: usize,
    pub method: &'a Method,
    pub path: &'a str,
    pub status: Option<u16>,
    pub error: Option<&'a str>,
}

pub enum FetchRetryDecision {
    Retry,
    RetryAfter(Duration),
    Fail,
}

fn default_retry_decision(context: &FetchRetryContext<'_>) -> FetchRetryDecision {
    match context
        .status
        .and_then(|status| StatusCode::from_u16(status).ok())
    {
        Some(status) if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS => {
            FetchRetryDecision::Retry
        }
        None if context.error.is_some() => FetchRetryDecision::Retry,
        _ => FetchRetryDecision::Fail,
    }
}

fn status_retry_delay(
    retry: &FetchRetry,
    attempt: usize,
    method: &Method,
    path: &str,
    status: StatusCode,
) -> Option<Duration> {
    retry.retry_delay(FetchRetryContext {
        attempt,
        method,
        path,
        status: Some(status.as_u16()),
        error: None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FetchMetadata {
    pub attempts: usize,
    pub retries: usize,
    pub last_status: Option<u16>,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchOutcome<T> {
    pub payload: T,
    pub metadata: FetchMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchError {
    message: String,
    pub metadata: FetchMetadata,
}

impl FetchError {
    fn new(message: String, metadata: FetchMetadata) -> Self {
        Self { message, metadata }
    }
}

impl fmt::Display for FetchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for FetchError {}
