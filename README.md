# WUFEI
Wufei is an async Rust CLI Tool for the aggregation of Kubernetes logs. This tool will write kubernetes logs for each pod down to a container level to a file or to stdout depending on the developers needs and also has the ability to log new pods that are spun up in the namespace as well. There is an informer written to let Wufei know when new pods spin up!

Heavily inspired by https://github.com/johanhaleby/kubetail Kubetail.


![Wufei](wufei.jpeg?raw=true "Wufei")

## Installation
As of right now, Wufei is NOT part of cargo.  Its on my todo list.  Right now just do cargo build in the root of the the project, and then access the wufei in target/debug/wufei
```bash
cargo run -- --namespace=<my-kube-namespace> --color
```

## Example Output
Video coming soon

## CLI Arguments
```
Wufei 0.2.3
Eric McBride <ericmcbridedeveloper@gmail.com> github.com/ericmcbride
Tail ALL your kubernetes logs at once, or record them to files

USAGE:
    wufei [FLAGS] [OPTIONS]

FLAGS:
        --color       Pods for the logs will appear in color in your terminal
    -f, --file        Record the logs to a file. Note: Logs will not appear in stdout
    -h, --help        Prints help information
        --previous    Grab previous logs
        --update      Runs an informer, that will add new pods to the tailed logs
    -V, --version     Prints version information

OPTIONS:
        --json-key <json-key>        key to search for in the json, prints out the value. Only single key supported
    -n, --namespace <namespace>      Namespace for logs [default: kube-system]
    -o, --outfile <outfile>          Outfile of where the logs are being recorded [default: /tmp/wufei/]
        --selector <selector>        Select pods by label example: version=v1
        --since <since>              Only return logs newer then a duration in seconds like 1, 60, or 180
        --tail-lines <tail-lines>    If set, the number of lines from the end of the logs to show [default: 1]
```

Wufei requires a namespace.
- The color flog `--color` will display pod names in colors in stdout.
- The file flag `--file` will write the logs to /tmp/wufei/<podname> based on pod name.
- The update flag `--update` will spin up an informer that will listen for new pods to spin up
- The previous flag `--previous` will show a previous containers logs.  Specify `--tail-lines` or
  it will only show you the last line from it.

- The namespace option `--namespace` is the namespace the developer wants to use to tail logs from
- The outfile option `--outfile` is used when the file flag is used, to change the location of
  where the files are used
- The selector option `--selector` will allow a single key/value pair to tail logs by.  Example
  would be `--selector='version=v1'`
- The since option `--since` will return logs newer then the duration in seconds.
- The tail-lines option `--tail-lines` will show the number of lines from the ends of the log to
  show.  Defaults to 1
- The json-key option `--json-key` allows the user to seach logs for a key in a valid json blob.
  The only thing wufei will print out are logs that contain the key.  If nothing is printing out,
  nothing matches

Examples:

```
cargo run -- --namespace=default --color
cargo run -- --namespace=default --selector='version=v1' --update
cargo run -- --namespace=default --file --outfile=/tmp/new_outfile --update
cargo run -- --namespace=default --selector=`version=v1` --file
cargo run -- --namespace=default --previous --tail-lines=20 --color
cargo run -- --namespace=default --previous --color
cargo run -- --namespace=default --previous --since=1800
cargo run -- --namespace=default --previous --tail-lines=20 --color --selector='version=v1'
cargo run -- --namespace=default --color --json-key=X-REQUEST-ID
cargo run -- --namespace=default --file --json-key=user_id --select='version=v2'
```

## WUFEI USES YOUR CURRENT KUBE CONTEXT
`export $KUBECONFIG=:$KUBECONFIG/path/to/your/config`

#### LIST CONTEXTS
`kubectl config view`

#### USE CONTEXT
`kubectl config use-context my-context`

## TOO MANY OPEN FILES ERROR
This error will pop up, depending on the settings on your operating system.  This is due to
security reasons.  Below is how you would fix this on a Mac.
#### OS/X
`ulimit -n 2048`

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://choosealicense.com/licenses/mit/)
