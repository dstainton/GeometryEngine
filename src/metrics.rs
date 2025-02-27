use actix_web::{web, HttpResponse};
use prometheus::{register_histogram_vec, register_int_counter_vec, HistogramVec, IntCounterVec};
use lazy_static::lazy_static;

lazy_static! {
    // HTTP request duration histogram by endpoint, method, and status
    static ref HTTP_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["endpoint", "method", "status"]
    ).unwrap();

    // Total HTTP requests counter by endpoint, method, and status
    static ref HTTP_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "http_requests_total",
        "Total number of HTTP requests",
        &["endpoint", "method", "status"]
    ).unwrap();

    // Add rate limit metrics
    static ref RATE_LIMIT_EXCEEDED: IntCounterVec = register_int_counter_vec!(
        "rate_limit_exceeded_total",
        "Total number of rate limit exceeded events",
        &["api_key"]
    ).unwrap();
}

pub async fn metrics() -> HttpResponse {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let mut buffer = Vec::new();
    encoder.encode(&prometheus::gather(), &mut buffer).unwrap();
    
    HttpResponse::Ok()
        .content_type("text/plain")
        .body(String::from_utf8(buffer).unwrap())
} 