#![allow(dead_code)]

use axum::{
    extract::MatchedPath,
    http::{Request, Response},
};
#[cfg(feature = "observability-opentelemetry")]
use opentelemetry::{
    metrics::{Counter, Histogram, Meter, UpDownCounter},
    KeyValue,
};
use std::time::Duration;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::trace::{MakeSpan, OnFailure, OnRequest, OnResponse};
use tracing::Span;

fn parse_span_name(span: &Span) -> (String, String) {
    let span_name = span
        .metadata()
        .map(|m| m.name())
        .unwrap_or("unknown unknown");

    let parts: Vec<&str> = span_name.splitn(2, ' ').collect();
    if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        ("unknown".to_string(), "unknown".to_string())
    }
}

#[derive(Clone)]
#[cfg(feature = "observability-opentelemetry")]
pub struct HttpMetrics {
    request_counter: Counter<u64>,
    request_duration: Histogram<f64>,
    active_requests: UpDownCounter<i64>,
    error_counter: Counter<u64>,
}

#[derive(Clone)]
#[cfg(not(feature = "observability-opentelemetry"))]
pub struct HttpMetrics;

#[cfg(feature = "observability-opentelemetry")]
impl HttpMetrics {
    pub fn new(meter: Meter) -> Self {
        let request_counter = meter
            .u64_counter("http.server.request.count")
            .with_description("Total number of HTTP requests")
            .build();

        let request_duration = meter
            .f64_histogram("http.server.request.duration")
            .with_description("HTTP request duration in seconds")
            .with_unit("s")
            .build();

        let active_requests = meter
            .i64_up_down_counter("http.server.active_requests")
            .with_description("Number of active HTTP requests")
            .build();

        let error_counter = meter
            .u64_counter("http.server.errors.count")
            .with_description("Total number of HTTP errors (4xx and 5xx responses)")
            .build();

        Self {
            request_counter,
            request_duration,
            active_requests,
            error_counter,
        }
    }

    pub fn record_request_start(&self, method: &str, path: &str) {
        self.active_requests.add(
            1,
            &[
                KeyValue::new("http.method", method.to_string()),
                KeyValue::new("http.route", path.to_string()),
            ],
        );
    }

    pub fn record_request_end(&self, method: &str, path: &str, status: u16, duration: f64) {
        let labels = &[
            KeyValue::new("http.method", method.to_string()),
            KeyValue::new("http.route", path.to_string()),
            KeyValue::new("http.status_code", status.to_string()),
        ];

        self.request_counter.add(1, labels);
        self.request_duration.record(duration, labels);
        self.active_requests.add(-1, labels);

        // Record error if status is 4xx or 5xx
        if status >= 400 {
            self.error_counter.add(1, labels);
        }
    }
}

#[cfg(not(feature = "observability-opentelemetry"))]
impl HttpMetrics {
    pub fn new(_meter: ()) -> Self {
        Self {}
    }

    pub fn record_request_start(&self, _method: &str, _path: &str) {
        // No-op when OpenTelemetry is disabled
    }

    pub fn record_request_end(&self, _method: &str, _path: &str, _status: u16, _duration: f64) {
        // No-op when OpenTelemetry is disabled
    }
}

#[derive(Clone)]
pub struct CustomMakeSpan {
    #[cfg(feature = "observability-opentelemetry")]
    metrics: HttpMetrics,
}

impl CustomMakeSpan {
    #[cfg(feature = "observability-opentelemetry")]
    pub fn new(metrics: HttpMetrics) -> Self {
        Self { metrics }
    }

    #[cfg(not(feature = "observability-opentelemetry"))]
    pub fn new(_metrics: HttpMetrics) -> Self {
        Self {}
    }
}

