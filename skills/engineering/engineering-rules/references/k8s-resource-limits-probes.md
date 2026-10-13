---
title: Every prod container sets resource requests/limits and three probes; use HPA with minReplicas >= 2
section: k8s
scope: general
applies-to: k8s
status: direction
tags: k8s, kubernetes, resources, probes, hpa, reliability
---

## Every prod container sets resource requests/limits and three probes; use HPA with minReplicas >= 2

Pods without resource requests get BestEffort QoS and are evicted first. Pods without probes
receive traffic before they are ready and restart silently when deadlocked. A single replica
with no HPA means any rolling update or node drain causes downtime. All three are required for
any production workload.

**Prefer:**

```yaml
# Deployment container spec
resources:
  requests:
    cpu: 100m
    memory: 256Mi
  limits:
    cpu: 500m
    memory: 512Mi       # 2x request; tune after observing steady-state

startupProbe:           # slow init (> 10s) — e.g. model load
  httpGet:
    path: /healthz
    port: 8080
  failureThreshold: 30
  periodSeconds: 10

readinessProbe:         # gates traffic; may check DB/cache connectivity
  httpGet:
    path: /ready
    port: 8080
  periodSeconds: 10
  failureThreshold: 3

livenessProbe:          # detects deadlock; never check external deps here
  httpGet:
    path: /healthz
    port: 8080
  periodSeconds: 20
  failureThreshold: 3
  initialDelaySeconds: 0

---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
spec:
  minReplicas: 2        # always >= 2 in prod
  maxReplicas: 20
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300   # prevent flapping
```

**Avoid:**

```yaml
# No probes, no resources, single replica — see k8s-current-gap
spec:
  replicas: 1
  containers:
    - name: app
      image: registry.example.com/my-service:latest
```

Reference: `k8s-current-gap` documents the common baseline to upgrade from; `k8s-security-context`
covers the complementary hardening step.
