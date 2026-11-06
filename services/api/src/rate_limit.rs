//! Small in-process request limiter for a single API instance.
//!
//! Limits are applied to the direct peer address and, when present, to a
//! one-way hash of the bearer token. Deployments with multiple API instances
//! must also enforce a shared limit at the edge.

use actix_web::{
    body::{BoxBody, MessageBody},
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::{header, StatusCode},
    Error,
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    future::{ready, Future, Ready},
    pin::Pin,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::error;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub anonymous_ip_per_minute: u32,
    pub authenticated_ip_per_minute: u32,
    pub user_per_minute: u32,
    pub sign_in_ip_per_minute: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            anonymous_ip_per_minute: 120,
            authenticated_ip_per_minute: 600,
            user_per_minute: 300,
            sign_in_ip_per_minute: 10,
        }
    }
}

#[derive(Clone)]
pub struct RateLimit {
    inner: Arc<Mutex<State>>,
    limits: Limits,
}

#[derive(Default)]
struct State {
    buckets: HashMap<String, Bucket>,
    requests_since_cleanup: u32,
}

struct Bucket {
    started: Instant,
    count: u32,
}

impl RateLimit {
    pub fn new(limits: Limits) -> Self {
        Self {
            inner: Arc::new(Mutex::new(State::default())),
            limits,
        }
    }

    fn accepts(&self, keys: &[(String, u32)], now: Instant) -> bool {
        let mut state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        state.requests_since_cleanup += 1;
        if state.requests_since_cleanup >= 1_000 {
            state
                .buckets
                .retain(|_, b| now.duration_since(b.started) < Duration::from_secs(120));
            state.requests_since_cleanup = 0;
        }
        if keys.iter().any(|(key, limit)| {
            state.buckets.get(key).is_some_and(|bucket| {
                now.duration_since(bucket.started) < Duration::from_secs(60)
                    && bucket.count >= *limit
            })
        }) {
            return false;
        }
        for (key, _) in keys {
            let bucket = state.buckets.entry(key.clone()).or_insert(Bucket {
                started: now,
                count: 0,
            });
            if now.duration_since(bucket.started) >= Duration::from_secs(60) {
                bucket.started = now;
                bucket.count = 0;
            }
            bucket.count += 1;
        }
        true
    }
}

impl Default for RateLimit {
    fn default() -> Self {
        Self::new(Limits::default())
    }
}

fn bearer_key(req: &ServiceRequest) -> Option<String> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")?;
    let digest = Sha256::digest(token.as_bytes());
    Some(format!("user:{}", hex::encode(digest)))
}

fn request_keys(req: &ServiceRequest, limits: Limits) -> Vec<(String, u32)> {
    let peer = req
        .peer_addr()
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|| "unknown".into());
    if req.path() == "/v1/auth/google" || req.path().starts_with("/v1/auth/passkeys/") {
        return vec![(format!("signin:{peer}"), limits.sign_in_ip_per_minute)];
    }
    if let Some(user) = bearer_key(req) {
        return vec![
            (
                format!("auth-ip:{peer}"),
                limits.authenticated_ip_per_minute,
            ),
            (user, limits.user_per_minute),
        ];
    }
    vec![(
        format!("anonymous-ip:{peer}"),
        limits.anonymous_ip_per_minute,
    )]
}

impl<S, B> Transform<S, ServiceRequest> for RateLimit
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = RateLimitMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitMiddleware {
            service,
            limiter: self.clone(),
        }))
    }
}

pub struct RateLimitMiddleware<S> {
    service: S,
    limiter: RateLimit,
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let keys = request_keys(&req, self.limiter.limits);
        if !self.limiter.accepts(&keys, Instant::now()) {
            let mut response = error::envelope(
                req.request(),
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "Too many requests. Please try again shortly.",
                None,
            );
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, header::HeaderValue::from_static("60"));
            let response = req.into_response(response).map_into_boxed_body();
            return Box::pin(async move { Ok(response) });
        }
        let future = self.service.call(req);
        Box::pin(async move { Ok(future.await?.map_into_boxed_body()) })
    }
}
