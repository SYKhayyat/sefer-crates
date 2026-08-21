# Troubleshooting

Symptoms you are likely to actually hit, what causes each one, and the fix.
Ordered roughly by how often they happen.

Everything here is grounded in something that has genuinely gone wrong in this
repository. Where a failure was silent and produced a *green* run, that is
called out — those are the expensive ones.

---

## Contents

- [Build and toolchain](#build-and-toolchain)
- [Lints that look like obstacles](#lints-that-look-like-obstacles)
- [The cross-repo check](#the-cross-repo-check)
- [Versions and pins](#versions-and-pins)
- [The two applications](#the-two-applications)
- [CI](#ci)
- [When you are stuck](#when-you-are-stuck)

---

## Build and toolchain

### `error: toolchain '1.97.1' is not installed`, or a long pause on first build

Expected on a fresh machine. `rust-toolchain.toml` pins an exact version and
`rustup` fetches it on demand. Let it finish.

If it fails outright, you probably have a distribution-packaged `rustc` rather
than `rustup`. Install `rustup` from <https://rustup.rs> and make sure its
shims come first on `PATH`:

```sh
rustup --version          # must succeed
which -a cargo rustc      # rustup shims should be first
rustup show               # should report 1.97.1 as the active toolchain here
```

**Do not** "fix" this by editing `rust-toolchain.toml` to `stable`. The file
says why: an exact version is what makes this project still build years from
now.

### Errors that make no sense in code you did not touch

Symptoms: an `.rlib` reported as corrupt or truncated, a panic inside `rustc`
itself, "unexpected end of file" reading a dependency, errors pointing at
crates.io code you have never opened.

Almost always **a full disk**. A full workspace build here is multi-gigabyte,
and `check-dependents.sh` builds two more applications on top of it. When the
disk fills mid-write, `rustc` leaves truncated artifacts behind, and their
errors read like your code is broken.

```sh
df -h .                   # check first, before believing any error message
cargo clean               # then reclaim and rebuild
```

If it recurs, `cargo clean` in the two sibling checkouts as well. Their
`target/` directories are the largest thing on the disk.

### `cargo test` fails inside a doc comment

Not cosmetic. The `//!` headers contain runnable examples and `cargo test` runs
them, deliberately — a header that stopped compiling is a header that stopped
being true. Fix the example, or fix the code it describes. Do not mark it
` ```ignore ` to get green.

### `cargo fmt -- --check` fails but the code looks fine

Run `cargo fmt` with no arguments and commit the result. On Windows this is
sometimes a line-ending issue rather than a formatting one — confirm with:

```sh
git config core.autocrlf     # should be false or input for this repo
git diff --stat              # a whole-file diff means line endings, not format
```

---

## Lints that look like obstacles

### `use of a disallowed method 'unwrap'` / `'expect'`

Working as intended. `unwrap_used` and `expect_used` are **denied in library
code** and allowed in tests.

The fix is nearly never `#[allow(clippy::unwrap_used)]`. It is one of:

- return a `Result` and let the caller decide;
- return an explicit "I do not know" variant — `Resolution::Ambiguous` is the
  canonical one in this repository;
- restructure so the impossible case is not representable.

The principle behind the lint: **a wrong ref is worse than no ref.** A resolver
that panics has failed loudly, which is bad; a resolver that guesses has failed
silently into a printed sefer, which is worse. Neither is acceptable, so the
type carries the uncertainty.

If you genuinely believe an `#[allow]` is right, say why in a comment on the
same line. A bare `#[allow]` in this repository will be asked about.

### `usage of an `unsafe` block` in a crate that is not `girsa-alive`

`unsafe_code = "forbid"` is workspace-wide, and `forbid` cannot be locally
overridden — that is the point of `forbid` rather than `deny`.

`girsa-alive` is the single exception, because asking the operating system
whether a pid is alive has no safe spelling. If you need a syscall, it goes
there, behind a safe API, with the reasoning in the header. It does not go
where you happen to need it.

### Clippy passes locally and fails in CI

CI runs `--all-targets --all-features`. If you ran plain `cargo clippy`, you
checked neither the test targets nor the feature-gated code. Several crates
here have an optional `serde` feature that is off by default.

```sh
cargo clippy --all-targets --all-features -- -D warnings
```

---

## The cross-repo check

All of these come from `tools/check-dependents.sh`.

### `SKIPPED (checkout missing): girsa ksav`

The script finds dependents as **sibling checkouts**, and they are not there.

```
<parent>/
  Girsa/
  Ksav/
  sefer-crates/     <- you are here
```

The skip is printed loudly on purpose: a silently skipped dependent is a check
that passes by not running, which is the exact failure this script exists
against. A run that skips both dependents has verified nothing about them.

Note that the "both applications" claim check also declines to run when
anything is skipped, rather than passing vacuously.

### `the path override did NOT take — these came from elsewhere:`

The script writes a `paths` override into the dependent's `.cargo/config.toml`,
then asks `cargo metadata` where each `girsa-*` package actually came from. This
message means at least one came from a `git+https://...?rev=...` pin instead of
your working tree — so the build that follows would have checked **the pinned
commit, not your changes**.

This is the failure the assertion exists for. The script really did build Ksav
against the last pushed commit for part of its life, and went green doing it.

Things to check, in order:

1. **Did an earlier run leave a stale config behind?** Look for
   `Girsa/.cargo/config.toml.check-dependents-backup` and the same under `Ksav/`.
   A `kill -9` (as opposed to Ctrl-C, which is trapped) skips the restore.
   Restore by hand:
   ```sh
   mv -f Girsa/.cargo/config.toml.check-dependents-backup Girsa/.cargo/config.toml
   ```
2. **Did a crate get added or renamed?** The override lists the crate
   directories it finds under `crates/`. A crate whose directory name and
   package name disagree will not be matched by either id spelling the script
   accepts.
3. **Is the dependent's manifest pointing somewhere unexpected?** Read the
   printed ids — they name the real source.

### `no girsa crate is in this dependency graph at all`

The dependent resolved with zero `girsa-*` packages. Either you pointed the
script at the wrong manifest, or a dependency was removed from that application.

The check exists because an empty graph would otherwise pass by being empty:
"every girsa package resolved to this checkout" is trivially true of no
packages. That shape — a check that passes because it found nothing to check —
has recurred in this project often enough to be worth naming.

### `cargo metadata failed — cannot verify the override took`

The dependent checkout is not healthy enough to resolve. Go into it and run
`cargo metadata --format-version 1` yourself; the real error will be there. The
usual causes are a corrupt lock file, an unreachable git dependency, or a
half-finished merge.

### `self-test: a windows path id was not recognised` (or any self-test line)

The script refuses to run. The prefix-matching used to distinguish a path
override from a git pin is broken, so the check below it could not tell the two
apart — and would report either "did not take" on every run, or success on
every run, depending on which way it broke. Both read like a working script,
which is why the self-test runs before anything is built.

If you touched `from_this_checkout`, `here_url`, or `here_prefix`, that is
where to look. The rule the code encodes: the leading slash belongs to the
path, and the literal prefix ends at `//`. On Linux `$here_url` already carries
that slash; `cygpath -m` returns `C:/Users/...` and does not.

### On Windows: cargo looks for `C:\c\Users\...`

Under Git Bash, `$here` is `/c/Users/...` while cargo emits and expects
`C:/Users/...`. A leading `/` in a `paths` entry is resolved by cargo against
the config file's own drive, which is how `C:\c\Users\...` gets constructed.

The script handles this with `cygpath -m`. If you are writing new tooling here,
use `$here_url` (the cygpath-normalised form) anywhere a path is **written for
cargo**, and `$here` only for shell operations. The script once had exactly one
of those two lines right.

### The check goes green but nothing seems to have been checked

Read the output rather than the exit code. A correct run prints, per dependent:

```
== girsa: path override installed at .../Girsa/.cargo/config.toml
== girsa: N girsa package(s), all resolved to this checkout
== girsa: cargo build --all-targets
== girsa: cargo test
```

and ends with `OK: every present dependent builds and tests green against this
tree.` If you see `SKIPPED` above that line, the green covers less than you
think.

### `.cargo/config.toml` in a sibling looks wrong after a crash

The script **prepends** its override and restores the original on exit,
including on Ctrl-C, via a trap. A hard kill bypasses the trap.

The original is at `<root>/.cargo/config.toml.check-dependents-backup`. Note
that Girsa's `.cargo/config.toml` is **tracked** and carries a linker choice
and a job count — so `git checkout` will restore it correctly there, but Ksav's
may not be tracked. Prefer the backup file.

---

## Versions and pins

### `error: failed to select a version for girsa-<something>`

The workspace version and the exact pins have drifted apart. One version lives
in `[workspace.package]` and seven `girsa-* = { version = "=X.Y.Z", ... }` lines
in `[workspace.dependencies]` must match it. They are seven hand-written strings
that have to move together, and nothing said so until a bump left six behind.

```sh
grep -n 'version' Cargo.toml     # workspace version, then the seven pins
```

`check-dependents.sh` reports this specifically:

```
== Cargo.toml: the workspace is X.Y.Z and these pins are not:
```

Fix all of them, then `cargo build` to refresh `Cargo.lock`, and commit the
lock file.

### `a crate claims both applications and one does not compile it`

A `//!` header says the crate is compiled into both applications, and one of the
two applications' manifests does not name it as a **direct** dependency.

Two legitimate fixes:

- **The claim is wrong.** Correct the header to say what is actually true. This
  is what happened to `girsa-cite`: Ksav has never formatted a citation, and the
  header now records both the old claim and why the real guarantee is stronger.
- **The manifest is wrong.** Add the dependency, if the application really
  should be compiling it.

The check treats a phrase preceded by a quotation mark on the same line as
somebody *quoting* the claim rather than making it — so a header is allowed to
describe a correction it is documenting. If your prose is being misread as a
claim, quote it.

Transitive reachability does not count. `girsa-ref` reaching Ksav through
`girsa-ksav` would satisfy a looser check for a crate no Ksav code path calls.

---

## The two applications

### My change to a shared crate does not show up in Ksav

Ksav pins these crates by **git rev**, not by path. Until you push and Ksav's
`rev` moves, Ksav is compiling the last pushed commit — by design, so that
`git clone ksav && cargo build` works for someone who does not have this
repository at all.

For work that spans the seam, copy `Ksav/.cargo/config.toml.example` into place;
it installs the `paths` override that points Ksav at your sibling checkout. That
file explains why it uses `paths` and not `[patch]`.

`tools/check-dependents.sh` installs the same override temporarily, which is
exactly why it can check your working tree at all.

### My change shows up in Girsa but not Ksav (or vice versa)

Expected, and it is the asymmetry described in
[ONBOARDING §6](ONBOARDING.md#6-developing-across-the-three-repositories).
Girsa carries `version` + `path`; Ksav carries `version` + `git` + `rev`. Girsa
picks up your edits immediately; Ksav does not without the override.

### The packet fixture check fails

```
== girsa -> ksav: the packet against the fixture the pen asserts on
```

Ksav asserts against a fixture of a packet Girsa really produced
(`Ksav/ksav/engine/tests/fixtures/girsa-packet.json`). A change here can move
the producer away from that fixture while every build stays green — building
both applications proves they compile, not that they still agree.

Girsa owns the check (`Girsa/tools/check-ksav-fixture.sh`); this repository runs
it, because a change here is what would cause the break. If the change to the
packet is intended, regenerate the fixture on the Ksav side and land the two
together.

If you see the fixture check in the `SKIPPED` list instead, one of those two
files is missing and the agreement was not verified at all.

### Adding a field to the Source Packet

- **Optional field** (`Option<T>`, or `#[serde(default)]`): no schema bump. Serde
  fills it in, deliberately, so a newer producer can still talk to an older
  consumer.
- **Required field**: a compile error at every site that builds a packet — which
  is the point — and it needs a `PACKET_SCHEMA_VERSION` bump. A mismatched pair
  then fails loudly at the handshake instead of quietly mis-rendering a citation.

Getting this backwards is easy, and the crate header used to state it both ways
thirty lines apart. Optional does not bump. Required does.

---

## CI

### The `dependents` job fails but `crates` passes

Your change compiles here and breaks one of the applications. That is the job
working. Read its log — it runs the same `tools/check-dependents.sh` you can run
locally, so you can reproduce it at your desk with sibling checkouts.

The choice is: fix it here, or bump the version and update the dependent
deliberately. Do not land the break.

### CI green, desk red (or the reverse)

CI and your desk run the *same script* for the dependents job, on purpose, so
they cannot disagree about what "the dependents build" means. When they do
disagree, the difference is nearly always one of:

- **Toolchain.** CI installs via `dtolnay/rust-toolchain@stable`; your desk
  respects `rust-toolchain.toml`. Check `rustc --version` in both.
- **Checkout layout.** CI checks out into `sefer-crates/`, `Girsa/`, `Ksav/`
  side by side. If your local parent directory names differ, the sibling lookup
  finds nothing and skips.
- **Stale local state.** A leftover `.cargo/config.toml` in a sibling, or a
  `target/` from a different toolchain.

### A CI run reports the override did not take, on every run

Historically this was the self-test bug described above — a path-prefix pattern
that matched on Windows and nowhere else. If it reappears after a change to the
path handling, the self-test in the script is the first thing to read.

---

## When you are stuck

In this order:

1. **Read the `//!` header** of the crate you are in. They are unusually long
   and they explain the reasoning, not just the API.
2. **Read [`CHANGELOG.md`](../CHANGELOG.md).** Entries state what was wrong, why
   the old reading was reasonable, and what the new rule is. If your confusion
   is about *why* something works the way it does, it is often literally there.
3. **Read the comments in `tools/check-dependents.sh`.** That file documents
   four separate silent failures it has had, in place, above the code that fixed
   each one. It is the best worked example in the repository of how failures
   here actually behave.
4. **Check the disk.** Genuinely. See the first section.
