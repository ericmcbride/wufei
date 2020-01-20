use crate::utils;
use colored::*;
use rand::seq::SliceRandom;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::str;

use std::thread;
use structopt::StructOpt;
use tokio_threadpool::{blocking, ThreadPool};

use futures::future::{lazy, poll_fn};
use futures::Future;

use kube_async::{
    api::v1Event,
    api::{Api, Informer, WatchEvent},
    client::APIClient,
    config,
};

use new_futures::stream::StreamExt;

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

#[derive(Clone, StructOpt, Debug)]
#[structopt(name = "basic")]
pub struct LogRecorderConfig {
    #[structopt(short, long, default_value = "kube-system")]
    pub namespace: String,

    #[structopt(short, long = "kubeconfig", default_value = "")]
    kube_config: String,

    #[structopt(short, long, default_value = "/tmp/wufei/")]
    outfile: String,

    #[structopt(short, long)]
    file: bool,

    #[structopt(long)]
    color: bool,
}

/// Pod infromation
#[derive(Debug, Default, Clone)]
pub struct PodInfo {
    name: String,
    container: String,
    parent: String,
}

/// Cli options for wufei
pub fn generate_config() -> Result<(LogRecorderConfig), Box<dyn ::std::error::Error>> {
    let opt = LogRecorderConfig::from_args();
    Ok(opt)
}

/// Entrypoint for the tailing of the logs
pub fn run_logs(log_options: &LogRecorderConfig) -> Result<(), Box<dyn ::std::error::Error>> {
    let pod_vec = get_all_pod_info(&log_options.namespace, &log_options.kube_config)?;
    let pod_hashmap = generate_hashmap(pod_vec, &log_options.outfile);
    run_cmd(pod_hashmap, &log_options)?;
    Ok(())
}

///  Kicks off the concurrent logging
fn run_cmd(
    pod_hashmap: HashMap<String, PodInfo>,
    log_options: &LogRecorderConfig,
) -> Result<(), Box<dyn ::std::error::Error>> {
    let mut children = vec![];
    fs::create_dir_all(&log_options.outfile)?;

    let pool = ThreadPool::new();
    for (k, v) in pod_hashmap {
        let namespace = log_options.namespace.clone();
        let kube_config = log_options.kube_config.clone();
        let file = log_options.file.clone();
        let color = log_options.color.clone();

        // In this chunk of code we are using a tokio threadpool.  The threadpool runs a task,
        // which can easily be compared to a green thread or a GO routine.  We do not have a
        // complicated requirement here, so we use just use futures built in poll_fn which is a
        // stream wrapper function that returns a poll.  This satisifies the pool.spawn function
        children.push(pool.spawn(lazy(move || {
            poll_fn(move || {
                blocking(|| {
                    run_individual(
                        k.to_string(),
                        &v,
                        namespace.to_owned(),
                        kube_config.to_owned(),
                        file.to_owned(),
                        color.to_owned(),
                    )
                })
                .map_err(|_| panic!("the threadpool shutdown"))
            })
        })));
    }
    pool.shutdown_on_idle().wait().unwrap();
    Ok(())
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
    println!("Kicking off individual pod for {:?}", v.name);
    let mut kube_cmd = Command::new("kubectl");
    let container = get_app_container(&v.container);

    if kube_config.len() != 0 {
        kube_cmd.arg("--kubeconfig");
        kube_cmd.arg(&kube_config);
    }

    kube_cmd.arg("logs");
    kube_cmd.arg("-f");
    kube_cmd.arg(&v.parent);
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
    let mut log_prefix = "[".to_owned() + &v.parent + "][" + &container + "]";

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
                let new_line = format!("{}\n", line);
                out_file.write(&new_line.as_bytes()).unwrap();
            });
    } else {
        reader
            .lines()
            .filter_map(|line| line.ok())
            .for_each(|line| {
                let log_msg = format!("{}: {}\n", &log_prefix, line);
                let _ = std::io::stdout().write(log_msg.as_bytes());
            });
    }
}

