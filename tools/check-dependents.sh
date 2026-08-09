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
#
# # "Against the working tree" was not true, and that is the point of the file
#
# Ksav pins these crates by **git rev**, deliberately (see the note above the
# girsa dependencies in `Ksav/ksav/engine/Cargo.toml`: a `path` to a sibling of
# the checkout root meant `git clone ksav && cargo build` failed at `cargo
# metadata`, before a compiler ran). Which means this script was building Ksav
# against **the last pushed commit**, not against the tree it is supposed to be
# checking. Renaming a public item here and running this went green.
#
# Two of the three dependents' worth of safety net, quietly not there — in the
# one script whose header calls itself the whole reason the split is affordable.
#
# So the override goes in first. `paths` and not `[patch]`, for the reason
# `Ksav/.cargo/config.toml.example` sets out at length: `[patch]` re-resolves and
# rewrites the lock file, `paths` substitutes sources for crates already resolved
# and leaves it byte-identical. And it is **asserted**, not assumed: `cargo
# metadata` is asked where each girsa crate actually came from, because an
# override that silently stopped applying is this script going green for the
# second time in its life for the same reason.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
siblings="$(cd "$here/.." && pwd)"

# The same directory, spelled the way cargo spells it inside a `file://` URL.
# Under Git Bash `$here` is `/c/Users/…` and cargo emits `C:/Users/…`, so a
# substring comparison between the two silently never matches — which would make
# the assertion below fail every run on Windows, or, written the other way
# round, pass every run everywhere. Neither is a check.
here_url="$here"
if command -v cygpath >/dev/null 2>&1; then
  here_url="$(cygpath -m "$here")"
fi

# name -> path to the manifest cargo should be pointed at.
# Ksav is listed but not yet wired: it gains its girsa-source dependency in W4,
# and until then building it proves the checkout is healthy and nothing more.
dependents=(
  "girsa:$siblings/Girsa/Cargo.toml"
  "ksav:$siblings/Ksav/ksav/engine/Cargo.toml"
)

# Where each dependent's repository root is, for the `.cargo/config.toml` that
# the override goes in. Cargo walks up from the invocation directory, so one
# file at the repository root covers every workspace inside it.
declare -A roots=(
  ["girsa"]="$siblings/Girsa"
  ["ksav"]="$siblings/Ksav"
)

failed=()
skipped=()
installed=()

