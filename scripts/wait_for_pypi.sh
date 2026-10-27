#!/bin/sh
# Wait until uv resolves botica==VERSION from PyPI. Used by the jobs `mirror` and `mirror-check`
# of .github/workflows/botica-release.yml.
#
# The JSON API of PyPI lists an upload before the simple index that uv and prek read: in the
# release of 0.14.2 the JSON API had the version and `uvx prek` in the next job did not. So the
# check is a resolution by uv, with no cache and no configuration: the `exclude-newer` of the
# pyproject.toml of this repository (CES-128) leaves out a release younger than 7 days. The wait
# between checks doubles from 5 seconds up to 160, and the script gives up after 10 minutes.
set -u

version="${1:?usage: wait_for_pypi.sh VERSION}"
budget=600
delay=5
waited=0

while :; do
    if echo "botica==$version" | uv pip compile --no-cache --no-config --quiet --no-header --python-version 3.9 - >/dev/null 2>&1; then
        echo "uv resolves botica $version after ${waited}s"
        exit 0
    fi
    if [ "$waited" -ge "$budget" ]; then
        echo "uv does not resolve botica $version after ${waited}s" >&2
        exit 1
    fi
    [ $((waited + delay)) -gt "$budget" ] && delay=$((budget - waited))
    echo "botica $version is not in the index yet; next check in ${delay}s"
    sleep "$delay"
    waited=$((waited + delay))
    delay=$((delay * 2))
    [ "$delay" -gt 160 ] && delay=160
done
