use std::collections::BTreeMap;

use litellm_config::{
    AdditionalFields, Config, GeneralSettings, LiteLlmSettings, Object, RouterSettings, Value,
};

use super::{Refused, UserCode};

const SIMPLE_SHUFFLE: &str = "simple-shuffle";

/// Whether the native app runs `config` the way the Python proxy does. A setting is honored when
/// the native app reads it, when it only shapes routes the gateway never admits, or when it only
/// picks how a caller is authenticated, which `credential` judges for each request.
pub(super) fn honored(config: &Config) -> Result<(), Refused> {
    user_code(config).map_or(Ok(()), |kind| Err(Refused::UserCode(kind)))?;
    routing_strategy(&config.router_settings)?;
    unhonored(config).map_or(Ok(()), |setting| Err(Refused::Setting(setting)))
}

fn user_code(config: &Config) -> Option<UserCode> {
    let litellm = &config.litellm_settings;
    let callbacks = [
        &litellm.callbacks,
        &litellm.success_callback,
        &litellm.failure_callback,
    ];
    [
        (
            UserCode::Auth,
            config.general_settings.custom_auth.is_some(),
        ),
        (
            UserCode::Callback,
            callbacks
                .into_iter()
                .flatten()
                .flatten()
                .any(is_import_path),
        ),
        (
            UserCode::RouterPlugin,
            config
                .router_settings
                .additional_fields
                .contains_key("plugins"),
        ),
        (
            UserCode::PostCallRule,
            litellm.additional_fields.contains_key("post_call_rules"),
        ),
    ]
    .into_iter()
    .find_map(|(kind, loaded)| loaded.then_some(kind))
}

/// A callback the proxy imports, where one it ships is named without a dot.
fn is_import_path(callback: &Value) -> bool {
    callback.as_str().is_some_and(|name| name.contains('.'))
}

fn routing_strategy(router: &RouterSettings) -> Result<(), Refused> {
    let of_groups = router.routing_groups.iter().flatten().map(|group| {
        group
            .get("routing_strategy")
            .and_then(Value::as_str)
            .unwrap_or_default()
    });
    router
        .routing_strategy
        .as_deref()
        .into_iter()
        .chain(of_groups)
        .find(|strategy| *strategy != SIMPLE_SHUFFLE)
        .map_or(Ok(()), |strategy| {
            Err(Refused::RoutingStrategy(strategy.into()))
        })
}

trait IsSet {
    fn is_set(&self) -> bool;
}

impl IsSet for bool {
    fn is_set(&self) -> bool {
        *self
    }
}

impl<T> IsSet for Option<T> {
    fn is_set(&self) -> bool {
        self.is_some()
    }
}

impl<T> IsSet for Box<[T]> {
    fn is_set(&self) -> bool {
        !self.is_empty()
    }
}

impl<K, V> IsSet for BTreeMap<K, V> {
    fn is_set(&self) -> bool {
        !self.is_empty()
    }
}

impl IsSet for Object {
    fn is_set(&self) -> bool {
        !self.is_empty()
    }
}

/// The first of `fields` that is set, named as the configuration spells it.
macro_rules! first_set {
    ($section:literal, [$($field:ident),+ $(,)?]) => {
        [$((concat!($section, stringify!($field)), IsSet::is_set($field))),+]
            .into_iter()
            .find_map(|(name, set)| set.then(|| Box::from(name)))
    };
}

fn unknown(section: &str, fields: &AdditionalFields) -> Option<Box<str>> {
    fields
        .keys()
        .next()
        .map(|key| format!("{section}{key}").into())
}

/// Each struct is taken apart field by field, so a setting added to the configuration does not
/// compile here until it is either honored or listed as refused.
fn unhonored(config: &Config) -> Option<Box<str>> {
    let Config {
        model_list: _,
        general_settings,
        router_settings,
        litellm_settings,
        environment_variables: _,
        include: _,
        callback_settings,
        assistant_settings,
        default_vertex_config,
        mcp_servers,
        credential_list,
        guardrails,
        prompts,
        sandbox_tools,
        search_tools,
        files_settings,
        finetune_settings,
        mcp_tools,
        vector_store_registry,
        worker_registry,
        agents,
        agent_list,
        policies,
        policy_attachments,
        additional_fields,
    } = config;
    general(general_settings)
        .or_else(|| router(router_settings))
        .or_else(|| litellm(litellm_settings))
        .or_else(|| {
            first_set!(
                "",
                [
                    callback_settings,
                    assistant_settings,
                    default_vertex_config,
                    mcp_servers,
                    credential_list,
                    guardrails,
                    prompts,
                    sandbox_tools,
                    search_tools,
                    files_settings,
                    finetune_settings,
                    mcp_tools,
                    vector_store_registry,
                    worker_registry,
                    agents,
                    agent_list,
                    policies,
                    policy_attachments,
                ]
            )
        })
        .or_else(|| unknown("", additional_fields))
}