impl<B> MakeSpan<B> for CustomMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let method = request.method().as_str();
        let uri = request.uri().path();
        let path = request
            .extensions()
            .get::<MatchedPath>()
            .map(|mp| mp.as_str())
            .unwrap_or(uri);

        // Record request start
        #[cfg(feature = "observability-opentelemetry")]
        {
            self.metrics.record_request_start(method, path);
        }

        let span = tracing::info_span!(
            "http_request",
            otel.name = %format!("{} {}", method, path),
            otel.kind = "server",
            http.method = %method,
            http.route = %path,
            http.target = %uri,
            http.scheme = ?request.uri().scheme_str(),
            http.client_ip = tracing::field::Empty,
            http.user_agent = tracing::field::Empty,
            http.status_code = tracing::field::Empty,
            http.request_content_length = tracing::field::Empty,
            http.response_content_length = tracing::field::Empty,
        );

        if let Some(addr) = request
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .or_else(|| {
                request
                    .headers()
                    .get("x-real-ip")
                    .and_then(|v| v.to_str().ok())
            })
        {
            span.record("http.client_ip", addr);
        }

        if let Some(user_agent) = request
            .headers()
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
        {
            span.record("http.user_agent", user_agent);
        }

        if let Some(content_length) = request
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
        {
            span.record("http.request_content_length", content_length);
        }

        span
    }
}

#[derive(Clone)]
pub struct CustomOnRequest;

impl<B> OnRequest<B> for CustomOnRequest {
    fn on_request(&mut self, _request: &Request<B>, _span: &Span) {
        tracing::debug!("started processing request");
    }
}

#[derive(Clone)]
pub struct CustomOnResponse {
    #[cfg(feature = "observability-opentelemetry")]
    metrics: HttpMetrics,
}

impl CustomOnResponse {
    #[cfg(feature = "observability-opentelemetry")]
    pub fn new(metrics: HttpMetrics) -> Self {
        Self { metrics }
    }

    #[cfg(not(feature = "observability-opentelemetry"))]
    pub fn new(_metrics: HttpMetrics) -> Self {
        Self {}
    }
}

impl<B> OnResponse<B> for CustomOnResponse {
    fn on_response(self, response: &Response<B>, latency: Duration, span: &Span) {
        let status = response.status().as_u16();
        span.record("http.status_code", status);

        if let Some(content_length) = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
        {
            span.record("http.response_content_length", content_length);
        }

        let (_method, _path) = parse_span_name(span);

        #[cfg(feature = "observability-opentelemetry")]
        {
            self.metrics
                .record_request_end(&_method, &_path, status, latency.as_secs_f64());
        }

        if response.status().is_server_error() {
            tracing::error!(
                status = status,
                latency_ms = latency.as_millis(),
                "request failed"
            );
        } else if response.status().is_client_error() {
            tracing::warn!(
                status = status,
                latency_ms = latency.as_millis(),
                "client error"
            );
        } else {
            tracing::info!(
                status = status,
                latency_ms = latency.as_millis(),
                "request completed"
            );
        }
    }
}

#[derive(Clone)]
pub struct CustomOnFailure {
    #[cfg(feature = "observability-opentelemetry")]
    metrics: HttpMetrics,
}

impl CustomOnFailure {
    #[cfg(feature = "observability-opentelemetry")]
    pub fn new(metrics: HttpMetrics) -> Self {
        Self { metrics }
    }

    #[cfg(not(feature = "observability-opentelemetry"))]
    pub fn new(_metrics: HttpMetrics) -> Self {
        Self {}
    }
}

impl OnFailure<ServerErrorsFailureClass> for CustomOnFailure {
    fn on_failure(&mut self, failure: ServerErrorsFailureClass, latency: Duration, span: &Span) {
        // Record error in span
        span.record("otel.status_code", "ERROR");
        span.record("http.status_code", 500);

        tracing::error!(
            latency_ms = latency.as_millis(),
            error = ?failure,
            "request failed with error"
        );

        // Extract method and path from span name
        let (_method, _path) = parse_span_name(span);

        // Record metrics
        #[cfg(feature = "observability-opentelemetry")]
        {
            self.metrics
                .record_request_end(&_method, &_path, 500, latency.as_secs_f64());
        }
    }
}
