mod configuration;

use axum::{
    extract::Request,
    http::{Method, request::Parts},
};
use litellm_config::Config;
use litellm_gateway_auth::Auth;

/// Why the gateway does not serve a request the way the Python proxy would.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum Refused {
    #[error("the gateway has not read the proxy's configuration")]
    Configuration,
    #[error("the proxy's configuration loads {0}, which only Python runs")]
    UserCode(UserCode),
    #[error("the proxy routes with `{0}`, and the gateway only routes with simple-shuffle")]
    RoutingStrategy(Box<str>),
    #[error("the proxy's configuration sets `{0}`, which the gateway does not honor yet")]
    Setting(Box<str>),
    #[error("the gateway does not serve this route")]
    Route,
    #[error("the gateway cannot check this caller's credential")]
    Credential,
    #[error("the gateway does not read request bodies yet")]
    Body,
}

/// Python the operator wrote, which the proxy imports from a dotted path in its configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::Display)]
pub enum UserCode {
    #[strum(to_string = "custom auth")]
    Auth,
    #[strum(to_string = "a custom callback")]
    Callback,
    #[strum(to_string = "a router plugin")]
    RouterPlugin,
    #[strum(to_string = "a post-call rule")]
    PostCallRule,
}

/// Admits the requests the gateway serves the way the Python proxy would. The configuration is
/// judged once, since what it sets applies to every request.
#[derive(Clone)]
pub struct Admit(Result<Auth, Refused>);

impl Admit {
    pub fn unconfigured() -> Self {
        Self(Err(Refused::Configuration))
    }

    /// By `config`, which both the native app and the Python proxy run with.
    pub fn from_config(config: &Config) -> Self {
        let honored = configuration::honored(config);
        if let Err(why) = &honored {
            tracing::info!(%why, "python answers every request");
        }
        Self(honored.map(|()| Auth::from_config(config)))
    }

    /// Admits `request`, or refuses it, giving it back as the client sent it, with why.
    pub async fn admit(self, request: Request) -> Result<Request, (Box<Request>, Refused)> {
        let (parts, body) = request.into_parts();
        let checked = self.check(&parts).await;
        let request = Request::from_parts(parts, body);
        match checked {
            Ok(()) => Ok(request),
            Err(why) => Err((Box::new(request), why)),
        }
    }

    async fn check(&self, request: &Parts) -> Result<(), Refused> {
        let auth = self.0.as_ref().map_err(Clone::clone)?;
        route(request)?;
        credential(auth, request).await?;
        // TODO: read the body, then admit the models and parameters the native app understands
        Err(Refused::Body)
    }
}

fn route(request: &Parts) -> Result<(), Refused> {
    match (&request.method, request.uri.path()) {
        (&Method::POST, "/v1/messages") => Ok(()),
        _ => Err(Refused::Route),
    }
}

/// The native app only knows the master key, so Python answers every caller it would turn away,
/// who may hold a key that Python accepts.
async fn credential(auth: &Auth, request: &Parts) -> Result<(), Refused> {
    match auth.authenticate_parts(request).await {
        Ok(_) => Ok(()),
        Err(_) => Err(Refused::Credential),
    }
}
