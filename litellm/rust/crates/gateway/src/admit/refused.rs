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
    #[error("the gateway only authenticates a caller by a bearer key or JWT")]
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
