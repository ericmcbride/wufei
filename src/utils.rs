use crate::kube;
use clap::ArgMatches;

pub fn str_to_string(input: Vec<&str>) -> Vec<String> {
    // TODO MAKE FUNCTIONAL INSTAED OF IMPERATIVE
    let mut string_vec = Vec::new();
    for x in input {
        string_vec.push(x.to_owned());
    }
    string_vec
}

pub fn set_args(args: &ArgMatches) -> Result<kube::LogRecorderConfig, Box<::std::error::Error>> {
    let namespace = args.value_of("NAMESPACE").unwrap();
    let kube_config = if let Some(kube_config) = args.value_of("KUBECONFIG") {
        let new_config = kube_config;
        new_config
    } else {
        ""
    };
    let file = if let Some(_) = args.value_of("FILE") {
        true
    } else {
        false // if not passed set to false
    };

    Ok(kube::LogRecorderConfig::new(
        namespace.to_string(),
        kube_config.to_string(),
        file,
    ))
}
