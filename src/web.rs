use crate::{
    issue, prompt,
    settings::{Config, Locale},
};
use anyhow::{Context, Result, bail};
use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Path as RoutePath, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use subtle::ConstantTimeEq;
use tokio::net::TcpListener;
use zeroize::Zeroizing;

const SESSION_COOKIE: &str = "ALIWEB";

#[derive(Clone)]
struct WebState {
    config: Arc<Config>,
    locale: Locale,
    language: String,
    origin: String,
    bootstrap: Arc<Zeroizing<String>>,
    session: Arc<Zeroizing<String>>,
    bootstrap_used: Arc<AtomicBool>,
    session_started: Instant,
    index: Arc<String>,
    script: Arc<String>,
    stylesheet: Arc<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PrepareRequest {
    url: String,
    agent_id: Option<String>,
}

#[derive(Serialize)]
struct AgentOption {
    id: String,
    label: String,
}

#[derive(Serialize)]
struct PrepareResponse {
    prompt: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

pub async fn run(config_dir: PathBuf, language: Option<&str>) -> Result<()> {
    let config = Config::load(&config_dir)?;
    if !config.web.enabled {
        bail!("local web interface is disabled by config/runtime.toml");
    }
    let language = language
        .unwrap_or(&config.product.default_locale)
        .to_owned();
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
    let port = listener.local_addr()?.port();
    let origin = format!("http://127.0.0.1:{port}");
    let (router, capability) = build_router(&config_dir, config, &language, origin)?;
    webbrowser::open(capability.as_str()).context("could not open the local web interface")?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

pub fn build_router(
    config_dir: &std::path::Path,
    config: Config,
    language: &str,
    origin: String,
) -> Result<(Router, Zeroizing<String>)> {
    let locale = config.locale(language)?;
    let mut bootstrap_bytes = [0u8; 32];
    let mut session_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bootstrap_bytes);
    OsRng.fill_bytes(&mut session_bytes);
    let index_html = read_asset(config_dir, language, "index.html")?;
    let app_script = read_asset(config_dir, language, "app.js")?;
    let style_css = read_asset(config_dir, language, "style.css")?;
    let capability = Zeroizing::new(hex(&bootstrap_bytes));
    let capability_url = Zeroizing::new(format!("{origin}/bootstrap/{}", capability.as_str()));
    let state = WebState {
        config: Arc::new(config),
        locale,
        language: language.to_owned(),
        origin,
        bootstrap: Arc::new(Zeroizing::new(capability.to_string())),
        session: Arc::new(Zeroizing::new(hex(&session_bytes))),
        bootstrap_used: Arc::new(AtomicBool::new(false)),
        session_started: Instant::now(),
        index: Arc::new(index_html),
        script: Arc::new(app_script),
        stylesheet: Arc::new(style_css),
    };
    bootstrap_bytes.fill(0);
    session_bytes.fill(0);
    let router = Router::new()
        .route("/bootstrap/{capability}", get(bootstrap))
        .route("/app", get(index))
        .route("/assets/app.js", get(script))
        .route("/assets/style.css", get(stylesheet))
        .route("/api/agents", get(agents))
        .route("/api/prepare", axum::routing::post(prepare))
        .layer(DefaultBodyLimit::max(state.config.web.max_request_bytes))
        .layer(middleware::from_fn(security_middleware))
        .with_state(state);
    Ok((router, capability_url))
}

fn read_asset(root: &std::path::Path, locale: &str, file: &str) -> Result<String> {
    let path = root.join("web").join(locale).join(file);
    let metadata = std::fs::symlink_metadata(&path)
        .with_context(|| format!("cannot inspect web asset {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 1_048_576 {
        bail!("web assets must be regular files within the configured size limit");
    }
    std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read web asset {}", path.display()))
}

async fn bootstrap(
    State(state): State<WebState>,
    RoutePath(capability): RoutePath<String>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    verify_host(&headers, &state)?;
    let valid = state
        .bootstrap
        .as_bytes()
        .ct_eq(capability.as_bytes())
        .unwrap_u8()
        == 1;
    if !valid
        || state
            .bootstrap_used
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let cookie = format!(
        "{SESSION_COOKIE}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
        state.session.as_str(),
        state.config.web.session_seconds
    );
    let mut response = Redirect::to("/app").into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    secure_headers(response.headers_mut());
    Ok(response)
}

async fn index(State(state): State<WebState>, headers: HeaderMap) -> Result<Response, StatusCode> {
    authorize(&headers, &state, false)?;
    let mut response = Html(state.index.as_str().to_owned()).into_response();
    secure_headers(response.headers_mut());
    Ok(response)
}

async fn script(State(state): State<WebState>, headers: HeaderMap) -> Result<Response, StatusCode> {
    authorize(&headers, &state, false)?;
    let mut response = Response::new(Body::from(state.script.as_str().to_owned()));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/javascript; charset=utf-8"),
    );
    secure_headers(response.headers_mut());
    Ok(response)
}

async fn stylesheet(
    State(state): State<WebState>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    authorize(&headers, &state, false)?;
    let mut response = Response::new(Body::from(state.stylesheet.as_str().to_owned()));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/css; charset=utf-8"),
    );
    secure_headers(response.headers_mut());
    Ok(response)
}

async fn agents(
    State(state): State<WebState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AgentOption>>, StatusCode> {
    authorize(&headers, &state, false)?;
    let options = state
        .config
        .agents
        .iter()
        .map(|agent| {
            Ok(AgentOption {
                id: agent.id.clone(),
                label: agent
                    .labels
                    .get(&state.language)
                    .context("validated agent locale is missing")?
                    .clone(),
            })
        })
        .collect::<Result<Vec<_>>>()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(options))
}

async fn prepare(
    State(state): State<WebState>,
    headers: HeaderMap,
    Json(request): Json<PrepareRequest>,
) -> Result<Json<PrepareResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize(&headers, &state, true).map_err(|status| {
        (
            status,
            Json(ErrorResponse {
                error: state.message("web_request_rejected"),
            }),
        )
    })?;
    if request.url.len() > 2048 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: state.message("web_invalid_request"),
            }),
        ));
    }
    if request
        .agent_id
        .as_ref()
        .is_some_and(|id| !state.config.agents.iter().any(|agent| agent.id == *id))
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: state.message("web_invalid_request"),
            }),
        ));
    }
    let issue = issue::fetch(&request.url, &state.config)
        .await
        .map_err(|_| {
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: state.message("web_issue_failed"),
                }),
            )
        })?;
    let prompt = prompt::build_for_agent(
        &issue,
        &state.config,
        &state.language,
        request.agent_id.as_deref(),
    )
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: state.message("web_prompt_failed"),
            }),
        )
    })?;
    Ok(Json(PrepareResponse { prompt }))
}

