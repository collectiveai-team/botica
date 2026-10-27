## Directive

`prek.toml` has the hook `kube-linter`, so that the Kubernetes manifests of the repository pass
kube-linter with the `.kube-linter.yaml` of the recipe `k8s` before they are committed. Each
container of a workload runs as a user other than root, drops `ALL` capabilities, does not allow
privilege escalation, has a read-only root filesystem, and declares a CPU request and a memory
limit. Each container of a Deployment, a StatefulSet or a DaemonSet also has a startup, a liveness
and a readiness probe. A Deployment with the label `autoscale: "true"` has a
HorizontalPodAutoscaler that targets it.

## Why

A Deployment without a `securityContext` runs as root, with the default capabilities of the
runtime and a writable root filesystem, and the API server accepts it. A container without a
memory limit can take the memory of its node, and one without a CPU request is scheduled as if it
needed nothing. Without a readiness probe a pod receives traffic before it is ready, and without a
liveness probe a process that hangs is not restarted. None of this fails when the manifest is
applied.

## Exceptions

A Job and a CronJob run to completion and take no traffic, so the probe checks do not read them.
The checks of the security context and of the resources read them, and their init containers.

The implementation has no `applies-when`: no file name tells a Kubernetes manifest from other
YAML, so `external-config` asks for the hook whenever the rule is active. The rule is off by
default, and the recipe `k8s` selects it.

## Tool

[kube-linter](https://docs.kubelinter.io/) reads plain YAML, the folders of Helm charts, which it
renders with their `values.yaml`, and the folders with a `kustomization.yaml`, which it builds.
The hook `kube-linter` of prek runs `kube-linter lint .` from the root of the repository, and
kube-linter reads `.kube-linter.yaml` from there. YAML that is not a Kubernetes object, such as a
workflow of GitHub Actions, is skipped, and a repository with no manifests passes. The
configuration is what enforces this rule: the checks that kube-linter turns on by default, the
probes on long-running workloads, and custom checks for `drop: ["ALL"]`, the startup probe and the
autoscaler.

The hook has `language: golang`, so prek builds kube-linter from the tag in `rev` the first time
it runs, and kube-linter does not have to be installed. `external-config` checks the hook id; it
does not read `.kube-linter.yaml`, and `botica recipe check k8s` reports that file when it is
missing.

## Configuration

The block `# MARK: kube-linter` that the recipe `k8s` adds to `prek.toml`:

```toml
[[repos]]
repo = "https://github.com/stackrox/kube-linter"
rev = "v0.8.3"
hooks = [{ id = "kube-linter", args = ["."], pass_filenames = false, priority = 2 }]
```

A file of the templates of a chart is not valid YAML until it is rendered, and kube-linter given
that file alone reports no object and exits with 0, so the hook lints the whole tree instead of
the files that prek passes. The recipe `k8s` also writes `.kube-linter.yaml`.

## Do

```yaml
spec:
  template:
    spec:
      securityContext:
        runAsNonRoot: true
        runAsUser: 10001
        seccompProfile:
          type: RuntimeDefault
      containers:
        - name: api
          image: ghcr.io/acme/api:1.4.2
          securityContext:
            allowPrivilegeEscalation: false
            readOnlyRootFilesystem: true
            capabilities:
              drop: ["ALL"]
          resources:
            requests:
              cpu: 100m
              memory: 256Mi
            limits:
              memory: 256Mi
          startupProbe:
            httpGet: { path: /healthz/live, port: http }
            failureThreshold: 30
          livenessProbe:
            httpGet: { path: /healthz/live, port: http }
          readinessProbe:
            httpGet: { path: /healthz/ready, port: http }
          volumeMounts:
            - { name: tmp, mountPath: /tmp }
      volumes:
        - { name: tmp, emptyDir: {} }
```

## Don't

```yaml
spec:
  template:
    spec:
      containers:
        - name: api
          image: ghcr.io/acme/api:1.4.2
```

kube-linter reports `run-as-non-root`, `no-read-only-root-fs`, `drop-all-capabilities`,
`unset-cpu-requirements`, `unset-memory-requirements` and the three probes.

## Suppression

An annotation on the object that names the check and the reason:
`ignore-check.kube-linter.io/no-read-only-root-fs: "writes its cache to /var/cache"`. A check that
does not apply to the repository goes in `checks.exclude` of `.kube-linter.yaml`, with a comment
that gives the reason. The hook is not removed to silence one check.
