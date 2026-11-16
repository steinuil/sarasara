mod program;
mod signature;

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};
use clap::Parser;
use reqwest::StatusCode;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt as _, util::SubscriberInitExt as _};
use url::Url;

/// An RSS proxy for RaiPlay Sound.
///
/// Transforms the JSON returned by RaiPlay Sound's /programming/{program_name}.json API
/// into RSS fit for consumption by a podcast client, and proxies audio requests, removing
/// same-site headers.
///
/// The audio URLs in the RSS feed are signed using the secret provided in the
/// SARASARA_PROXY_SECRET env variable to prevent bad actors from using this endpoint
/// as a public proxy.
#[derive(Parser)]
struct Options {
    #[clap(long, default_value = "0.0.0.0:8080")]
    pub bind_address: String,

    /// Public base URL of the server, used for the audio proxy.
    #[clap(long)]
    pub public_url: Url,

    /// URL of RaiPlay Sound.
    ///
    /// You probably don't need to modify this.
    #[clap(long, default_value = "https://www.raiplaysound.it")]
    pub raiplaysound_url: Url,

    /// Hex secret used to sign the audio URLs to proxy.
    ///
    /// Should be at least 32 bytes. You can generate it with `openssl rand -hex 32`.
    ///
    /// It should not change between runs, or the URLs cached by podcast clients will become invalid!
    #[clap(long, env = "SARASARA_PROXY_SECRET", hide_env_values = true, value_parser = signature::parse)]
    pub proxy_secret: signature::Secret,

    /// Value of the User-Agent header used by the audio proxy.
    #[clap(long, default_value = "Firefox")]
    pub proxy_user_agent: String,
}

#[derive(Clone)]
struct Params {
    pub raiplaysound_url: Url,
    pub public_url: Url,
    pub proxy_secret: signature::Secret,
    pub proxy_user_agent: String,
}

pub struct Rss(pub String);

impl IntoResponse for Rss {
    fn into_response(self) -> axum::response::Response {
        (
            axum::response::AppendHeaders([(http::header::CONTENT_TYPE, "application/xml")]),
            self.0,
        )
            .into_response()
    }
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("not found")]
    NotFound,

    #[error("upstream: {0}")]
    Upstream(#[from] reqwest::Error),

    #[error("invalid signature")]
    InvalidSignature,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::NotFound => StatusCode::NOT_FOUND.into_response(),
            AppError::Upstream(e) => {
                tracing::error!("upstream error: {e}");
                StatusCode::BAD_GATEWAY.into_response()
            }
            AppError::InvalidSignature => StatusCode::FORBIDDEN.into_response(),
        }
    }
}

async fn proxy_rss(
    State(params): State<Arc<Params>>,
    Path(program_name): Path<String>,
) -> Result<Rss, AppError> {
    let mut url = params.raiplaysound_url.clone();
    url.set_path(&format!("programmi/{program_name}.json"));

    let resp = reqwest::get(url).await?;
    if resp.status() == StatusCode::NOT_FOUND {
        return Err(AppError::NotFound);
    }

    let program: program::RaiPlayProgram = resp.error_for_status()?.json().await?;

    Ok(Rss(program.into_rss(&params).to_string()))
}

#[derive(Debug, Clone, serde::Deserialize)]
struct AudioQueryParams {
    pub url: String,
    pub sig: String,
}

const MAX_REDIRECTS: usize = 3;
const BAD_HEADERS: [&str; 4] = ["host", "origin", "src-fetch-mode", "src-fetch-site"];

async fn proxy_audio(
    State(params): State<Arc<Params>>,
    Query(args): Query<AudioQueryParams>,
) -> Result<Response, AppError> {
    if !signature::verify(&params.proxy_secret, &args.url, &args.sig) {
        return Err(AppError::InvalidSignature);
    }

    let resp = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
        .user_agent(&params.proxy_user_agent)
        .build()?
        .get(&args.url)
        .send()
        .await?
        .error_for_status()?;

    let mut resp_out = Response::builder().status(StatusCode::OK);

    for (key, val) in resp
        .headers()
        .iter()
        .filter(|(key, _)| !BAD_HEADERS.contains(&key.as_str()))
    {
        resp_out = resp_out.header(key, val);
    }

    Ok(resp_out
        .body(axum::body::Body::from_stream(resp.bytes_stream()))
        .expect("valid response"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("crypto provider already installed");

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug,tower_http=debug,axum::rejection=trace,reqwest=debug,hyper_util=debug,hyper=debug".into()))
        .with(tracing_subscriber::fmt::layer().with_ansi(false).without_time())
        .init();

    let opts = Options::parse();

    let app = axum::Router::new()
        .route("/programmi/{program}", axum::routing::get(proxy_rss))
        .route("/audio", axum::routing::get(proxy_audio))
        .with_state(Arc::new(Params {
            raiplaysound_url: opts.raiplaysound_url,
            public_url: {
                let mut url = opts.public_url;
                let path = url.path().to_string();
                url.set_path(path.trim_suffix("/"));
                url
            },
            proxy_secret: opts.proxy_secret,
            proxy_user_agent: opts.proxy_user_agent,
        }))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(opts.bind_address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
