use actix_web::{
    body::{to_bytes, BoxBody, MessageBody},
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

/// Marker set on error responses whose JSON body still needs the request id.
#[derive(Clone, Copy, Debug)]
pub struct NeedsRequestId;

pub struct RequestId;

impl<S, B> Transform<S, ServiceRequest> for RequestId
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
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
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
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
            let mut res = fut.await?.map_into_boxed_body();
            if res
                .response()
                .extensions()
                .get::<NeedsRequestId>()
                .is_some()
            {
                let (req, resp) = res.into_parts();
                let (head, body) = resp.into_parts();
                let bytes = to_bytes(body)
                    .await
                    .map_err(|_| actix_web::error::ErrorInternalServerError("body"))?;
                let patched = match serde_json::from_slice::<serde_json::Value>(&bytes) {
                    Ok(mut v) => {
                        v["request_id"] = serde_json::Value::String(id.clone());
                        serde_json::to_vec(&v).unwrap_or_else(|_| bytes.to_vec())
                    }
                    Err(_) => bytes.to_vec(),
                };
                res = ServiceResponse::new(req, head.set_body(BoxBody::new(patched)));
            }
            if let Ok(v) = HeaderValue::from_str(&id) {
                res.headers_mut().insert(HeaderName::from_static(HEADER), v);
            }
            Ok(res)
        })
    }
}
