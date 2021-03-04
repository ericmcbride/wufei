use colored::*;
use rand::seq::SliceRandom;

use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;

use std::str;

use std::{thread, time};
use std::collections::HashMap;
use structopt::StructOpt;

use kube_async::{
    api::v1Event,
    api::{Api, Informer, ListParams, LogParams, Object, WatchEvent},
    client::APIClient,
    config,
};

use futures::stream::StreamExt;
use k8s_openapi::api::core::v1::{PodSpec, PodStatus};
use once_cell::sync::OnceCell;
use serde_json::Value;
use futures::future;

type Pod = Object<PodSpec, PodStatus>;

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

pub static CONFIG: OnceCell<LogRecorderConfig> = OnceCell::new();
pub static KUBE_CLIENT: OnceCell<KubeClient> = OnceCell::new();

#[derive(Clone, Debug, StructOpt)]
#[structopt(
    name = "Wufei",
    about = "Tail ALL your kubernetes logs at once, or record them to files",
    author = "Eric McBride <ericmcbridedeveloper@gmail.com> github.com/ericmcbride"
)]
pub struct LogRecorderConfig {
    /// Namespace for logs
    #[structopt(short, long, default_value = "kube-system")]
    namespace: String,

    /// Record the logs to a file. Note: Logs will not appear in stdout.
    #[structopt(short, long)]
    file: bool,

    /// Outfile of where the logs are being recorded
    #[structopt(short, long, default_value = "/tmp/wufei/")]
    outfile: String,

    /// Pods for the logs will appear in color in your terminal
    #[structopt(long)]
    color: bool,

    /// Runs an informer, that will add new pods to the tailed logs
    #[structopt(long)]
    pub update: bool,

    /// Select pods by label example: version=v1
    #[structopt(long)]
    selector: Option<String>,

    /// Grab previous logs
    #[structopt(long)]
    previous: bool,

    /// Only return logs newer then a duration in seconds like 1, 60, or 180
    #[structopt(long)]
    since: Option<i64>,

    /// If set, the number of lines from the end of the logs to show.
    #[structopt(long, default_value = "1")]
    tail_lines: i64,

    /// key to search for in the json, prints out the value. Only single key supported
    #[structopt(long)]
    json_key: Option<String>,
    
    /// Dont follow the logs, but gather all of them at once
    #[structopt(long)]
    gather: bool,

    /// Select containers by name
    #[structopt(short, long)]
    container: Vec<String>,

    /// Select pods by name
    #[structopt(long)]
    pod: Vec<String>,

}

impl LogRecorderConfig {
    pub fn global() -> &'static LogRecorderConfig {
        CONFIG.get().expect("Config is not initialized")
    }
}

pub struct KubeClient {
    client: APIClient,
}

impl KubeClient {
    pub fn client() -> &'static KubeClient {
        KUBE_CLIENT.get().expect("Client not initialized")
    }
}

/// Pod infromation
#[derive(Clone, Debug, Default)]
pub struct PodInfo {
    name: String,
    container: String,
    file_name: String,
}

impl PodInfo {
    pub fn new(name: String, container: String, file_path: String) -> PodInfo {
        let full_file_path = file_path
            + &name
            + "-"
            + &container
            + ".txt";
        PodInfo {
            name: name,
            container: container,
            file_name: full_file_path,
        }
    }
}

/// Cli options for wufei
pub fn generate_config() -> LogRecorderConfig {
    let opt = LogRecorderConfig::from_args();
    opt
}

/// Entrypoint for the tailing of the logs
pub async fn run_logs() -> Result<(), Box<dyn ::std::error::Error>> {
    let pod_vec = get_all_pod_info().await?;
    run_cmd(pod_vec).await?;
    Ok(())
}

///  Kicks off the concurrent logging
async fn run_cmd(pod_info: Vec<PodInfo>) -> Result<(), Box<dyn ::std::error::Error>> {
    if LogRecorderConfig::global().file {
        tokio::fs::create_dir_all(&LogRecorderConfig::global().outfile).await?;
    }

    println!("Beginning to tail logs ... press <ctrl> + c to kill wufei...");
    let mut children = Vec::new();
    let pods = Api::v1Pod(KubeClient::client().client.clone())
        .within(&LogRecorderConfig::global().namespace);

    for pod in pod_info {
        let p = pods.clone();
        children.push(tokio::task::spawn(async move {
            run_individual(&pod, &p).await.unwrap()
        }));
    }
    let _ = future::join_all(children).await;

    Ok(())
}

