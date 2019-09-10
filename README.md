# WUFEI
Wufei is a Rust CLI Tool for the aggregation of Kubernetes logs. I had a use case where I needed to write all logs to files, for debugging so I decided to write Wufei.

Heavily inspired by https://github.com/johanhaleby/kubetail Kubetail.


![Wufei](wufei.jpeg?raw=true "Wufei")

## Installation
As of right now, Wufei is NOT part of cargo.  Its on my todo list.  Right now just do cargo build in the root of the the project, and then access the wufei in target/debug/wufei
```bash
./target/debug/wufei --namespace=<my-kube-namespace> --kubeconfig=<kube.config> --color
```

## Example Output
Example output with Linkerd:

![Screen](screen.jpeg?raw=true "Screen")

## CLI Arguments
```
wufei 1.0
Eric McBride <ericmcbridedeveloper@gmail.com>
View All Logs from Kubernetes Namespace

USAGE:
    wufei [FLAGS] [OPTIONS] --namespace <NAMESPACE>

FLAGS:
        --color      Show colored output
    -f, --file       Write logs to files based on pod name /tmp/podname
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -k, --kubeconfig <KUBECONFIG>    Kube config file if not using context
    -n, --namespace <NAMESPACE>      Namespace for logs
    -o, --outfile <OUTFILE>          Outfile for --file flag
```

Wufei requires a namespace.  The Color flog `--color` will display pod names in colors in stdout.  The file flag `--file` will write the logs to /tmp/<podname> based on pod name. If `--kubeconfig` is passed, it will use a absolute path to the config file you want to use.
Example: 

```
cargo run -- --namespace=dev --kubeconfig=/my/full/path/kube.config --color
```
If the `--kubeconfig` flag is not passed, then it was use you're current
kube context 


## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://choosealicense.com/licenses/mit/)
