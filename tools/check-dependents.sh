#!/usr/bin/env bash
# Build every application that compiles these crates, against the working tree.
#
# This is the whole reason the three-repository split is affordable. Standalone
# repositories mean a breaking change to a shared crate is no longer one atomic
# commit that compile-checks both applications — which is exactly the drift the
# shared-crate design exists to prevent. So the check moves here: a break shows
# up in *this* repository's PR, not weeks later inside an app.
#
# Run it from anywhere. Dependents are found as sibling checkouts.
#
#   tools/check-dependents.sh
#
# Exit code is non-zero if any dependent fails to build or test.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
siblings="$(cd "$here/.." && pwd)"

# name -> path to the manifest cargo should be pointed at.
# Ksav is listed but not yet wired: it gains its girsa-source dependency in W4,
# and until then building it proves the checkout is healthy and nothing more.
dependents=(
  "girsa:$siblings/Girsa/Cargo.toml"
  "ksav:$siblings/Ksav/ksav/engine/Cargo.toml"
)

failed=()
skipped=()

for entry in "${dependents[@]}"; do
  name="${entry%%:*}"
  manifest="${entry#*:}"

  if [[ ! -f "$manifest" ]]; then
    echo "== $name: SKIPPED (no checkout at $manifest)"
    skipped+=("$name")
    continue
  fi

  echo "== $name: cargo build --all-targets"
  if ! cargo build --manifest-path "$manifest" --all-targets; then
    failed+=("$name (build)")
    continue
  fi

  echo "== $name: cargo test"
  if ! cargo test --manifest-path "$manifest"; then
    failed+=("$name (test)")
  fi
done

# Building both dependents proves they still compile against this tree. It does
# not prove they still agree with each other — and the Source Packet, which is
# the thing they agree *about*, is defined in this repository. Ksav asserts
# against a fixture of a packet Girsa really produced, so a change here can
# quietly move the producer away from that fixture while every build stays
# green. Girsa owns the check; this runs it, because this is the repository
# whose change would cause it.
fixture_check="$siblings/Girsa/tools/check-ksav-fixture.sh"
if [[ -f "$fixture_check" && -f "$siblings/Ksav/ksav/engine/tests/fixtures/girsa-packet.json" ]]; then
  echo "== girsa -> ksav: the packet against the fixture the pen asserts on"
  if ! bash "$fixture_check"; then
    failed+=("girsa->ksav (packet fixture)")
  fi
else
  skipped+=("girsa->ksav packet fixture")
fi

echo
if [[ ${#skipped[@]} -gt 0 ]]; then
  # Loud, because a silently skipped dependent is a check that passes by not
  # running — the failure mode this script exists to prevent.
  echo "SKIPPED (checkout missing): ${skipped[*]}"
fi

if [[ ${#failed[@]} -gt 0 ]]; then
  echo "FAILED: ${failed[*]}"
  echo
  echo "A change in sefer-crates broke a dependent. Fix it here, or bump the"
  echo "version and update the dependent deliberately — do not land the break."
  exit 1
fi

echo "OK: every present dependent builds and tests green against this tree."
