`compose.prod.yaml` serves the application of a Compose stack on the internet from a single host.
Traefik receives the requests on ports 80 and 443, gets a certificate of Let's Encrypt for each
host name, and routes each one to the service whose labels name it. It applies over the
`compose.yaml` of the topic `compose` (`botica refs show compose`), and leaves out its
`compose.override.yaml`:

```sh
docker compose -f compose.yaml -f compose.prod.yaml up -d
```

## What to read

- Traefik does not mount the Docker socket. The socket gives full control of the daemon, and so of
  the host, to whoever reaches it; `:ro` on the mount protects the file and not the API calls that
  go through it. The service `socket-proxy` mounts it and answers only the calls that its variables
  grant: `CONTAINERS: 1` lets Traefik list the containers and read their labels, and `POST: 0`
  refuses every call that changes something. The proxy grants `EVENTS`, `PING` and `VERSION` by
  default.
- `socket-proxy` is only on the network `socket`, which is `internal: true`: it has no route out of
  the host, and only Traefik joins it. Traefik reaches the proxy at `tcp://socket-proxy:2375`.
- `--providers.docker.exposedbydefault=false` routes only the containers with the label
  `traefik.enable=true`; the database and the proxy stay unreachable from outside.
- The entry point `web` (port 80) redirects every request to `websecure` (port 443). Let's Encrypt
  validates each host name through the HTTP-01 challenge on `web`, so port 80 of the host is open.
- Traefik keeps the account and the certificates in `acme.json`, in the named volume
  `letsencrypt`, so a new container of Traefik does not request them again.
- The labels of `app` name its router (`Host`, the entry point `websecure` and the resolver `le`)
  and the port of the container. Traefik reaches `app` on the network `web`
  (`--providers.docker.network=web`), and `app` stays on `default` to reach `db`.
- `app` takes a tag that the workflow of `ci-docker` pushed, and the host builds nothing.
  `${APP_TAG:?...}` stops `docker compose up` with that message when the variable is unset.
- The dashboard and the API of Traefik are off. `--ping=true` serves `/ping` on port 8080 inside
  the container, for its healthcheck, and the file publishes no port 8080.

## What to adapt

- Set `ACME_EMAIL`, `APP_HOST` and `APP_TAG` in the `.env` of the host, beside the files. Point the
  DNS record of `APP_HOST` at the host before the first start: Let's Encrypt fails the challenge
  otherwise and limits the failed attempts.
- Each service that Traefik serves adds the labels of `app` with its own router name, its
  `Host` rule and its port, and joins the network `web`.
- A wildcard certificate, or a host that cannot open port 80, needs the DNS-01 challenge
  (`dnschallenge`) with the credentials of the DNS provider, from a secret.
- To try the configuration without the limits of Let's Encrypt, add
  `--certificatesresolvers.le.acme.caserver=https://acme-staging-v02.api.letsencrypt.org/directory`,
  then remove it and the volume `letsencrypt` before the real start.
- On a host with SELinux or AppArmor, add `privileged: true` to `socket-proxy`: the README of
  docker-socket-proxy runs it privileged because those systems block its connection to the socket.
- Keep the versions pinned: `traefik:v3.7.16` and `ghcr.io/tecnativa/docker-socket-proxy:v0.5.0`.
  No tool of botica updates them.