/// Run individual async tasks and gather logs or stream logs based off of params.  It will collect
/// logs for each individual container.
async fn run_individual(
    pod_info: &PodInfo,
    current_pods: &Api<Pod>,
) -> Result<(), Box<dyn ::std::error::Error>> {
    let mut lp = LogParams::default();
    let container = &pod_info.container;
    lp.container = Some(container.to_owned());
    lp.tail_lines = Some(LogRecorderConfig::global().tail_lines);
    lp.previous = LogRecorderConfig::global().previous;
    lp.since_seconds = LogRecorderConfig::global().since;
    lp.follow = true;

    let mut log_prefix = "[".to_owned() + &pod_info.name + "][" + &pod_info.container + "]";

    if LogRecorderConfig::global().color {
        let color = COLOR_VEC.choose(&mut rand::thread_rng()); // get random color
        let str_color = color.unwrap().to_string(); // unwrap random
        log_prefix = log_prefix.color(str_color).to_string();
    }

    let mut out_file = if LogRecorderConfig::global().file {
        Some(
            OpenOptions::new()
                .append(true)
                .create(true)
                .open(&pod_info.file_name)
                .await?,
        )
    } else {
        None
    };
    
    if LogRecorderConfig::global().gather {
        lp.follow = false;
        lp.pretty = true;
        let output = current_pods.log(&pod_info.name, &lp).await?;
        let log_msg = format!("{}: {:?}\n", &log_prefix, &output);
        match out_file {
            Some(ref mut file) => record(file, output).await?,
            None => stdout(log_msg).await?,
        }
        return Ok(())
    }

    let mut output = current_pods.log_stream(&pod_info.name, &lp).await?.boxed();
    while let Some(line) = output.next().await {
        let unpacked_line = line.unwrap();
        let log_msg = if LogRecorderConfig::global().json_key != None {
            let line_str = String::from_utf8_lossy(&unpacked_line);
            let key = &LogRecorderConfig::global().json_key.as_ref().unwrap();
            let json_blob: Result<Value, serde_json::error::Error> =
                serde_json::from_str(&line_str);
            match json_blob {
                Ok(json_b) => {
                    let j: Value = json_b;
                    let log = if j[key].is_null() {
                        format!("")
                    } else {
                        format!("{}: {:?}\n", &log_prefix, line_str)
                    };
                    log
                }
                Err(_) => format!(""),
            }
        } else {
            format!(
                "{}: {:?}\n",
                &log_prefix,
                String::from_utf8_lossy(&unpacked_line)
            )
        };

        match out_file {
            Some(ref mut file) => record(file, log_msg).await?,
            None => stdout(log_msg).await?,
        }
    }
    Ok(())
}

async fn record(out_file: &mut File, log_msg: String) -> Result<(), Box<dyn ::std::error::Error>> {
    out_file.write(&log_msg.as_bytes()).await?;
    Ok(())
}

async fn stdout(log_msg: String) -> Result<(), Box<dyn ::std::error::Error>> {
    let _ = tokio::io::stdout().write(log_msg.as_bytes()).await?;
    Ok(())
}

/// A thin adapter function that will add a new tokio task to the task pool, to follow any new
/// pods that the informer alerts us too.
async fn run_individual_async(pod_info: PodInfo) {
    let pods = Api::v1Pod(KubeClient::client().client.clone())
        .within(&LogRecorderConfig::global().namespace);
    let single_task = tokio::task::spawn(async move {
        println!(
            "Informer found new pod: {:?} with container: {:?}, starting to tail the logs",
            pod_info.name, pod_info.container,
        );
        run_individual(&pod_info, &pods).await.unwrap();
    });

    tokio::task::spawn(async {
        single_task.await.unwrap();
    });
}

/// Gather all information about the pods currently deployed in the users kubernetes cluster
async fn get_all_pod_info() -> Result<Vec<PodInfo>, Box<dyn ::std::error::Error>> {
    println!(
        "Getting all pods in namespace {}...",
        LogRecorderConfig::global().namespace
    );
    let pods = Api::v1Pod(KubeClient::client().client.clone())
        .within(&LogRecorderConfig::global().namespace);
    let mut pod_info_vec: Vec<PodInfo> = vec![];
    let mut lp = ListParams::default();
    lp.label_selector = LogRecorderConfig::global().selector.clone();

    
    let found_pods = pods.list(&lp).await?;

    if LogRecorderConfig::global().container.len() > 0 || LogRecorderConfig::global().pod.len() > 0 {

        let filter_pods = LogRecorderConfig::global().pod.to_vec();
        let filter_containers = LogRecorderConfig::global().container.to_vec();

        
        let pod_filter_map: HashMap<String, bool> = filter_pods.iter().map(|filter_p| (filter_p.clone(), true)).collect();
        let container_filter_map: HashMap<String, bool> = filter_containers.iter().map(|filter_c| (filter_c.clone(), true)).collect();

        for p in found_pods {
            filter_pods_results(p, &pod_filter_map, &container_filter_map, &mut pod_info_vec);
        }
        
        if pod_info_vec.len() < 1 {
            Err("no pods found with filter criteria")?;
        }

    } else {
        for p in found_pods {
            for c in p.spec.containers {
                let container = c.name;
                let pod_name = p.metadata.name.to_string();
                let pod_info = PodInfo::new(pod_name, container, LogRecorderConfig::global().outfile.to_string());
                pod_info_vec.push(pod_info);
            }
        }
    }

    Ok(pod_info_vec)
}

