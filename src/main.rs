extern crate structopt;

mod kube;
mod utils;

/// Main Entrypoint into the code
fn main() {
    match kube::run() {
        Ok(log_config) => match kube::run_logs(&log_config) {
            Ok(_) => {
                println!("Log files are found at /tmp/<podname>");
            }
            Err(e) => {
                eprintln!("Error {}", e);
            }
        },
        Err(e) => {
            eprintln!("Error {}", e);
        }
    }
}
