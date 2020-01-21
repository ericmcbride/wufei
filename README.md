# WUFEI
Wufei is a Rust CLI Tool for the aggregation of Kubernetes logs. This tool will write kubernetes logs for each pod to a file and also has the ability to log new pods that are spun up in the namespace as well. There is an informer written to let Wufei know when new pods spin up!

Heavily inspired by https://github.com/johanhaleby/kubetail Kubetail.


![Wufei](wufei.jpeg?raw=true "Wufei")

## Installation
As of right now, Wufei is NOT part of cargo.  Its on my todo list.  Right now just do cargo build in the root of the the project, and then access the wufei in target/debug/wufei
```bash
cargo run -- --namespace=<my-kube-namespace> --kubeconfig=<kube.config> --color
```

## Example Output
Video coming soon

## CPU USAGE WARNING:
Depending on what cloud provider you are using, and how your kubernetes configs / contexts are
set up, you may have to generate a new token on each request.  That may not sound like a huge
deal, but for example, AWS has a python script that calls out and gets a token everytime
kubectl is called.  This spins up a new pyenv environment everytime.  If you see this happen,
you can htop and see all the pyenvs spinning up.  You may need to change the strategy to
generate one token upfront, and use that throughout.  It may not be the most secure method of
doing this, and you may need to set some sort of RBAC role, because this issue will happen,
especially the more pods you have in your cluster...

https://kubernetes.io/docs/reference/access-authn-authz/authentication/

## CLI Arguments
```
Wufei 0.1.0
Eric McBride <ericmcbridedeveloper@gmail.com> github.com/ericmcbride
Tail ALL your kubernetes logs at once, or record them to files

USAGE:
    wufei [FLAGS] [OPTIONS]

FLAGS:
        --color      Pods for the logs will appear in color in your terminal
    -f, --file       Record the logs to a file. Note: Logs will not appear in stdout
    -h, --help       Prints help information
        --update     Runs an informer, that will add new pods to the tailed logs
    -V, --version    Prints version information

OPTIONS:
    -k, --kubeconfig <kube-config>    The kube config for accessing your cluster [default: ]
    -n, --namespace <namespace>       Namespace for logs [default: kube-system]
    -o, --outfile <outfile>           Outfile of where the logs are being recorded [default: /tmp/wufei/]
```

Wufei requires a namespace.  The Color flog `--color` will display pod names in colors in stdout.  The file flag `--file` will write the logs to /tmp/wufei/<podname> based on pod name. If `--kubeconfig` is passed, it will use a absolute path to the config file you want to use.
Example: 

```
cargo run -- --namespace=dev --kubeconfig=/my/full/path/kube.config --color
```
If the `--kubeconfig` flag is not passed, then it was use you're current
kube context 


## Coming Soon
Complete rewrite.  Going to use async, instead of a tokio threadpool.  I will have an option to
pass in -thread or -async for personal benchmarking reasons for a bit.  Then I will remove it.

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://choosealicense.com/licenses/mit/)
