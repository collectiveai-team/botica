---
title: Containers run hardened — non-root, read-only rootfs, dropped capabilities, dedicated ServiceAccount
section: k8s
scope: general
applies-to: k8s
status: direction
tags: k8s, kubernetes, security, securitycontext, hardening
---

## Containers run hardened — non-root, read-only rootfs, dropped capabilities, dedicated ServiceAccount

Running containers as root, with a writable root filesystem, or with the default ServiceAccount
are the first things an attacker exploits after a container escape. All four controls must be set
together — any one alone is incomplete.

**Prefer:**

```yaml
spec:
  serviceAccountName: my-app-sa       # dedicated SA per workload; never use "default"
  automountServiceAccountToken: false  # only true if pod needs Kubernetes API access
  securityContext:
    runAsNonRoot: true
    fsGroup: 1000
    seccompProfile:
      type: RuntimeDefault
  containers:
    - name: app
      securityContext:
        runAsUser: 1000
        runAsGroup: 1000
        readOnlyRootFilesystem: true
        allowPrivilegeEscalation: false
        capabilities:
          drop:
            - ALL
      volumeMounts:
        - name: tmp
          mountPath: /tmp             # mount writable emptyDir for any path that needs writes
  volumes:
    - name: tmp
      emptyDir: {}
```

**Avoid:**

```yaml
# Missing security context entirely — a ServiceAccount alone is not enough
spec:
  serviceAccountName: my-app-sa
  containers:
    - name: app
      # no securityContext block
```

**Direction:** new manifests must include the full block above. For existing manifests, add the
security context in the same PR that adds probes and resource limits (see `k8s-resource-limits-probes`).

Reference: Kubernetes Pod Security Standards `restricted` profile enforces a superset of these
controls at the namespace level via admission control.