fn general(settings: &GeneralSettings) -> Option<Box<str>> {
    let GeneralSettings {
        master_key: _,
        litellm_license: _,
        custom_auth: _,
        admission_queue_timeout_seconds: _,
        database_connection_pool_limit: _,
        database_connection_timeout: _,
        database_connect_timeout: _,
        database_socket_timeout: _,
        database_max_idle_connection_lifetime: _,
        database_connection_pool_timeout: _,
        database_statement_timeout: _,
        database_lock_timeout: _,
        database_idle_in_transaction_session_timeout: _,
        database_disable_prepared_statements: _,
        proxy_config_reload_interval_seconds: _,
        health_check_interval: _,
        health_check_concurrency: _,
        enable_public_model_hub: _,
        mcp_allowed_hosts: _,
        mcp_allowed_origins: _,
        enable_jwt_auth: _,
        enable_oauth2_auth: _,
        enable_oauth2_proxy_auth: _,
        public_routes: _,
        litellm_jwtauth: _,
        completion_model,
        max_in_flight_requests_per_worker,
        max_queued_requests_per_worker,
        database_url,
        tracing,
        max_parallel_requests,
        global_max_parallel_requests,
        max_request_size_mb,
        max_response_size_mb,
        background_health_checks,
        store_model_in_db,
        forward_client_headers_to_llm_api,
        cancel_on_disconnect,
        infer_model_from_keys,
        dangerously_permit_weak_or_unset_master_key,
        plugins,
        coordination_redis,
        pass_through_endpoints,
        allow_user_auth,
        allow_requests_on_db_unavailable,
        allowed_ips,
        allowed_routes,
        litellm_key_header_name,
        role_permissions,
        enforce_rbac,
        user_header_name,
        user_header_mappings,
        additional_fields,
    } = settings;
    first_set!(
        "general_settings.",
        [
            completion_model,
            max_in_flight_requests_per_worker,
            max_queued_requests_per_worker,
            database_url,
            tracing,
            max_parallel_requests,
            global_max_parallel_requests,
            max_request_size_mb,
            max_response_size_mb,
            background_health_checks,
            store_model_in_db,
            forward_client_headers_to_llm_api,
            cancel_on_disconnect,
            infer_model_from_keys,
            dangerously_permit_weak_or_unset_master_key,
            plugins,
            coordination_redis,
            pass_through_endpoints,
            allow_user_auth,
            allow_requests_on_db_unavailable,
            allowed_ips,
            allowed_routes,
            litellm_key_header_name,
            role_permissions,
            enforce_rbac,
            user_header_name,
            user_header_mappings,
        ]
    )
    .or_else(|| unknown("general_settings.", additional_fields))
}

fn router(settings: &RouterSettings) -> Option<Box<str>> {
    let RouterSettings {
        routing_strategy: _,
        routing_groups: _,
        retry_policy: _,
        model_group_retry_policy: _,
        allowed_fails: _,
        cooldown_time: _,
        num_retries: _,
        timeout: _,
        retry_after: _,
        fallbacks: _,
        context_window_fallbacks: _,
        routing_strategy_args,
        model_group_affinity_config,
        max_retries,
        model_group_alias,
        enable_tag_filtering,
        weights,
        tag_routing_prefix,
        optional_pre_call_checks,
        additional_fields,
    } = settings;
    first_set!(
        "router_settings.",
        [
            routing_strategy_args,
            model_group_affinity_config,
            max_retries,
            model_group_alias,
            enable_tag_filtering,
            weights,
            tag_routing_prefix,
            optional_pre_call_checks,
        ]
    )
    .or_else(|| unknown("router_settings.", additional_fields))
}

fn litellm(settings: &LiteLlmSettings) -> Option<Box<str>> {
    let LiteLlmSettings {
        ssl_verify: _,
        ssl_certificate: _,
        ssl_security_level: _,
        ssl_ecdh_curve: _,
        force_ipv4: _,
        http2: _,
        aiohttp_trust_env: _,
        disable_aiohttp_trust_env: _,
        disable_aiohttp_transport: _,
        drop_params: _,
        json_logs: _,
        set_verbose: _,
        request_timeout,
        num_retries,
        cache,
        cache_params,
        callbacks,
        success_callback,
        failure_callback,
        additional_fields,
    } = settings;
    first_set!(
        "litellm_settings.",
        [
            request_timeout,
            num_retries,
            cache,
            cache_params,
            callbacks,
            success_callback,
            failure_callback,
        ]
    )
    .or_else(|| unknown("litellm_settings.", additional_fields))
}
