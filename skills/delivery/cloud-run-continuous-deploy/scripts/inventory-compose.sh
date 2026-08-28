#!/usr/bin/env bash
# Read-only inventory of a docker-compose project, classified for Cloud Run.
#
# Writes nothing. Prints a table, then a starter worksheet after a marker line.
# Resolution goes through `docker compose config` so anchors, `extends` and
# interpolation are already expanded — this script never parses YAML itself.
set -euo pipefail

COMPOSE_FILE=''
while [[ $# -gt 0 ]]; do
  case "$1" in
    --compose-file)
      [[ $# -ge 2 && -n "$2" ]] || { printf 'inventory-compose: missing value for --compose-file\n' >&2; exit 2; }
      COMPOSE_FILE="$2"
      shift 2
      ;;
    *)
      printf 'inventory-compose: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
done

command -v docker >/dev/null 2>&1 || {
  printf 'inventory-compose: docker is required to resolve the compose file\n' >&2
  exit 2
}
command -v python3 >/dev/null 2>&1 || {
  printf 'inventory-compose: python3 is required\n' >&2
  exit 2
}

if [[ -n "$COMPOSE_FILE" ]]; then
  CONFIG="$(docker compose --file "$COMPOSE_FILE" config --format json)"
else
  CONFIG="$(docker compose config --format json)"
fi

# The config travels in the environment, not on stdin: a heredoc already owns
# stdin here, so a pipe would be swallowed and python would read this script.
CONFIG="$CONFIG" python3 - <<'PY'
import json
import os
import sys

STATEFUL = {
    "postgres": "Cloud SQL, Supabase or AlloyDB",
    "mysql": "Cloud SQL",
    "mariadb": "Cloud SQL",
    "redis": "Memorystore",
    "valkey": "Memorystore",
    "memcached": "Memorystore",
    "minio": "Cloud Storage",
    "elasticsearch": "Vertex AI Search or self-hosted GCE",
    "opensearch": "Vertex AI Search or self-hosted GCE",
    "rabbitmq": "Pub/Sub",
    "kafka": "Pub/Sub",
    "clickhouse": "BigQuery or self-hosted GCE",
    "neo4j": "self-hosted GCE",
    "qdrant": "Vertex AI Vector Search or self-hosted GCE",
    "weaviate": "Vertex AI Vector Search or self-hosted GCE",
    "chroma": "Vertex AI Vector Search or self-hosted GCE",
}


def image_family(image):
    # "docker.io/library/postgres:16" -> "postgres"
    return image.split("/")[-1].split(":")[0].lower()


def first_port(service):
    for entry in service.get("ports") or []:
        target = entry.get("target") if isinstance(entry, dict) else None
        if target:
            return int(target)
        if isinstance(entry, str) and ":" in entry:
            return int(entry.rsplit(":", 1)[1].split("/")[0])
    for entry in service.get("expose") or []:
        return int(str(entry).split("/")[0])
    return None


def named_volumes(service):
    names = []
    for entry in service.get("volumes") or []:
        source = entry.get("source") if isinstance(entry, dict) else str(entry).split(":")[0]
        kind = entry.get("type") if isinstance(entry, dict) else None
        if source and not source.startswith((".", "/")) and kind != "bind":
            names.append(source)
    return names


config = json.loads(os.environ["CONFIG"])
services = config.get("services") or {}

rows = []
for name, service in sorted(services.items()):
    image = service.get("image") or ""
    family = image_family(image) if image else ""
    port = first_port(service)
    volumes = named_volumes(service)
    managed = STATEFUL.get(family, "")

    if managed:
        role, note = "externalize", managed
    elif volumes:
        role, note = "externalize", f"named volume {volumes[0]!r}; Cloud Run has no persistent disk"
    elif not service.get("build"):
        role, note = "externalize", "no build context; nothing to deploy from this repo"
    elif port:
        role, note = "http", f"port {port}"
    else:
        role, note = "job", "no published port"

    rows.append((name, role, port, note, service))

width = max((len(r[0]) for r in rows), default=7)
print(f"{'service'.ljust(width)}  role         notes")
print(f"{'-' * width}  -----------  -----")
for name, role, _, note, _ in rows:
    print(f"{name.ljust(width)}  {role.ljust(11)}  {note}")

http = [r for r in rows if r[1] == "http"]
print()
print(f"{len(rows)} service(s): {len(http)} http, "
      f"{len([r for r in rows if r[1] == 'job'])} job, "
      f"{len([r for r in rows if r[1] == 'externalize'])} to externalize.")
if len(http) > 1:
    print("More than one http service: single-container topology is available. "
          "Confirm path prefixes with the operator before generating.")

print()
print("--- worksheet ---")
print("# Starter deploy/cloud-run.map.yml. Every value below is a guess except the")
print("# service names. Confirm each one with the operator before generating.")
print("project_id: CHANGEME")
print("region: us-central1")
print("integration_region: us-central1   # optional; defaults to region")
print("ar_repo: CHANGEME")
print("topology: single-container")
print("branches:")
print("  dev: dev")
print("  prod: main")
print('review_tag_pattern: "review/pr-*/*"')
print("services:")
for name, role, port, _, service in rows:
    print(f"  - compose: {name}")
    print(f"    role: {role}")
    if role == "http":
        print(f"    port: {port}")
        print(f"    path_prefix: /{name}")
    elif role == "job":
        command = service.get("command")
        if isinstance(command, str):
            command = command.split()
        print(f"    command: {json.dumps(command or [])}")
    else:
        family = image_family(service.get("image") or "")
        print(f"    managed: {STATEFUL.get(family, 'CHANGEME')}")

# Emitted as a guess like everything else, rather than left out. Absent from the
# worksheet the key is invisible: the generator has no answer for
# {{RESET_ENTRYPOINT}} and no prompt to ask the operator for one.
print("reset_entrypoint: ./reset.sh   # optional; integration only. Delete this line")
print("                               # if the repo has no reset entrypoint -- and then")
print("                               # delete the reset step from deploy-integration.yml")
print("                               # too. See SKILL.md Phase 1.")
PY