async fn run_individual_async(
    k: String,
    v: PodInfo,
    namespace: String,
    kube_config: String,
    file: bool,
    color_on: bool,
) {
    thread::spawn(move || {
        println!(
            "Informer found new pod {:?}, starting to tail the logs",
            v.name
        );
        run_individual(k, &v, namespace, kube_config, file, color_on)
    });
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
) -> Result<Vec<String>, Box<dyn ::std::error::Error>> {
    let mut kube_cmd = Command::new("kubectl");
    if kube_config.len() != 0 {
        kube_cmd.arg("--kubeconfig");
        kube_cmd.arg(&kube_config);
    }
    kube_cmd.arg("get");
    kube_cmd.arg("pods");
    kube_cmd.arg("-n");
    kube_cmd.arg(&namespace);
    kube_cmd.arg("-o");
    kube_cmd.arg("jsonpath={range .items[*]}{.metadata.name} {.spec['containers', 'initContainers'][*].name}\n{end}");

    let output = kube_cmd.output().expect("Failed to get kubernetes pods");

    if output.stderr.len() != 0 {
        let byte_string = String::from_utf8_lossy(&output.stderr);
        utils::generate_err(byte_string.to_string())?
    }

    let pods = str::from_utf8(&output.stdout)?;
    let pods_vec: Vec<&str> = pods.split("\n").collect();
    Ok(pods_vec.iter().map(|&x| x.to_string()).collect())
}

/// We generate a hashmap with the key being the file_path (easy access to write files), and the
/// value being a PodInfo struct
fn generate_hashmap(pod_vec: Vec<String>, outfile: &str) -> HashMap<String, PodInfo> {
    let mut pods_hashmap = HashMap::new();
    for info in pod_vec {
        if info == "" {
            continue;
        }

        let pod_info = info.split_whitespace();
        let mut pod_info_vec: VecDeque<&str> = pod_info.collect();
        let parent_pod_name = &pod_info_vec.pop_front().unwrap();

        for pod in pod_info_vec {
            let file_path = outfile.to_owned() + &parent_pod_name + "-" + &pod + ".txt";
            let containers = &pod;
            let name = parent_pod_name.to_string() + "-" + &pod.to_string();
            pods_hashmap.insert(
                file_path,
                PodInfo {
                    name: name.to_string(),
                    container: containers.to_string(),
                    parent: parent_pod_name.to_string(),
                },
            );
        }
    }

    pods_hashmap
}

pub async fn pod_informer(
    wufei_config: &LogRecorderConfig,
) -> Result<(), Box<dyn ::std::error::Error>> {
    // #TODO: Figure out how to get this to work with a file path
    let config = config::load_kube_config().await.unwrap();
    let client = APIClient::new(config);

    let events = Api::v1Event(client);
    let ei = Informer::new(events).init().await.unwrap();
    loop {
        let mut events = ei.poll().await.unwrap().boxed();

        while let Some(event) = events.next().await {
            let event = event.unwrap();
            handle_events(&wufei_config, event).await?;
        }
    }
}
// This function lets the app handle an event from kube
async fn handle_events(
    wufei_config: &LogRecorderConfig,
    ev: WatchEvent<v1Event>,
) -> Result<(), Box<dyn ::std::error::Error>> {
    let config = config::load_kube_config().await.unwrap();
    let client = APIClient::new(config);
    println!(
        "Waiting on events for namespace: {:?}",
        wufei_config.namespace
    );
    match ev {
        WatchEvent::Added(o) => {
            println!("New Event: {}, {}", o.type_, o.message);
            if o.message.contains("Created pod") {
                let async_config = wufei_config.clone();
                println!(
                    "Pod created, pulling pod into threadpool message: {}",
                    o.message
                );
                let pods = Api::v1Pod(client.clone()).within(&async_config.namespace);
                let pod_message: Vec<&str> = o.message.split(":").collect();
                let pod_str = pod_message[1].trim();
                let pod = pods.get(&pod_str).await.unwrap();
                let container_name = pod.spec.containers[0].name.clone();
                // get rid of pod_str should be container name
                let file_name = wufei_config.outfile.clone()
                    + &pod.metadata.name.clone()
                    + "-"
                    + pod_str
                    + ".txt";
                let pod_info = PodInfo {
                    name: pod.metadata.name,
                    container: container_name,
                    parent: pod_str.to_string(),
                };

                run_individual_async(
                    file_name,
                    pod_info,
                    async_config.namespace,
                    async_config.kube_config,
                    async_config.file,
                    async_config.color,
                )
                .await;
            }
        }
        WatchEvent::Modified(_) => {}
        WatchEvent::Deleted(_) => {}
        WatchEvent::Error(_) => {}
    }
    Ok(())
}
