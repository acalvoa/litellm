use axum::{
    body::Body,
    extract::Request,
    http::{Method, header::AUTHORIZATION},
};
use http_body_util::BodyExt;
use litellm_config::Config;
use litellm_gateway::{Admit, Refused, UserCode};
use litellm_gateway_auth_core::{RouteGroup, RouteGroups};
use rstest::rstest;
use strum::VariantArray;

const BARE: &str = "general_settings: {master_key: sk-master}";
const JWT_AUTH: &str = "general_settings: {master_key: sk-master, enable_jwt_auth: true}";
const JWT: &str = "Bearer header.claims.signature";
const KEY: &str = "Bearer sk-virtual";
const ENTERPRISE: bool = true;
const COMMUNITY: bool = false;

/// The groups the planner reads, each empty, so no route is public unless the configuration says so.
fn route_groups() -> RouteGroups {
    RouteGroups::new(
        RouteGroup::VARIANTS
            .iter()
            .map(|group| (<&str>::from(*group).to_owned(), Vec::new())),
    )
    .unwrap()
}

fn admit_under(config: &str, premium: bool) -> Admit {
    Admit::from_config(
        &Config::from_yaml(config).unwrap(),
        premium,
        true,
        &route_groups(),
    )
}

fn call(method: Method, uri: &str, headers: &[(&str, &str)]) -> Request {
    headers
        .iter()
        .fold(
            Request::builder().method(method).uri(uri),
            |request, (name, value)| request.header(*name, *value),
        )
        .body(Body::from(r#"{"model":"claude","max_tokens":8}"#))
        .unwrap()
}

fn messages_call() -> Request {
    call(
        Method::POST,
        "/v1/messages",
        &[("authorization", "Bearer sk-master")],
    )
}

async fn refusal(admit: Admit, request: Request) -> Refused {
    admit.admit(request).await.unwrap_err().1
}

async fn refusal_under(config: &str) -> Refused {
    refusal(admit_under(config, COMMUNITY), messages_call()).await
}

#[rstest]
#[tokio::test]
async fn a_gateway_that_has_not_read_the_configuration_refuses_every_request() {
    assert_eq!(
        refusal(Admit::unconfigured(), messages_call()).await,
        Refused::Configuration
    );
}

#[rstest]
#[case::bare(BARE)]
#[case::settings_the_native_app_reads(
    "general_settings: {master_key: sk-master}
litellm_settings: {drop_params: true, ssl_verify: false}
environment_variables: {REGION: us-east-1}
router_settings:
  routing_strategy: simple-shuffle
  num_retries: 2
  routing_groups: [{group_name: claude, models: [claude], routing_strategy: simple-shuffle}]"
)]
#[tokio::test]
async fn a_configuration_the_native_app_honors_gets_as_far_as_the_body(#[case] config: &str) {
    assert_eq!(refusal_under(config).await, Refused::Body);
}

#[rstest]
#[case::custom_auth(
    "general_settings: {custom_auth: custom_auth.user_api_key_auth}",
    UserCode::Auth
)]
#[case::one_callback(
    "litellm_settings: {callbacks: custom_callbacks.proxy_handler_instance}",
    UserCode::Callback
)]
#[case::success_callback_among_named_ones(
    "litellm_settings: {success_callback: [langfuse, custom_callbacks.on_success]}",
    UserCode::Callback
)]
#[case::failure_callback(
    "litellm_settings: {failure_callback: [custom_callbacks.on_failure]}",
    UserCode::Callback
)]
#[case::router_plugin(
    "router_settings: {plugins: [routing_plugins.Canary]}",
    UserCode::RouterPlugin
)]
#[case::post_call_rule(
    "litellm_settings: {post_call_rules: rules.no_refusals}",
    UserCode::PostCallRule
)]
#[case::named_ahead_of_a_setting_the_gateway_does_not_honor(
    "general_settings: {database_url: 'postgresql://db/litellm', custom_auth: custom_auth.check}",
    UserCode::Auth
)]
#[tokio::test]
async fn python_answers_every_request_when_the_configuration_loads_user_code(
    #[case] config: &str,
    #[case] kind: UserCode,
) {
    assert_eq!(refusal_under(config).await, Refused::UserCode(kind));
}

#[rstest]
#[case::default_strategy("router_settings: {routing_strategy: least-busy}", "least-busy")]
#[case::one_routing_group(
    "router_settings:
  routing_strategy: simple-shuffle
  routing_groups: [{group_name: fast, models: [claude], routing_strategy: latency-based-routing}]",
    "latency-based-routing"
)]
#[tokio::test]
async fn python_answers_every_request_when_it_routes_with_another_strategy(
    #[case] config: &str,
    #[case] strategy: &str,
) {
    assert_eq!(
        refusal_under(config).await,
        Refused::RoutingStrategy(strategy.into())
    );
}

#[rstest]
#[case::named_callback(
    "litellm_settings: {callbacks: [prometheus]}",
    "litellm_settings.callbacks"
)]
#[case::database(
    "general_settings: {database_url: 'postgresql://db/litellm'}",
    "general_settings.database_url"
)]
#[case::flag(
    "general_settings: {enforce_rbac: true}",
    "general_settings.enforce_rbac"
)]
#[case::router_setting(
    "router_settings: {enable_tag_filtering: true}",
    "router_settings.enable_tag_filtering"
)]
#[case::section("guardrails: [{guardrail_name: pii}]", "guardrails")]
#[case::key_the_gateway_does_not_know(
    "general_settings: {alerting: [slack]}",
    "general_settings.alerting"
)]
#[case::section_the_gateway_does_not_know(
    "scheduler_settings: {interval: 5}",
    "scheduler_settings"
)]
#[tokio::test]
async fn python_answers_every_request_when_a_setting_is_not_honored(
    #[case] config: &str,
    #[case] setting: &str,
) {
    assert_eq!(
        refusal_under(config).await,
        Refused::Setting(setting.into())
    );
}