fn filter_pods_results(input_pod: Pod, pod_filter: &HashMap<String, bool>, container_filter: &HashMap<String, bool>, filtered_pod_info: &mut Vec<PodInfo>) {

    if pod_filter.capacity() > 0 {
        match pod_filter.get(&input_pod.metadata.name.to_string()) {
            Some(_t) => {
                for c in input_pod.spec.containers {
                
                    if container_filter.capacity() > 0 {
                        // only add if included in filter
                        match container_filter.get(&c.name) {
                            Some(_t) => {
                                let pod_name = input_pod.metadata.name.to_string();
                                let container = c.name;
                                let pod_info = PodInfo::new(pod_name, container, LogRecorderConfig::global().outfile.to_string());
                                filtered_pod_info.push(pod_info);
                            },
                            None => ()
                        }
                    } else {
                        // add without container filter
                        let pod_name = input_pod.metadata.name.to_string();
                        let container = c.name;
                        let pod_info = PodInfo::new(pod_name, container, LogRecorderConfig::global().outfile.to_string());
                        filtered_pod_info.push(pod_info);
                    }
                }
            
            },
            None => ()
        }
    } else {
        for c in input_pod.spec.containers {
            if container_filter.capacity() > 0 {
                match container_filter.get(&c.name) {
                    Some(_t) => {
                        let pod_name = input_pod.metadata.name.to_string();
                        let container = c.name;
                        let pod_info = PodInfo::new(pod_name, container, LogRecorderConfig::global().outfile.to_string());
                        filtered_pod_info.push(pod_info);
                    },
                    None => ()
                }
            } else {
                // add without filter
                let pod_name = input_pod.metadata.name.to_string();
                let container = c.name;
                let pod_info = PodInfo::new(pod_name, container, LogRecorderConfig::global().outfile.to_string());
                filtered_pod_info.push(pod_info);
            }
        }
    }

}


/// An informer that will update the main thread pool if a new pod is spun up.
pub async fn pod_informer() -> Result<(), Box<dyn ::std::error::Error>> {
    let events = Api::v1Event(KubeClient::client().client.clone());
    let ei = Informer::new(events).init().await?;
    loop {
        let mut events = ei.poll().await.unwrap().boxed();

        while let Some(event) = events.next().await {
            let event = event?;
            handle_events(event).await?;
        }
    }
}

/// Watches for an event.  If there is a new added event, we check if its a created pod type.  If
/// it is we see if the pod exists in the clusters namespace, and if it does exist, we make sure
/// the pod is healthy.  If the pod is healthy, we had it to the threadpool and begin tailing the
/// containers in the pod
async fn handle_events(ev: WatchEvent<v1Event>) -> Result<(), Box<dyn ::std::error::Error>> {
    match ev {
        WatchEvent::Added(o) => {
            if o.message.contains("Created pod") {
                println!("{}, checking to see if this event effects wufei", o.message);

                let pod_message: Vec<&str> = o.message.split(":").collect();
                let pod_str = pod_message[1].trim();

                let pods = get_all_pod_info().await?;
                for p in pods {
                    if pod_str == p.name {
                        loop {
                            let healthy = check_status(&p.name).await?;
                            if healthy {
                                break;
                            }
                            let five_secs = time::Duration::from_secs(5);
                            thread::sleep(five_secs);
                        }
                        run_individual_async(p.clone()).await;
                    }
                }
            }
        }
        WatchEvent::Modified(_) => {}
        WatchEvent::Deleted(_) => {}
        WatchEvent::Error(_) => {}
    }
    Ok(())
}

/// Checks to see if the newly created pod is healthy, if the pod is healthy, then it is ready to
/// be added to the logging threadpool
async fn check_status(pod: &str) -> Result<bool, Box<dyn ::std::error::Error>> {
    let pods = Api::v1Pod(KubeClient::client().client.clone())
        .within(&LogRecorderConfig::global().namespace);
    let pod_obj = pods.get(pod).await?;
    let status = pod_obj.status.unwrap().phase.unwrap();

    if status != "Running" {
        return Ok(false);
    }
    Ok(true)
}

pub async fn create_kube_client() -> Result<KubeClient, Box<dyn ::std::error::Error>> {
    let config = config::load_kube_config().await?;
    Ok(KubeClient {
        client: APIClient::new(config),
    })
}
