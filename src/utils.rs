use crate::kube;
use clap::ArgMatches;
use std::io::{Error, ErrorKind};


/// Parse the cli args, and return the kube::LogRecorderConfig
pub fn set_args(args: &ArgMatches) -> Result<kube::LogRecorderConfig, Box<dyn ::std::error::Error>> {
    let namespace = args.value_of("NAMESPACE").unwrap().to_string();
    let kube_config = if let Some(kube_config) = args.value_of("KUBECONFIG") {
        kube_config
    } else {
        ""
    };

    let mut outfile = "";
    let file = args.is_present("FILE");
    if file {
        outfile = if let Some(o) = args.value_of("OUTFILE") {
            o
        } else {
            "/tmp/wufei/"
        }
    }

    let color = args.is_present("COLOR");
    Ok(kube::LogRecorderConfig::new(
        namespace.to_string(),
        kube_config.to_string(),
        file,
        color,
        outfile.to_string(),
    ))
}

/// Since Command returns stdout or stderr attrs instead of actual errors, we need a helper
/// function to generate custom errors when dealing with Command.
pub fn generate_err(err_msg: String) -> Result<(), Box<dyn ::std::error::Error>> {
    Err(Box::new(Error::new(ErrorKind::Other, err_msg)))
}