#[rstest]
#[case::another_method(Method::GET, "/v1/messages")]
#[case::another_endpoint(Method::POST, "/v1/chat/completions")]
#[case::a_path_under_messages(Method::POST, "/v1/messages/count_tokens")]
#[tokio::test]
async fn python_answers_every_route_but_a_messages_call(#[case] method: Method, #[case] uri: &str) {
    let admit = admit_under(BARE, COMMUNITY);
    let request = call(method, uri, &[("authorization", "Bearer sk-master")]);

    assert_eq!(refusal(admit, request).await, Refused::Route);
}

#[rstest]
#[tokio::test]
async fn a_query_string_does_not_change_the_route() {
    let admit = admit_under(BARE, COMMUNITY);
    let request = call(
        Method::POST,
        "/v1/messages?beta=true",
        &[("authorization", "Bearer sk-master")],
    );

    assert_eq!(refusal(admit, request).await, Refused::Body);
}

#[rstest]
#[case::the_master_key(BARE, COMMUNITY, "Bearer sk-master")]
#[case::a_virtual_key(BARE, COMMUNITY, KEY)]
#[case::a_jwt(JWT_AUTH, ENTERPRISE, JWT)]
#[case::a_key_beside_jwt_auth(JWT_AUTH, ENTERPRISE, KEY)]
#[case::a_jwt_shaped_key_when_jwt_auth_is_off(BARE, COMMUNITY, JWT)]
#[case::a_key_when_the_operators_code_only_judges_jwts(
    "general_settings: {enable_jwt_auth: true, litellm_jwtauth: {custom_validate: hooks.validate}}",
    ENTERPRISE,
    KEY
)]
#[tokio::test]
async fn a_caller_the_proxy_authenticates_by_a_key_or_a_jwt_gets_as_far_as_the_body(
    #[case] config: &str,
    #[case] premium: bool,
    #[case] authorization: &str,
) {
    let request = call(
        Method::POST,
        "/v1/messages",
        &[("authorization", authorization)],
    );

    assert_eq!(
        refusal(admit_under(config, premium), request).await,
        Refused::Body
    );
}

#[rstest]
#[case::a_jwt_the_license_does_not_cover(JWT_AUTH, COMMUNITY, JWT)]
#[case::a_jwt_the_operators_code_judges(
    "general_settings: {enable_jwt_auth: true, litellm_jwtauth: {custom_validate: hooks.validate}}",
    ENTERPRISE,
    JWT
)]
#[case::a_token_for_oauth2_introspection(
    "general_settings: {enable_oauth2_auth: true}",
    ENTERPRISE,
    KEY
)]
#[case::oauth2_introspection_the_license_does_not_cover(
    "general_settings: {enable_oauth2_auth: true}",
    COMMUNITY,
    KEY
)]
#[case::a_caller_the_oauth2_proxy_vouches_for(
    "general_settings: {enable_oauth2_proxy_auth: true}",
    COMMUNITY,
    KEY
)]
#[case::a_route_the_configuration_makes_public(
    "general_settings: {public_routes: [/v1/messages]}",
    ENTERPRISE,
    KEY
)]
#[tokio::test]
async fn python_answers_a_caller_the_proxy_authenticates_another_way(
    #[case] config: &str,
    #[case] premium: bool,
    #[case] authorization: &str,
) {
    let request = call(
        Method::POST,
        "/v1/messages",
        &[("authorization", authorization)],
    );

    assert_eq!(
        refusal(admit_under(config, premium), request).await,
        Refused::Credential
    );
}

#[rstest]
#[case::none(&[])]
#[case::a_key_in_a_header_the_native_app_does_not_read(&[("x-api-key", "sk-master")])]
#[case::a_scheme_the_native_app_does_not_read(&[("authorization", "Basic sk-master")])]
#[tokio::test]
async fn python_answers_a_caller_whose_credential_the_native_app_does_not_read(
    #[case] headers: &[(&str, &str)],
) {
    let request = call(Method::POST, "/v1/messages", headers);

    assert_eq!(
        refusal(admit_under(BARE, COMMUNITY), request).await,
        Refused::Credential
    );
}

#[rstest]
#[tokio::test]
async fn a_refused_request_comes_back_as_the_client_sent_it() {
    let admit = admit_under(BARE, COMMUNITY);
    let sent = call(
        Method::POST,
        "/v1/messages?beta=true",
        &[("authorization", "Bearer sk-virtual"), ("x-request", "one")],
    );

    let (request, _) = admit.admit(sent).await.unwrap_err();

    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri(), "/v1/messages?beta=true");
    assert_eq!(request.headers()[AUTHORIZATION], "Bearer sk-virtual");
    assert_eq!(request.headers()["x-request"], "one");
    let body = request.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body, r#"{"model":"claude","max_tokens":8}"#);
}

#[rstest]
#[case(Refused::Configuration, "configuration")]
#[case(Refused::UserCode(UserCode::Callback), "user_code")]
#[case(Refused::RoutingStrategy("least-busy".into()), "routing_strategy")]
#[case(Refused::Setting("guardrails".into()), "setting")]
#[case(Refused::Route, "route")]
#[case(Refused::Credential, "credential")]
#[case(Refused::Body, "body")]
fn each_refusal_names_itself_to_the_client_with_one_word(
    #[case] refused: Refused,
    #[case] reason: &'static str,
) {
    assert_eq!(<&'static str>::from(refused), reason);
}
