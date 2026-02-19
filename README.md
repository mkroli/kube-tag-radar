# kube-tag-radar

**kube-tag-radar** checks if the container images in your Kubernetes cluster are up to date. It can be run locally or deployed into Kubernetes. It provides an API, a web UI, and Prometheus metrics with built-in service discovery annotations.

## Getting Started

### Run Locally

```sh
cargo install --locked --git https://github.com/mkroli/kube-tag-radar.git kube-tag-radar
kube-tag-radar
```

### Deploy to Kubernetes

```sh
kubectl apply -k https://github.com/mkroli/kube-tag-radar.git/kubernetes?ref=0.10.6
```

### Deploy using Argo CD

```yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: kube-tag-radar
  namespace: argocd
spec:
  project: default
  destination:
    namespace: kube-tag-radar
    server: https://kubernetes.default.svc
  source:
    repoURL: https://github.com/mkroli/kube-tag-radar.git
    path: kubernetes
    targetRevision: 0.10.6
    kustomize:
      namespace: kube-tag-radar
      patches:
      - patch: |-
          apiVersion: v1
          kind: ConfigMap
          metadata:
            name: kube-tag-radar-config
          data:
            config.yaml: |-
              database: /data/kube-tag-radar.sqlite
              ignore: []
      - patch: |-
          apiVersion: apps/v1
          kind: Deployment
          metadata:
            name: kube-tag-radar
            annotations:
              reloader.stakater.com/auto: "true"
          spec:
            template:
              spec:
                volumes:
                  - name: data
                    emptyDir:
                    hostPath:
                      path: "/tmp/kube-tag-radar"
  syncPolicy:
    syncOptions:
      - CreateNamespace=true
    automated:
      selfHeal: true
      prune: true
```

## Configuration

`kube-tag-radar` looks for Pod annotations and a configuration file at `config.yaml` by default. When deployed in Kubernetes, the configuration file is provided via a `ConfigMap` named `kube-tag-radar-config`.

### Annotations

`kube-tag-radar` checks annotations on Pods. If you have a deployment make sure to add annotations to `spec.template.metadata.annotations...`.

| Annotation | Default | Description |
| --- | --- | --- |
| `kube-tag-radar.mkroli.com/tag` | `latest` | Will compare the current image digest with the digest of the given tag to check if it's up-to-date |
| `kube-tag-radar.mkroli.com/version_req` | `*` | Used to restrict the latest version of the image (see [Specifying Dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html)). |
| `kube-tag-radar.mkroli.com/version_regex` | `.*` | Can be used to filter available tags. If specified - the first capture group will be used to extract a semver version for proper comparison. Example: `^(.*)-alpine$` |

### Example config file

```yaml
database: "./kube-tag-radar.sqlite"
update_delay: "PT5M"
update_interval: "PT3H"
ignore:
- namespace: ...
  image: ...
```

## Sample Alerting Rule

```yaml
groups:
- name: kube-tag-radar
  rules:
  - alert: KubeTagUpdateAvailable
    expr: 'kube_tag_radar_container > 0'
    for: 1m
    annotations:
      summary: "New image available for {{ $labels.namespace }}/{{ $labels.pod }}/{{ $labels.container }}"
```
