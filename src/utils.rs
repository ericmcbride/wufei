use clap::ArgMatches;
use crate::kube;



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
    Ok(kube::LogRecorderConfig::new(
        namespace.to_string(),
    ))
}
