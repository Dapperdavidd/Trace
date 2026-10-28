use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::header::{HeaderName, HeaderValue},
    Error, HttpMessage,
};
use std::{
    future::{ready, Future, Ready},
    pin::Pin,
};

pub const HEADER: &str = "x-request-id";

/// Request id stored in request extensions so handlers and the error
/// envelope can include it. Honors a sane inbound id, otherwise generates one.
#[derive(Clone, Debug)]
pub struct RequestIdValue(pub String);

pub struct RequestId;

impl<S, B> Transform<S, ServiceRequest> for RequestId
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequestIdMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestIdMiddleware { service }))
    }
}

pub struct RequestIdMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestIdMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let id = req
            .headers()
            .get(HEADER)
            .and_then(|v| v.to_str().ok())
            .filter(|v| {
                !v.is_empty()
                    && v.len() <= 64
                    && v.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            })
            .map(str::to_string)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        req.extensions_mut().insert(RequestIdValue(id.clone()));
        let fut = self.service.call(req);
        Box::pin(async move {
            let mut res = fut.await?;
            if let Ok(v) = HeaderValue::from_str(&id) {
                res.headers_mut().insert(HeaderName::from_static(HEADER), v);
            }
            Ok(res)
        })
    }
}
