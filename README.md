# sefer-crates

The shared Rust crates behind **[Girsa](https://github.com/SYKhayyat/girsa)**
(the Torah library app) and **[Ksav](https://github.com/SYKhayyat/ksav)** (the
Hebrew writing app). Both applications live in their own repositories and each
builds and ships on its own, but the pieces where they have to *agree* — the
wire format they exchange, how a citation is parsed and printed, how Hebrew text
is normalized, and the localhost link between them — live here, compiled into
both, so the two sides cannot drift apart.

This repository is the contract. It is for anyone working on Girsa or Ksav who
needs to change something that crosses between them, and for anyone who wants to
understand how the two apps stay in sync without being one codebase.

## Quick start

```sh
git clone https://github.com/SYKhayyat/sefer-crates
cd sefer-crates
cargo test
```

That is the whole setup. The pinned toolchain in
[`rust-toolchain.toml`](rust-toolchain.toml) installs itself on first build; you
need `rustup` and nothing else. Sibling checkouts of Girsa and Ksav are needed
only for the cross-repo check described below.

## Documentation

| | |
|---|---|
| [docs/ONBOARDING.md](docs/ONBOARDING.md) | Start here if you are new. What each crate owns, the rules the linter enforces, the change loop, the three-repository layout, and how to cut a release. |
| [docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md) | Symptom-first. Build failures, denied lints, every message `check-dependents.sh` can print, pin drift, and the CI-versus-desk differences. |
| [CHANGELOG.md](CHANGELOG.md) | Release history, and the best record of *why* things are the way they are. |

## The crates

| Crate | Purpose |
|---|---|
| `girsa-source` | The Source Packet — the wire contract exchanged between the two apps |
| `girsa-ref` | Canonical refs, citation parsing, offline resolution, the redirect table |
| `girsa-hebrew` | Hebrew normalization: nikud, prefixes, ktiv male/chaser, gershayim, abbreviations |
| `girsa-cite` | Citation formatting — one implementation of what a citation is |
| `girsa-post` | The loopback between the apps: token-gated, localhost only, and the `girsa://` deep links |
| `girsa-ksav` | Writing real Ksav/Typst markup — the document language both apps produce |
| `girsa-alive` | Is a process still running? The one crate here that may call the operating system |

`unsafe_code` is **forbidden** workspace-wide. `girsa-alive` is the sole
exception — asking whether a process is alive has no safe spelling — and every
`unsafe` block in the workspace is contained in its `lib.rs`.

Panicking paths (`unwrap`, `expect`) are denied in library code and allowed only
in tests: a wrong ref is worse than no ref, so a resolver returns an ambiguity
rather than panicking.

## Why a separate repository, and how the cost is paid

Girsa and Ksav each stand alone: neither is unbuildable or unreleasable because
of the other. The price of that independence is that a breaking change to a
shared crate is no longer one atomic commit that compile-checks both apps —
which is the drift these crates exist to prevent. Three things pay that price,
and none of them are optional:

1. **Exact version pins.** Both apps depend on `=<version>`, not `^`. Taking a
   new release is a deliberate act on each side.
2. **CI here builds both dependents.** `tools/check-dependents.sh`, run by
   `.github/workflows/ci.yml`, builds and tests Girsa and Ksav against the
   proposed change, so a break surfaces in *this* repository's pull request
   rather than weeks later inside an app.
3. **The Source Packet carries a schema version.** A mismatched pair fails
   loudly at the handshake instead of quietly mis-rendering a citation.

Every crate shares one version (`workspace.package.version` in the root
`Cargo.toml`) and is bumped as a single unit. See [CHANGELOG.md](CHANGELOG.md)
for the release history, and
[docs/ONBOARDING.md](docs/ONBOARDING.md#8-cutting-a-release) for the bump
checklist.

## Requirements

- The pinned Rust toolchain in [`rust-toolchain.toml`](rust-toolchain.toml)
  (currently 1.97.1). `rustup` installs it automatically on first build.
- `bash` and sibling checkouts of Girsa and Ksav to run the cross-repo check
  (see below). The crates themselves build with `cargo` alone.

## Build and test

From a clone of this repository:

```sh
cargo build --all-targets
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
```

These four commands are exactly what the `crates` CI job runs. If they pass at
your desk, that job passes. `cargo test` covers unit, integration and doc tests
— the runnable examples in the `//!` headers are tests, not decoration.

## The cross-repo check

```sh
bash tools/check-dependents.sh
```

This is the check that makes the three-repository split affordable. It builds
and tests both Girsa and Ksav against the working tree of these crates, found as
sibling checkouts, so a breaking change is caught in *this* repository's PR. To
see it work, rename a public item in `girsa-hebrew` and run it — both apps go
red here, before the change can reach either of them.

The script installs a `paths` override to point each dependent at this working
tree, asserts (via `cargo metadata`) that the override actually took, and removes
it on exit, including on Ctrl-C.

It also checks three things that are not compilers: that the seven exact version
pins still match the workspace version, that no crate's doc comment claims to be
"compiled into both applications" while one manifest does not name it, and that
the Source Packet still matches the fixture Ksav asserts against. Each message
it can print is listed in
[docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md#the-cross-repo-check).

## Developing across the three repositories

The three repositories sit side by side:

```
Videos/
  Girsa/          the library app
  Ksav/           the writing app
  sefer-crates/   this repository
```

The two apps depend on these crates differently, by design:

- **Girsa** carries both `version` and `path` in its manifests. The `path`
  points at the sibling checkout for day-to-day work; when these crates are
  published, cargo uses `version` and ignores `path`. It feels like one
  workspace without being one.
- **Ksav** carries `version` and **`git` + `rev`**, because a bare path to a
  sibling of its checkout root breaks `git clone ksav && cargo build` at
  `cargo metadata`. For work that spans the seam, `Ksav/.cargo/config.toml.example`
  restores the sibling-checkout layout with a `paths` override; that file explains
  why `paths` and not `[patch]`.

If a change of yours appears in one app and not the other, that asymmetry is
the reason — see
[docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md#the-two-applications).

## Licence

MIT OR Apache-2.0 — see [`COPYRIGHT`](COPYRIGHT), [`LICENSE-MIT`](LICENSE-MIT)
and [`LICENSE-APACHE`](LICENSE-APACHE). The dual licence is forced by the code
shared with Ksav. Note that Zayit, HebMorph and Sefaria-ElasticSearch are all
copyleft and none of them may be copied from.
