mod configuration;
mod credential;
mod refused;
mod route;

pub use refused::{Refused, UserCode};

use std::sync::Arc;

use axum::{extract::Request, http::request::Parts};
use litellm_config::Config;
use litellm_gateway_auth_core::{RouteGroups, plan};

use route::Route;

/// Admits the requests the gateway serves the way the Python proxy would. The configuration is
/// judged once, since what it sets applies to every request.
#[derive(Clone)]
pub struct Admit(Result<Arc<plan::Settings>, Refused>);

impl Admit {
    pub fn unconfigured() -> Self {
        Self(Err(Refused::Configuration))
    }

    /// By `config`, which both the native app and the Python proxy run with, and by what the proxy
    /// knows beyond it: whether its license is an enterprise one, its
    /// `litellm.require_auth_for_metrics_endpoint` and its `LiteLLMRoutes`.
    pub fn from_config(
        config: &Config,
        premium: bool,
        metrics_require_auth: bool,
        route_groups: &RouteGroups,
    ) -> Self {
        let honored = configuration::honored(config);
        if let Err(why) = &honored {
            tracing::info!(%why, "python answers every request");
        }
        Self(honored.map(|()| {
            Arc::new(plan::Settings::new(
                &config.general_settings,
                premium,
                metrics_require_auth,
                route_groups,
            ))
        }))
    }

    /// Admits `request`, or refuses it, giving it back as the client sent it, with why.
    pub async fn admit(self, request: Request) -> Result<Request, (Box<Request>, Refused)> {
        let (parts, body) = request.into_parts();
        let checked = self.check(&parts);
        let request = Request::from_parts(parts, body);
        match checked {
            Ok(()) => Ok(request),
            Err(why) => Err((Box::new(request), why)),
        }
    }

    fn check(&self, request: &Parts) -> Result<(), Refused> {
        let plan = self.0.as_ref().map_err(Clone::clone)?;
        let route = Route::of(request)?;
        credential::check(plan, route, request)?;
        // TODO: the native app only knows the master key, so before anything is admitted it has to
        // authenticate virtual keys and JWTs through gateway-auth-core, as the proxy does
        // TODO: read the body, then admit the models and parameters the native app understands
        Err(Refused::Body)
    }
}
