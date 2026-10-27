---
id: CES-0
title: Kubernetes workloads run without privileges and declare their resources and probes
references: ["k8s-manifests"]
---

## Directive

The Kubernetes manifests of a repository pass [`kube-linter`](https://docs.kubelinter.io/) with
the `.kube-linter.yaml` of the recipe `k8s`, as a hook of prek. In each workload:

- each container runs as a user other than root (`runAsNonRoot: true` and a `runAsUser` other
  than 0), sets `allowPrivilegeEscalation: false`, drops `ALL` capabilities and adds back only the
  ones it needs, and has `readOnlyRootFilesystem: true`, with an `emptyDir` for the paths it
  writes, such as `/tmp`;
- each container declares a CPU request and a memory limit;
- each container of a Deployment, a StatefulSet or a DaemonSet has a startup, a liveness and a
  readiness probe;
- a Deployment with the label `autoscale: "true"` has a HorizontalPodAutoscaler that targets it,
  and a HorizontalPodAutoscaler targets an object of the repository.

The recipe `k8s` adds the hook and `.kube-linter.yaml`, and selects the rule
`k8s-workload-hardening`.

## Why

The API server accepts a Deployment with no `securityContext`, no `resources` and no probes, and
runs it as root, with the default capabilities of the runtime, a writable root filesystem, no
ceiling on memory and traffic sent to it before it is ready. Nothing fails until an audit or an
incident. kube-linter reads the manifests before they are applied, with no cluster, and runs as a
hook of prek like `hadolint` (CES-114), which checks the image and not the pod that runs it.

## Scope

The hook runs `kube-linter lint .` from the root of the repository. kube-linter reads plain YAML,
renders the folder of each Helm chart with its `values.yaml` and builds the folder of each
`kustomization.yaml`. YAML that is not a Kubernetes object is skipped, so the hook passes in a
repository with no manifests. The values of a chart for another environment are not rendered: a
chart keeps the security context and the resources in its templates, and the values change only
their sizes.

The hook has `language: golang`: prek builds kube-linter from the tag that `rev` pins the first
time it runs, which needs Go. The weekly `pins` workflow moves `rev` with `prek update`.

A Job and a CronJob have no probes. The CPU limit is not required: it throttles a container under
load. The memory request may be left out; Kubernetes then sets it to the limit.

## Migration (existing repos)

Run `botica recipe install k8s`, then `prek run kube-linter --all-files`. The first run on
manifests that were not written for it reports most containers. Add the security context first,
start each service with a read-only root filesystem in a test cluster and mount an `emptyDir` on
each path it writes, then add the resources and the probes.

## Suppression

An annotation on the object, with the reason as its value:
`ignore-check.kube-linter.io/<check>: "<reason>"`. A check that does not apply to the repository
goes in `checks.exclude` of `.kube-linter.yaml`, with a comment that gives the reason. The hook is
not removed to silence one check.
