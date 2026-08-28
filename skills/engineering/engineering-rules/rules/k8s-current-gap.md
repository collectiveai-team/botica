---
title: Bare Deployments without probes, resources, or HPA are a gap — harden before shipping
section: k8s
scope: general
applies-to: k8s
status: current
tags: k8s, kubernetes, gap, probes, resources, hpa
---

## Bare Deployments without probes, resources, or HPA are a gap — harden before shipping

A Deployment that exists in a codebase without probes, resource requests/limits, or an HPA
is the current state, not the target. Do not copy bare manifests as-is for new services.
Treat any such manifest as something to harden before it is production-ready.

**What a bare manifest looks like (do not replicate):**

```yaml
spec:
  replicas: 1                        # single replica — no HA
  containers:
    - name: app
      image: registry.example.com/my-service:test   # mutable tag in prod
      # NO readinessProbe
      # NO livenessProbe
      # NO resources.requests / resources.limits
# NO HorizontalPodAutoscaler
```

**What is worth keeping from existing manifests:**

```yaml
# GKE Ingress + FrontendConfig for HTTPS redirect
apiVersion: networking.gke.io/v1beta1
kind: FrontendConfig
spec:
  redirectToHttps:
    enabled: true

# ClusterIP services + dedicated ServiceAccounts per workload
```

**Known gaps to fix before treating any manifest as production-ready:**

- Add `readinessProbe` + `livenessProbe` (and `startupProbe` for slow init)
- Add `resources.requests` and `resources.limits` to every container
- Add a `HorizontalPodAutoscaler` (`autoscaling/v2`, `minReplicas: 2`)
- Replace mutable image tags (`:latest`, `:test`) with pinned digest or semver tag
- Add security context (`runAsNonRoot`, `readOnlyRootFilesystem`, drop capabilities)

Reference: see `k8s-resource-limits-probes` and `k8s-security-context` for the target state.
