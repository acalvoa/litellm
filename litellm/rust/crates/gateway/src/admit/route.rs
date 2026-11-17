use axum::http::{Method, request::Parts};

use super::Refused;

/// A route the native app serves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Route {
    Messages,
}

impl Route {
    pub(super) fn of(request: &Parts) -> Result<Self, Refused> {
        match (&request.method, request.uri.path()) {
            (&Method::POST, "/v1/messages") => Ok(Self::Messages),
            _ => Err(Refused::Route),
        }
    }

    /// `RouteChecks.is_llm_api_route`, which the proxy's auth planner reads.
    pub(super) fn is_llm_api(self) -> bool {
        match self {
            Self::Messages => true,
        }
    }
}
