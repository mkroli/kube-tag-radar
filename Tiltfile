load('ext://restart_process', 'docker_build_with_restart')
load('ext://namespace', 'namespace_create', 'namespace_inject')

local_resource(
    'build',
    'mkdir -p target/tilt/registry && docker run --rm -v "$(pwd)/target/tilt/registry:/usr/local/cargo/registry" -v "$(pwd):/src" -w /src -u $(id -u):$(id -g) rust cargo build -j 8 --locked --all-features --target-dir target/tilt',
    deps=['Cargo.lock', 'Cargo.toml', 'assets', 'src'],
)

dockerfile = """
FROM rust
ADD target/tilt/debug/kube-tag-radar /
"""
docker_build_with_restart(
    'ghcr.io/mkroli/kube-tag-radar',
    context='.',
    dockerfile_contents=blob(dockerfile),
    entrypoint='/kube-tag-radar',
    only=['target/tilt/debug/kube-tag-radar'],
    live_update=[
        sync('target/tilt/debug/kube-tag-radar', '/kube-tag-radar'),
    ],
)

namespace_create('kube-tag-radar')
k8s_yaml(namespace_inject(kustomize('kubernetes'), 'kube-tag-radar'))
k8s_resource(
    'kube-tag-radar',
    port_forwards=[
        port_forward(8080, name='UI'),
        port_forward(8081, container_port=8080, name='Metrics', link_path='/metrics')
    ],
    objects=[
        'kube-tag-radar:namespace',
        'kube-tag-radar:serviceaccount',
        'kube-tag-radar:clusterrole',
        'kube-tag-radar:clusterrolebinding',
        'kube-tag-radar-config:configmap',
    ],
)
