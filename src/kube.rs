use std::process::{Command, Stdio};
use std::fs::File;

use std::collections::HashMap;
use std::str;

use crate::utils;

#[derive(Debug)]
pub struct LogRecorderConfig {
    namespace: String,
}

#[derive(Debug)]
pub struct PodInfo {
    name: String,
    container: String,
}


impl LogRecorderConfig {
    pub fn new(namespace: String) -> LogRecorderConfig {
        LogRecorderConfig {
            namespace: namespace,
        }
    }
}


// Returns a Hashmap of Log FilePath, PodInfo <Returns Hashmap <String Podinfo>
pub fn run_logs(log_options: &LogRecorderConfig)  {
    let pod_vec = get_all_pod_info(&log_options.namespace);
    let pod_hashmap = generate_hashmap(pod_vec);
    run_cmd(pod_hashmap, &log_options.namespace);
}


fn run_cmd(pod_hashmap: HashMap<String, PodInfo>, namespace:&str) {
    let mut kube_cmd = Command::new("kubectl");
    let last_element_count = pod_hashmap.len();
    let mut count = 0;
    for (k, v) in pod_hashmap.iter() {
        let container = get_app_container(&v.container);
        let out_file = File::create(&k).unwrap();
        if count != last_element_count && count != 0 {
            kube_cmd.arg("&");
            kube_cmd.arg("kubectl");
        }
        
        let deploy_string = "deployment/".to_string() + &container;
        kube_cmd.arg("--kubeconfig");
        kube_cmd.arg("kube.config");
        kube_cmd.arg("logs");
        kube_cmd.arg(&deploy_string);
        kube_cmd.arg(&container);
        kube_cmd.arg("-n");
        kube_cmd.arg(&namespace);
        kube_cmd.stdout(Stdio::from(out_file));

        count += 1
    }

    let output = kube_cmd.output().expect("Couldn't run command");
}

fn get_app_container(containers: &str) -> String {
    // Need to split the whitespaces, and get the first container name only (not worried about
    // istio resources right now)
    let container = containers.split_whitespace();
    let container_vec: Vec<&str> = container.collect();
    return container_vec[0].to_string()
}

fn get_all_pod_info(namespace: &str) -> Vec<String>  {
    let output = Command::new("kubectl")
        .args(&["--kubeconfig", "kube.config"])
        .args(&["get", "pods"])
        .args(&["-n", &namespace])
        .args(&["-o", "jsonpath={range .items[*]}{.metadata.name} {.spec['containers', 'initContainers'][*].name}\n{end}"])
        .output()
        .expect("Failed to get kubernetes pods");

    // if error handle it here
    // if output.stderr handle
    let pods = str::from_utf8(&output.stdout).unwrap();
    let pods_vec: Vec<&str> = pods.split("\n").collect();
    utils::str_to_string(pods_vec)
}

fn generate_hashmap(pod_vec: Vec<String>) -> HashMap<String, PodInfo>  {
    let mut pods_hashmap = HashMap::new();
    for info in pod_vec {
        // Empty namespaces happen for some reason.  Breaks the indices
        if info == "" {
            continue;
        }

        // explode on whitespace to seperate concerns
        let pod_info = info.split_whitespace();
        let mut pod_info_vec: Vec<&str> = pod_info.collect();
        let single_pod_vec = pod_info_vec.split_off(0);

        // fix the name split and container split
        let string_vec = utils::str_to_string(single_pod_vec);
        let file_path = "/tmp/".to_owned() + &string_vec[0] + ".txt";

        let name = &string_vec[0];
        let containers = &string_vec[1];

        pods_hashmap.insert(
            file_path,
            PodInfo{
                name: name.to_string(),
                container: containers.to_string(),
            }
        );
    };

    pods_hashmap
}
