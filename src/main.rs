mod kube;
mod utils;

/// Main Entrypoint into the code
#[tokio::main]
async fn main() -> Result<(), Box<dyn::std::error::Error>> {
    let config = kube::generate_config()?;
    // add option to run the pod updater
    let async_config = config.clone();
    tokio::task::spawn(async move {
        println!("Starting Async Kube Informer");
        kube::pod_informer(&async_config).await.unwrap();
    });
    
    let _ = kube::run_logs(&config)?;
    Ok(())
}
