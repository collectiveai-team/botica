A Deployment, its Service and its HorizontalPodAutoscaler that pass kube-linter with the
`.kube-linter.yaml` of the recipe `k8s`.

## What to read

- The pod sets `runAsNonRoot` and a `runAsUser` other than 0. Each container sets
  `allowPrivilegeEscalation: false`, `readOnlyRootFilesystem: true` and drops `ALL` capabilities.
- The root filesystem is read-only, so the container writes only to its volumes. An `emptyDir`
  on `/tmp` takes the temporary files, with a `sizeLimit` so that it cannot fill the node.
- `requests.cpu` is what the scheduler reserves. `limits.memory` is the ceiling: past it the
  container is killed. There is no CPU limit, which would throttle the container under load.
  `requests.memory` equals the limit, which gives the scheduler the real size of the pod.
- The startup probe allows 30 failures at 5 seconds, two and a half minutes to start, before the
  liveness probe takes over. The readiness probe has its own path, so that a pod that is alive but
  cannot serve, such as one that lost its database, leaves the Service and is not restarted.
- The Deployment has the label `autoscale: "true"` and no `replicas`; the HorizontalPodAutoscaler
  sets the number of pods.

## What to adapt

- The image, the port, the paths of the probes and the values of the resources are an example.
  Measure the memory of the service and set the limit above its peak.
- A service that listens on a port below 1024 adds `NET_BIND_SERVICE` to `capabilities.add`, or
  listens on a higher port.
- A Deployment that does not scale drops the label `autoscale` and the HorizontalPodAutoscaler,
  and sets `replicas`.
- A Job or a CronJob keeps the security context and the resources, and has no probes.
