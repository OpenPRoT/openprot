#!/usr/bin/env bash
# Licensed under the Apache-2.0 license
#
# Builds the release targets in checkout $1 exactly as release.yml does and
# copies their output files to directory $2. Used by reproducibility.yml.
#
# Requires RELEASE_PLATFORM and RELEASE_TARGETS in the environment.

set -euo pipefail

src="$1"
out="$2"
: "${RELEASE_PLATFORM:?}" "${RELEASE_TARGETS:?}"

mkdir -p "${out}"
cd "${src}"

bazel --nosystem_rc --nohome_rc build \
    --config=release \
    --platforms="${RELEASE_PLATFORM}" \
    ${RELEASE_TARGETS}

# cquery inherits the `test` rc settings (--platform_suffix=test,
# enable_tests=true); reset them so the paths match the build above.
bazel --nosystem_rc --nohome_rc cquery \
    --config=release \
    --platforms="${RELEASE_PLATFORM}" \
    --platform_suffix= \
    --@pigweed//pw_kernel:enable_tests=false \
    --output=files \
    ${RELEASE_TARGETS} \
  | sort -u > "${out}.paths"

if [ ! -s "${out}.paths" ]; then
  echo "::error::No output files found for ${RELEASE_TARGETS}"
  exit 1
fi
while read -r f; do
  cp -L "$f" "${out}/"
done < "${out}.paths"

(cd "${out}" && sha256sum -- *)

# Free disk before the next build; the runner has limited space.
bazel --nosystem_rc --nohome_rc clean --expunge
