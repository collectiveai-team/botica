---
title: Multi-service apps use docker compose; prod fronts services with Traefik
section: infra
scope: general
applies-to: all
status: current
tags: docker-compose, traefik, infra, networking
---

## Multi-service apps use docker compose; prod fronts services with Traefik

Multi-service stacks are wired with `docker compose`. Production deployments join an external
`traefik` network and declare labels for routing, TLS (Let's Encrypt), and HTTPS redirect.
A shared `./resources:/resources` volume passes large files between services without copying.

**Prefer (Traefik labels on each service; external traefik network):**

```yaml
services:
  api:
    networks:
      - traefik
      - app-network
    labels:
      - traefik.enable=true
      - traefik.docker.network=traefik
      - traefik.http.routers.api-https.rule=Host(`api.example.com`)
      - traefik.http.routers.api-https.tls.certresolver=letsencrypt
      - traefik.http.services.api.loadbalancer.server.port=8000
    volumes:
      - ./resources:/resources

networks:
  app-network:
    driver: bridge
  traefik:
    external: true
    name: traefik
```

**Avoid:**

```yaml
# Don't expose ports directly on prod services — route through Traefik
ports:
  - "8000:8000"
```

Reference: see `infra-makefile-docker` for the Makefile targets that drive `docker compose up/down`.
