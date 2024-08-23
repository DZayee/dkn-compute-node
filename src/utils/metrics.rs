use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use opentelemetry::KeyValue;
use prometheus_client::encoding::text::encode;
use prometheus_client::registry::Registry;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

const METRICS_CONTENT_TYPE: &str = "application/openmetrics-text;charset=utf-8;version=1.0.0";

pub async fn serve_metrics_server(registry: Registry) -> Result<(), std::io::Error> {
    // Serve on localhost.
    let addr: SocketAddr = ([127, 0, 0, 1], 0).into();
    let service = MetricService::new(registry);
    let server = Router::new()
        .route("/metrics", get(respond_with_metrics))
        .with_state(service);
    let tcp_listener = TcpListener::bind(addr).await?;
    let local_addr = tcp_listener.local_addr()?;

    log::info!("http://{}/metrics", local_addr);

    axum::serve(tcp_listener, server.into_make_service()).await?;
    Ok(())
}

#[derive(Clone)]
pub(crate) struct MetricService {
    reg: Arc<Mutex<Registry>>,
}

async fn respond_with_metrics(state: State<MetricService>) -> impl IntoResponse {
    let mut sink = String::new();
    let reg = state.get_reg();
    encode(&mut sink, &reg.lock().unwrap()).unwrap();

    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, METRICS_CONTENT_TYPE)],
        sink,
    )
}

type SharedRegistry = Arc<Mutex<Registry>>;

impl MetricService {
    fn new(registry: Registry) -> Self {
        Self {
            reg: Arc::new(Mutex::new(registry)),
        }
    }

    fn get_reg(&self) -> SharedRegistry {
        Arc::clone(&self.reg)
    }
}

pub fn setup_tracing() -> Result<(), Box<dyn Error>> {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .with_trace_config(opentelemetry_sdk::trace::Config::default().with_resource(
            opentelemetry_sdk::Resource::new(vec![KeyValue::new("service.name", "libp2p")]),
        ))
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    if let Err(e) = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(EnvFilter::from_default_env()))
        .with(
            tracing_opentelemetry::layer()
                .with_tracer(tracer)
                .with_filter(EnvFilter::from_default_env()),
        )
        .try_init()
    {
        log::warn!("Tracing initialization error: {}", e);
    }

    Ok(())
}
