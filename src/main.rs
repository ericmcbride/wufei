mod kube;

/// Main Entrypoint into the code
#[tokio::main]
async fn main() -> Result<(), Box<dyn ::std::error::Error>> {
    kube::CONFIG.set(kube::generate_config()).unwrap();
    kube::KUBE_CLIENT.set(kube::create_kube_client().await);
    // if informer is called, then spawn a new tokio task
    if kube::LogRecorderConfig::global().update {
        tokio::task::spawn(async move {
            println!("Starting Async Kube Informer");
            kube::pod_informer().await.unwrap();
        });
    }
    kube::run_logs().await.unwrap();
    Ok(())
}
