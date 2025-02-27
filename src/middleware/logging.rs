use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::Error;
use futures::future::{ok, Ready};
use log::{error, info};
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;

// Middleware for logging request/response details with timing
pub struct LoggingMiddleware;

impl<S, B> Transform<S, ServiceRequest> for LoggingMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = LoggingMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(LoggingMiddlewareService { service })
    }
}

pub struct LoggingMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for LoggingMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start = Instant::now();
        let method = req.method().to_string();
        let path = req.path().to_string();

        info!("Request started: {} {}", method, path);

        let fut = self.service.call(req);
        Box::pin(async move {
            match fut.await {
                Ok(res) => {
                    let duration = start.elapsed();
                    info!(
                        "Request completed: {} {} - {} in {:?}",
                        method,
                        path,
                        res.status(),
                        duration
                    );
                    Ok(res)
                }
                Err(e) => {
                    error!(
                        "Request failed: {} {} - {:?} after {:?}",
                        method,
                        path,
                        e,
                        start.elapsed()
                    );
                    Err(e)
                }
            }
        })
    }
} 