# Every crate in this workspace, by directory name. Read rather than listed, so
# a new crate is covered on the day it is added.
crates=()
for dir in "$here"/crates/*/; do
  [[ -f "$dir/Cargo.toml" ]] && crates+=("$(basename "$dir")")
done
if [[ ${#crates[@]} -eq 0 ]]; then
  echo "no crates found under $here/crates — is this the right checkout?"
  exit 1
fi

# ---------------------------------------------------------------- the override

# Put back whatever was there, whatever happens — including on Ctrl-C. A
# developer's own `.cargo/config.toml` is not this script's to eat.
restore_configs() {
  for root in "${installed[@]}"; do
    if [[ -f "$root/.cargo/config.toml.check-dependents-backup" ]]; then
      mv -f "$root/.cargo/config.toml.check-dependents-backup" "$root/.cargo/config.toml"
    else
      rm -f "$root/.cargo/config.toml"
      rmdir "$root/.cargo" 2>/dev/null || true
    fi
  done
  installed=()
}
trap restore_configs EXIT INT TERM

# Prepended, never replaced. Girsa's `.cargo/config.toml` is tracked and carries
# a linker choice and a job count; eating it for the duration of a build would
# change what is being measured. `paths` is a bare top-level key, so it has to go
# *above* the first `[table]` header or TOML reads it as a member of that table —
# which is why this prepends rather than appending.
install_override() {
  local root="$1" existing=""
  mkdir -p "$root/.cargo"
  if [[ -f "$root/.cargo/config.toml" ]]; then
    cp -f "$root/.cargo/config.toml" "$root/.cargo/config.toml.check-dependents-backup"
    existing="$(cat "$root/.cargo/config.toml")"
  fi
  installed+=("$root")
  {
    echo "# Prepended by sefer-crates/tools/check-dependents.sh. Removed when it exits."
    echo "paths = ["
    for c in "${crates[@]}"; do
      # `$here_url`, not `$here`. Under Git Bash `$here` is `/c/Users/…`, and
      # cargo on Windows resolves a leading `/` against the config file's own
      # drive — so it looked for `C:\c\Users\…`, which is not anywhere. The
      # comparison twenty lines down already knew this and used `cygpath -m`;
      # the line that *writes* the path did not, which is the same file
      # disagreeing with itself about what a path is.
      echo "    \"$here_url/crates/$c\","
    done
    echo "]"
    echo
    printf '%s\n' "$existing"
  } > "$root/.cargo/config.toml"
}

# Did it take?
#
# Each package's `id` in `cargo metadata` carries where it came from — a path
# dependency is `path+file:///…`, a git one is `git+https://…?rev=…`. That is
# exactly the distinction being asserted, in one field, so this reads ids rather
# than manifest paths. (`manifest_path` would be the obvious choice and is a
# trap: `targets` sits between the name and it, so any line-splitting scheme
# separates them.)
#
# grep and not a JSON parser because `jq` is not on every runner, and because a
# false *negative* here is the failure this whole block exists against — so the
# check is that **every** girsa package resolves to a path, and that at least one
# was found at all. A graph with no girsa crates in it would otherwise pass by
# being empty, which is the shape this project keeps rebuilding.

# Run cargo from the dependent's own root, not from here.
#
# Cargo discovers `.cargo/config.toml` by walking up from the **current working
# directory**, not from `--manifest-path`. So every cargo call in this file read
# *this* repository's config and never saw the override that had just been
# written into the dependent's — and the assertion above reported, correctly,
# that the override had not taken. The header says "run it from anywhere", and
# that sentence was the bug: true of where the script sits, false of where cargo
# looks.
in_root() {
  local root="$1"
  shift
  (cd "$root" && "$@")
}

override_took() {
  local manifest="$1" name="$2"
  local meta
  if ! meta="$(in_root "${roots[$name]}" cargo metadata --format-version 1 --manifest-path "$manifest" 2>/dev/null)"; then
    echo "== $name: cargo metadata failed — cannot verify the override took"
    return 1
  fi
  local ids elsewhere=() found=0
  ids="$(printf '%s' "$meta" | tr ',' '\n' | grep -o '"id":"[^"]*"' | sort -u)"
  for c in "${crates[@]}"; do
    # Two spellings, because cargo omits the name when the directory already
    # carries it: `path+file:///…/girsa-ksav#0.5.3` and
    # `path+file:///…/girsa-source#girsa-hebrew@0.5.3` are both this crate.
    local mine
    mine="$(printf '%s' "$ids" | grep -E "(#$c@|/$c#)" || true)"
    [[ -z "$mine" ]] && continue
    while IFS= read -r id; do
      [[ -z "$id" ]] && continue
      found=$((found + 1))
      case "$id" in
        '"id":"path+file:///'*"$here_url"/*) ;;
        *) elsewhere+=("$id") ;;
      esac
    done <<< "$mine"
  done

  if [[ $found -eq 0 ]]; then
    echo "== $name: no girsa crate is in this dependency graph at all"
    return 1
  fi
  if [[ ${#elsewhere[@]} -gt 0 ]]; then
    echo "== $name: the path override did NOT take — these came from elsewhere:"
    printf '     %s\n' "${elsewhere[@]}"
    echo "     (so the build below would have checked the pinned commit, not this tree)"
    return 1
  fi
  echo "== $name: $found girsa package(s), all resolved to this checkout"
  return 0
}

# ------------------------------------------------------------- the pins agree

# The workspace declares one version and six dependency lines pin it exactly.
# Seven hand-written strings that have to move together, three lines apart, and
# nothing said so until a bump left the other six behind and `cargo test` failed
# with "failed to select a version" in a repository whose own manifest names it.
version="$(grep -m1 '^version = "' "$here/Cargo.toml" | sed 's/^version = "//; s/"$//')"
stale="$(grep -oE 'girsa-[a-z]+ = \{ version = "=[0-9.]+"' "$here/Cargo.toml" \
  | grep -v "\"=$version\"" || true)"
if [[ -n "$stale" ]]; then
  echo "== Cargo.toml: the workspace is $version and these pins are not:"
  printf '     %s\n' "$stale"
  failed+=("sefer-crates (version pins)")
fi

# ------------------------------------------------------------ the dependents

for entry in "${dependents[@]}"; do
  name="${entry%%:*}"
  manifest="${entry#*:}"

  if [[ ! -f "$manifest" ]]; then
    echo "== $name: SKIPPED (no checkout at $manifest)"
    skipped+=("$name")
    continue
  fi

  install_override "${roots[$name]}"
  echo "== $name: path override installed at ${roots[$name]}/.cargo/config.toml"
  if ! override_took "$manifest" "$name"; then
    failed+=("$name (override did not take — it was built against the pin, not this tree)")
    continue
  fi

  echo "== $name: cargo build --all-targets"
  if ! in_root "${roots[$name]}" cargo build --manifest-path "$manifest" --all-targets; then
    failed+=("$name (build)")
    continue
  fi

  echo "== $name: cargo test"
  if ! in_root "${roots[$name]}" cargo test --manifest-path "$manifest"; then
    failed+=("$name (test)")
  fi
done

restore_configs

# ------------------------------------------ who actually compiles which crate
#
# `girsa-cite/src/lib.rs` opened with *"one implementation, compiled into both
# applications"*, and Ksav has never named it in a manifest. The 9 August report
# found that sentence in three places; correcting those three left two more —
# the crate's own header and this repository's README — because the sweep was
# the quoted line numbers rather than the claim.
#
# A doc comment cannot check itself, and this script is the one place with both
# checkouts on disk. So: a crate whose documentation says *both applications*
# has to be named by both applications' manifests. Direct dependencies only —
# transitively compiled is not the claim, and `girsa-ref` reaching Ksav through
# `girsa-ksav` would make the loose version of this pass for a crate no Ksav
# code path can call.
#
# It declines to run rather than passing when a checkout is missing, for the
# same reason the skip list at the end is loud.
if [[ ${#skipped[@]} -eq 0 ]]; then
  echo "== the shared crates: is *both applications* true where it is claimed?"
  lying=()
  for c in "${crates[@]}"; do
    lib="$here/crates/$c/src/lib.rs"
    [[ -f "$lib" ]] || continue
    # Asserted, not quoted. `girsa-cite`'s header now *records* the old claim —
    # "this said 'compiled into both applications', and it never was" — and a
    # sweep that could not tell a claim from a citation of one would force every
    # correction in this repository to be written without naming what it
    # corrects. So a phrase with a quotation mark in front of it on the same
    # line is somebody quoting, which is the same rule the prohibition suites
    # draw around `lamdan/` and `docs/`.
    claim="$(grep -iE 'compiled into both|both applications compile|both apps compile' "$lib" \
      | grep -vE '["“][^"“]*(compiled into both|both applications compile|both apps compile)' || true)"
    [[ -z "$claim" ]] && continue
    named_by=()
    for name in "${!roots[@]}"; do
      # A dependency line, not a mention: `girsa-cite = { … }` or
      # `girsa-cite.workspace = true` at the start of a line in a manifest.
      if grep -rqE "^[[:space:]]*$c([[:space:]]*=|\.workspace)" \
        --include=Cargo.toml "${roots[$name]}" 2>/dev/null; then
        named_by+=("$name")
      fi
    done
    if [[ ${#named_by[@]} -lt 2 ]]; then
      lying+=("$c: says 'both applications'; named by: ${named_by[*]:-neither}")
    fi
  done
  if [[ ${#lying[@]} -gt 0 ]]; then
    echo "   a claim in a doc comment that the manifests do not support:"
    printf '     %s\n' "${lying[@]}"
    failed+=("sefer-crates (a crate claims both applications and one does not compile it)")
  else
    echo "   yes, everywhere it is claimed"
  fi
fi

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
