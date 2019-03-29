#[macro_use]
extern crate clap;

mod kube;
mod utils;

fn main() {
    match run() {
        Ok(log_config) => {
            kube::run_logs(&log_config);
        }
        Err(e) => {
            eprintln!("Error {}", e);
        }
    }
}

fn run() -> Result<(kube::LogRecorderConfig), Box<::std::error::Error>> {
    let args = clap_app!(wufei =>
        (version: "1.0")
        (author: "Eric McBride <ericmcbridedeveloper@gmail.com>")
        (about: "View All Logs from Kubernetes Namespace")
        (@arg NAMESPACE: -n --namespace +required +takes_value "Namespace for logs")
        (@arg KUBECONFIG: -k --kubeconfig +takes_value "Kube config file if not using context")
        (@arg FILE: -f --file "Write logs to files based on pod name /tmp/podname")
        (@arg COLOR: --color "Show colored output")
    )
    .get_matches();

    let log_recorder = utils::set_args(&args);
    match log_recorder {
        Ok(log_recorder) => Ok(log_recorder),
        Err(log_recorder) => Err(log_recorder),
    }
}