impl WebState {
    fn message(&self, key: &str) -> String {
        self.locale
            .message(key)
            .expect("web locale messages are validated during configuration load")
            .to_owned()
    }
}

fn authorize(
    headers: &HeaderMap,
    state: &WebState,
    require_origin: bool,
) -> Result<(), StatusCode> {
    verify_host(headers, state)?;
    let Some(cookie) = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
    else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let token = cookie
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find_map(|(name, value)| (name == SESSION_COOKIE).then_some(value));
    let valid = token
        .is_some_and(|token| state.session.as_bytes().ct_eq(token.as_bytes()).unwrap_u8() == 1);
    if !valid {
        return Err(StatusCode::UNAUTHORIZED);
    }
    if state.session_started.elapsed().as_secs() > state.config.web.session_seconds {
        return Err(StatusCode::UNAUTHORIZED);
    }
    if require_origin {
        if headers
            .get(header::ORIGIN)
            .and_then(|value| value.to_str().ok())
            != Some(state.origin.as_str())
        {
            return Err(StatusCode::FORBIDDEN);
        }
    } else if headers
        .get(header::ORIGIN)
        .is_some_and(|value| value.to_str().ok() != Some(state.origin.as_str()))
    {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

fn verify_host(headers: &HeaderMap, state: &WebState) -> Result<(), StatusCode> {
    let expected = state
        .origin
        .strip_prefix("http://")
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    if headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        != Some(expected)
    {
        return Err(StatusCode::MISDIRECTED_REQUEST);
    }
    Ok(())
}

async fn security_middleware(request: axum::http::Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    secure_headers(response.headers_mut());
    response
}

fn secure_headers(headers: &mut HeaderMap) {
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data:; base-uri 'none'; form-action 'self'; frame-ancestors 'none'"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("clipboard-write=(self)"),
    );
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}
