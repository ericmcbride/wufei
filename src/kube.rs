use crate::utils;
use colored::*;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::str;
use std::thread;

/// Static string to hold values we want to use to differentiate between pod logs.  These colors
/// are mapped from the colored cargo crate
static COLOR_VEC: &'static [&str] = &[
    "green",
    "red",
    "yellow",
    "blue",
    "cyan",
    "magenta",
    "white",
    "bright black",
    "bright red",
    "bright green",
    "bright yellow",
    "bright blue",
    "bright magenta",
    "bright cyan",
];


/// Config built from cli-args
#[derive(Debug)]
pub struct LogRecorderConfig {
    namespace: String,
    kube_config: String,
    file: bool,
    color: bool,
}

/// Pod infromation
#[derive(Debug, Clone)]
pub struct PodInfo {
    name: String,
    container: String,
}

impl LogRecorderConfig {
    pub fn new(
        namespace: String,
        kube_config: String,
        file: bool,
        color: bool,
    ) -> LogRecorderConfig {
        LogRecorderConfig {
            namespace: namespace,
            kube_config: kube_config,
            file: file,
            color: color,
        }
    }
}

/// Entrypoint for the tailing of the logs
pub fn run_logs(log_options: &LogRecorderConfig) -> Result<(), Box<::std::error::Error>> {
    let pod_vec = get_all_pod_info(&log_options.namespace, &log_options.kube_config)?;
    let pod_hashmap = generate_hashmap(pod_vec)?;
    run_cmd(pod_hashmap, &log_options);
    Ok(())
}


///  Kicks off the concurrent logging
fn run_cmd(pod_hashmap: HashMap<String, PodInfo>, log_options: &LogRecorderConfig) {
    let mut children = vec![];
    for (k, v) in pod_hashmap {
        let namespace = log_options.namespace.clone();
        let kube_config = log_options.kube_config.clone();
        let file = log_options.file.clone();
        let color = log_options.color.clone();
        children.push(thread::spawn(move || {
            run_individual(
                k.to_string(),
                &v,
                namespace.to_owned(),
                kube_config.to_owned(),
                file.to_owned(),
                color.to_owned(),
            )
        }));
    }

    let _: Vec<_> = children.into_iter().map(|thread| thread.join()).collect();
}


/// Each thread runs this function.   It gathers the individual logs at a thread level (pod
/// level in this case).  It does all the filtering of the cli args, spins off a background
/// process to tail the logs, and buffers the terminal output, allowing the each thread to print
/// out to stdout in a non-blocking fashion.  If an error happens, instead of handling using
/// channels, we are just writing the stderr into the output file if the flag exists.  If not the
/// thread (or pod) buffer will not be outputted to stdout
fn run_individual(
    k: String,
    v: &PodInfo,
    namespace: String,
    kube_config: String,
    file: bool,
    color_on: bool,
) {
    let mut kube_cmd = Command::new("kubectl");
    let container = get_app_container(&v.container);
    let deploy_string = "deployment/".to_string() + &container;

    if kube_config.len() != 0 {
        kube_cmd.arg("--kubeconfig");
        kube_cmd.arg(&kube_config);
    }

    kube_cmd.arg("logs");
    kube_cmd.arg("-f");
    kube_cmd.arg(&deploy_string);
    kube_cmd.arg(&container);
    kube_cmd.arg("-n");
    kube_cmd.arg(&namespace);
    kube_cmd.stdout(Stdio::piped());

    let output = kube_cmd
        .spawn()
        .unwrap()
        .stdout
        .ok_or_else(|| "Unable to follow kube logs")
        .unwrap();

    let reader = BufReader::new(output);
    let mut log_prefix = "[pod] ".to_string();
    log_prefix = log_prefix + "[" + &v.name + "]";

    if color_on {
        let color = COLOR_VEC.choose(&mut rand::thread_rng()); // get random color
        let str_color = color.unwrap().to_string(); // unwrap random
        log_prefix = log_prefix.color(str_color).to_string();
    }

    if file {
        let mut out_file = File::create(&k.to_string()).unwrap();
        reader
            .lines()
            .filter_map(|line| line.ok())
            .for_each(|line| {
                println!("{}: {}", &log_prefix, line);
                out_file.write(&line.as_bytes()).unwrap();
            });
    } else {
        reader
            .lines()
            .filter_map(|line| line.ok())
            .for_each(|line| println!("{}: {}", &log_prefix, line));
    }
}


/// Gets the container for the app.  Helps with the gathering of logs by using the deployment -
/// container log strategy instead of the pods.  If we were doing the kubernetes pod logging
/// strategy, we could run into issues if someone was using linkerd or istio, since envoy
/// sidecars are present.
fn get_app_container(containers: &str) -> String {
    let container = containers.split_whitespace();
    let container_vec: Vec<&str> = container.collect();
    container_vec[0].to_string()
}


/// Gather all information about the pods currently deployed in the users kubernetes cluster
fn get_all_pod_info(
    namespace: &str,
    kube_config: &str,
) -> Result<Vec<String>, Box<::std::error::Error>> {
    let output = Command::new("kubectl")
        .args(&["--kubeconfig", &kube_config])
        .args(&["get", "pods"])
        .args(&["-n", &namespace])
        .args(&["-o", "jsonpath={range .items[*]}{.metadata.name} {.spec['containers', 'initContainers'][*].name}\n{end}"])
        .output()
        .expect("Failed to get kubernetes pods");

    if output.stderr.len() != 0 {
        let byte_string = String::from_utf8_lossy(&output.stderr);
        utils::generate_err(byte_string.to_string())?
    }

    let pods = str::from_utf8(&output.stdout)?;
    let pods_vec: Vec<&str> = pods.split("\n").collect();
    Ok(utils::str_to_string(pods_vec))
}

/// We generate a hashmap with the key being the file_path (easy access to write files), and the
/// value being a PodInfo struct
fn generate_hashmap(
    pod_vec: Vec<String>,
) -> Result<HashMap<String, PodInfo>, Box<::std::error::Error>> {
    let mut pods_hashmap = HashMap::new();
    // #TODO: Probably should move this out of this function in the future
    fs::create_dir_all("/tmp/wufei")?;
    for info in pod_vec {
        if info == "" {
            continue;
        }

        let pod_info = info.split_whitespace();
        let mut pod_info_vec: Vec<&str> = pod_info.collect();
        let single_pod_vec = pod_info_vec.split_off(0);

        let string_vec = utils::str_to_string(single_pod_vec);
        let file_path = "/tmp/wufei/".to_owned() + &string_vec[0] + ".txt";

        let name = &string_vec[0];
        let containers = &string_vec[1];

        pods_hashmap.insert(
            file_path,
            PodInfo {
                name: name.to_string(),
                container: containers.to_string(),
            },
        );
    }

    Ok(pods_hashmap)
}
