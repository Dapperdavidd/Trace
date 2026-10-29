use actix_web::{http::StatusCode, web, HttpRequest, HttpResponse, ResponseError};
use serde_json::Value;
use trace_domain::{ApiErrorBody, DomainError};

use crate::request_id::RequestIdValue;

/// Application error. Rendered as the standard envelope; internal causes are
/// logged, never returned to clients.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    LimitReached(String),
    #[error("dependency unavailable: {0}")]
    Unavailable(String),
    #[error("internal error")]
    Internal(#[source] anyhow::Error),
    #[error(transparent)]
    Domain(#[from] DomainError),
}

impl ApiError {
    pub fn code(&self) -> &'static str {
        match self {
            ApiError::BadRequest(_) => "bad_request",
            ApiError::NotFound => "not_found",
            ApiError::Unauthorized => "unauthorized",
            ApiError::Conflict(_) => "conflict",
            ApiError::LimitReached(_) => "limit_reached",
            ApiError::Unavailable(_) => "dependency_unavailable",
            ApiError::Internal(_) => "internal_error",
            ApiError::Domain(DomainError::UnknownChain(_)) => "unknown_chain",
            ApiError::Domain(DomainError::InvalidAddress { .. }) => "invalid_address",
            ApiError::Domain(DomainError::InvalidAmount(_)) => "invalid_amount",
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) | ApiError::Domain(_) => StatusCode::BAD_REQUEST,
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::LimitReached(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let ApiError::Internal(e) = self {
            tracing::error!(error = ?e, "internal error");
        }
        let message = match self {
            ApiError::Internal(_) => "Something went wrong on our side.".to_string(),
            other => other.to_string(),
        };
        // request_id is attached by `envelope` where the request is available;
        // handlers returning ApiError directly get an empty id filled in below.
        let mut resp = HttpResponse::build(self.status_code()).json(ApiErrorBody {
            code: self.code().to_string(),
            message,
            details: None,
            request_id: String::new(),
        });
        resp.extensions_mut()
            .insert(crate::request_id::NeedsRequestId);
        resp
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        ApiError::Internal(e.into())
    }
}

pub fn request_id_of(req: &HttpRequest) -> String {
    use actix_web::HttpMessage;
    req.extensions()
        .get::<RequestIdValue>()
        .map(|r| r.0.clone())
        .unwrap_or_default()
}

pub fn envelope(
    req: &HttpRequest,
    status: StatusCode,
    code: &str,
    message: &str,
    details: Option<Value>,
) -> HttpResponse {
    HttpResponse::build(status).json(ApiErrorBody {
        code: code.to_string(),
        message: message.to_string(),
        details,
        request_id: request_id_of(req),
    })
}

pub async fn not_found(req: HttpRequest) -> HttpResponse {
    envelope(
        &req,
        StatusCode::NOT_FOUND,
        "not_found",
        "No such route.",
        None,
    )
}

/// Malformed JSON bodies produce the standard envelope rather than plain text.
pub fn json_config() -> web::JsonConfig {
    web::JsonConfig::default()
        .limit(64 * 1024)
        .error_handler(|err, req| {
            let resp = envelope(
                req,
                StatusCode::BAD_REQUEST,
                "invalid_json",
                &err.to_string(),
                None,
            );
            actix_web::error::InternalError::from_response(err, resp).into()
        })
}
