use axum::http::request::Parts;
use litellm_gateway_auth::{Bearer, Credential, CredentialExtractor};
use litellm_gateway_auth_core::{
    PremiumRequired, Token,
    plan::{self, Plan},
};

use super::{Refused, route::Route};

/// Admits a caller the proxy authenticates by a key, which is the master key or a virtual key, or
/// by a JWT. Python answers every other caller: one whose credential the native app does not read,
/// one the proxy authenticates another way, and one whose license does not cover the way it picks.
pub(super) fn check(
    settings: &plan::Settings,
    route: Route,
    request: &Parts,
) -> Result<(), Refused> {
    let Ok(Credential::Token(bearer)) = Bearer.extract(request) else {
        return Err(Refused::Credential);
    };
    let token = Token::new(bearer);
    let planned = plan::plan(
        &plan::Request {
            route: request.uri.path(),
            token: Some(&token),
            is_llm_api_route: route.is_llm_api(),
            ..plan::Request::default()
        },
        settings,
    );
    match planned {
        Ok(Plan::Key | Plan::Jwt) => Ok(()),
        Ok(
            Plan::Public
            | Plan::OAuth2Introspection
            | Plan::OAuth2Proxy
            | Plan::JwtWithOperatorCode
            | Plan::PassThroughOpen,
        )
        | Err(PremiumRequired::OAuth2 | PremiumRequired::Jwt) => Err(Refused::Credential),
    }
}
