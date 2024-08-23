use dkn_compute::{serve_metrics_server, setup_tracing, DriaComputeNode, DriaComputeNodeConfig};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Err(e) = dotenvy::dotenv() {
        log::warn!("Could not load .env file: {}", e);
    }

    // tracing_subscriber::fmt()
    //     .with_env_filter(EnvFilter::from_default_env())
    //     .init();
    setup_tracing()?;
    log::info!(
        "Initializing Dria Compute Node (version {})",
        dkn_compute::DRIA_COMPUTE_NODE_VERSION
    );

    // create configurations & check required services
    let config = DriaComputeNodeConfig::new();
    if let Err(err) = config.check_services().await {
        log::error!("Error checking services: {}", err);
        panic!("Service check failed.")
    }

    let token = CancellationToken::new();

    // launch the node
    let node_token = token.clone();
    let node_handle = tokio::spawn(async move {
        match DriaComputeNode::new(config, node_token).await {
            Ok(mut node) => {
                if let Err(err) = node.launch().await {
                    log::error!("Node launch error: {}", err);
                    panic!("Node failed.")
                };

                tokio::spawn(serve_metrics_server(node.p2p.metric_registry));
            }
            Err(err) => {
                log::error!("Node setup error: {}", err);
                panic!("Could not setup node.")
            }
        }
    });

    // add cancellation check
    tokio::spawn(async move {
        #[cfg(feature = "profiling")]
        {
            const PROFILE_DURATION_SECS: u64 = 120;
            tokio::time::sleep(tokio::time::Duration::from_secs(PROFILE_DURATION_SECS)).await;
            token.cancel();
        }

        #[cfg(not(feature = "profiling"))]
        if let Err(err) = wait_for_termination(token.clone()).await {
            log::error!("Error waiting for termination: {}", err);
            log::error!("Cancelling due to unexpected error.");
            token.cancel();
        };
    });

    // wait for tasks to complete
    if let Err(err) = node_handle.await {
        log::error!("Node handle error: {}", err);
        panic!("Could not exit Node thread handle.");
    };

    Ok(())
}

/// Waits for SIGTERM or SIGINT, and cancels the given token when the signal is received.
#[allow(unused)]
async fn wait_for_termination(cancellation: CancellationToken) -> std::io::Result<()> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = signal(SignalKind::terminate())?; // Docker sends SIGTERM
    let mut sigint = signal(SignalKind::interrupt())?; // Ctrl+C sends SIGINT
    tokio::select! {
        _ = sigterm.recv() => log::warn!("Recieved SIGTERM"),
        _ = sigint.recv() => log::warn!("Recieved SIGINT"),
        _ = cancellation.cancelled() => {
            // no need to wait if cancelled anyways
            // although this is not likely to happen
            return Ok(());
        }
    };

    log::info!("Terminating the node...");
    cancellation.cancel();
    Ok(())
}
