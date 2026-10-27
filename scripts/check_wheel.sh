#!/bin/sh
# Run the botica of an installed wheel: the job `wheels` of .github/workflows/botica-release.yml
# runs it on each wheel before it attaches the wheel to the release, so a wheel that does not
# install or does not run stops the release before `publish`. `botica` and `python` are the ones
# of the environment where the wheel is installed, first on the PATH.
set -eu

version="${1:?usage: check_wheel.sh VERSION}"

reported="$(botica --version)"
if [ "$reported" != "botica $version" ]; then
  echo "the wheel reports \"$reported\", not \"botica $version\"" >&2
  exit 1
fi
# The catalog of rules and the recipes are embedded in the binary.
botica rules list >/dev/null
botica recipe list >/dev/null
python -c "import botica.sdk"
echo "botica $version runs from the wheel"
