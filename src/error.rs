use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub struct Error {
    inner: Box<dyn std::error::Error>,
}

impl<T: std::error::Error + 'static> From<T> for Error {
    fn from(value: T) -> Self {
        Error {
            inner: value.into(),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.inner.to_string()).into_response()
    }
}
