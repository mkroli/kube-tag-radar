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
kubectl apply -k https://github.com/mkroli/kube-tag-radar.git/kubernetes?ref=main
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
    targetRevision: main
    kustomize:
      namespace: kube-tag-radar
      patches:
      - target:
          kind: ConfigMap
          name: kube-tag-radar-config
        patch: |-
          - op: add
            path: /data/config.yaml
            value: |-
              ignore: []
  syncPolicy:
    syncOptions:
      - CreateNamespace=true
    automated:
      selfHeal: true
      prune: true
```

## Configuration

`kube-tag-radar` looks for this file at `config.yaml` by default. When deployed in Kubernetes, configuration is provided via a `ConfigMap` named `kube-tag-radar-config`.

### Example config file

```yaml
database: "./kube-tag-radar.sqlite"
update_delay: "5 minutes"
update_interval: "3 hours"
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
