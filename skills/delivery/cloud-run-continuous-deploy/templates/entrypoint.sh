#!/usr/bin/env bash
# Run every HTTP service plus nginx in one Cloud Run container.
#
# bash, not sh: `wait -n` is a bashism and dash fails on it at runtime, inside
# the container, on the first deploy. The image must therefore install bash.
set -euo pipefail

: "${PORT:=8080}"
export PORT

# Restricted to ${PORT} on purpose. Unrestricted, envsubst also eats nginx's own
# $host, $remote_addr and $proxy_add_x_forwarded_for, and the rendered config is
# silently wrong rather than obviously broken.
envsubst '${PORT}' \
  < /etc/nginx/templates/default.conf.template \
  > /etc/nginx/conf.d/default.conf

# Kill the whole process group on the way out, so no child outlives the container.
trap 'kill 0' EXIT INT TERM

{{API_COMMAND}} &
{{WEB_COMMAND}} &
nginx -g 'daemon off;' &

# Returns as soon as the FIRST child exits. A dead application therefore takes
# the container down and Cloud Run replaces the revision, instead of the
# container staying up while serving errors from half of its routes.
wait -n
exit $?
