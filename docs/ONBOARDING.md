# Onboarding

Your first day in `sefer-crates`. Read this once, top to bottom; it should take
about twenty minutes and save you considerably more.

If you only have five minutes, do this and come back later:

```sh
git clone https://github.com/SYKhayyat/sefer-crates
cd sefer-crates
cargo test
```

The pinned toolchain installs itself on that first command. Everything else on
this page explains what you just built and how not to break it.

---

## 1. What this repository is, in one paragraph

Two applications — [Girsa](https://github.com/SYKhayyat/girsa), the Torah
library, and [Ksav](https://github.com/SYKhayyat/ksav), the Hebrew writing
app — ship separately and build separately. This repository holds the parts
where they are not allowed to disagree: the packet they pass to each other,
what a citation is, how Hebrew text is normalized, and the loopback link
between them. Both applications compile this code. That is the whole design:
a shared *type* drifts at compile time or not at all, where a shared *document*
drifts in silence.

Read that again with the emphasis on **not allowed to disagree**. Nearly every
rule in this repository follows from it, including the ones that look like
bureaucracy.

## 2. Prerequisites

| Thing | Why |
|---|---|
| `rustup` | Installs the pinned toolchain in [`rust-toolchain.toml`](../rust-toolchain.toml) automatically. Do not install a bare `rustc`. |
| `git` | Obviously. |
| `bash` | Only for `tools/check-dependents.sh`. Git Bash is fine on Windows; the script is tested there. |
| ~10 GB free disk | Cargo builds are not small, and the cross-repo check builds two more applications on top. See [TROUBLESHOOTING](TROUBLESHOOTING.md#errors-that-make-no-sense-in-code-you-did-not-touch). |

Sibling checkouts of Girsa and Ksav are **optional** for building these crates
and **required** for the cross-repo check. Section 6 covers the layout.

## 3. Build it

```sh
cargo build --all-targets     # everything, including tests and examples
cargo test                    # unit, integration, and doc tests
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
```

Those four commands are exactly what CI runs in the `crates` job
([`.github/workflows/ci.yml`](../.github/workflows/ci.yml)). If they pass
locally, that job passes. There is deliberately no separate "CI-only" step — a
check you cannot run at your desk is a check you will learn about too late.

The doc comments contain runnable examples, and `cargo test` runs them. A
broken example in a `//!` header is a failing test, not a cosmetic problem.

## 4. The crates, and how they stack

Seven crates, one shared version, bumped as a unit.

```
girsa-hebrew          normalization; depends on nothing
      |
girsa-ref             refs, parsing, resolution, redirects
      |          |
girsa-cite   girsa-source        formatting  /  the wire packet
                   |
             girsa-ksav          writing Ksav/Typst markup

girsa-alive           process liveness; depends on nothing
      |
girsa-post            the localhost link between the two apps
```

| Crate | What it owns | The one thing to know |
|---|---|---|
| [`girsa-hebrew`](../crates/girsa-hebrew) | nikud, prefixes, ktiv male/chaser, gershayim, abbreviations | Two operations, and the difference is the product: `normalize` produces the *normal form* and never destroys a word; `variants` produces *other surface forms* and is offered to the reader, never applied behind their back. |
| [`girsa-ref`](../crates/girsa-ref) | canonical refs, citation parsing, offline resolution, the redirect table | **Ambiguity is surfaced as a choice, never guessed.** A wrong ref resolves, opens a page, and the page is wrong — and if it reached a Ksav document, it is wrong in a printed sefer. |
| [`girsa-cite`](../crates/girsa-cite) | citation formatting | The only formatter in the system, and it is Girsa's. Ksav has no formatter at all; it prints `packet.display` and asks `/refresh` when the style changes. A formatter Ksav cannot reach cannot disagree with Girsa's. |
| [`girsa-source`](../crates/girsa-source) | the Source Packet — the wire contract | Carries `PACKET_SCHEMA_VERSION`. Adding a **required** field is a compile error at every construction site; adding an **optional** one is not, deliberately, so a newer producer can still talk to an older consumer. |
| [`girsa-ksav`](../crates/girsa-ksav) | writing real Ksav/Typst markup | Lives here, not in Ksav, because Girsa has a Ksav buffer in it and both sides must agree on what a quote block *is*. Escaping is the whole of the danger: quoted sefer text routinely contains `#`. |
| [`girsa-post`](../crates/girsa-post) | `/insert`, `/open`, `/cite`, `/health` over loopback | Binds `127.0.0.1` on an OS-chosen port, token-gated, and there is **no configuration for the bind address** — a configurable one is a typo away from `0.0.0.0`. `send` uses `POST` when it has a body and `GET` when it does not; the method is not a per-endpoint choice. |
| [`girsa-alive`](../crates/girsa-alive) | is that pid still running? | One-directional contract. `Gone` is trustworthy; `Alive` and `Unknown` mean only *go on and ask*, because pids are reused. The **only** crate permitted `unsafe`. |

### The three rules the linter enforces

Declared in the root [`Cargo.toml`](../Cargo.toml), so they apply everywhere:

1. **`unsafe_code = "forbid"`** — workspace-wide. `girsa-alive` opts out, and
   every `unsafe` block in the workspace is in that one `lib.rs`. It exists so
   that "is this process alive" has somewhere to live; the question has no safe
   spelling.
2. **`unwrap_used = "deny"`, `expect_used = "deny"`** — in library code, allowed
   in tests. A panicking resolver is a worse outcome than one that returns
   `Ambiguous`.
3. Everything Clippy says, at `-D warnings`.

If you find yourself fighting rule 2, the answer is almost never `#[allow]`. It
is usually that the function should return a `Result` or an explicit "I don't
know" variant. That is the design, not an obstacle to it.

## 5. Making your first change

The loop is short:

```sh
# edit
cargo test                       # fast; catches most of it
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt
```

Two habits worth forming on day one:

**Write the test as a sentence.** Look at the integration test filenames already
in the tree: `a_caller_can_ask_what_labels_a_level.rs`, `hyphen_in_an_address.rs`,
`the_words_the_corpus_uses_for_a_level_are_read_as_labels`. The name states the
claim. When one fails, the failure reads as a false sentence rather than as
`test_parse_3`.

**Change the doc comment in the same commit.** The headers here are not
decoration; several of them record a claim that was once wrong and how it was
corrected. `tools/check-dependents.sh` actively verifies one such claim (see
§7). A doc comment that outlives its code is the failure mode this repository
exists to prevent, one layer up.

### When your change is behavioural, measure it

The corpus fixtures exist for this.
`crates/girsa-ref/fixtures/citations.tsv` and
`crates/girsa-hebrew/fixtures/corpus-regression.tsv` are regression corpora,
and the CHANGELOG entries quote before/after counts against real shelves — see
the 0.5.4 entry, which moved 555 citations from *missed* to *landed*. If you
change a resolver, produce that table. "It looks better" is not a release note.

## 6. Developing across the three repositories

Put the three checkouts side by side. The script, CI, and both applications'
path overrides all assume this exact shape:

```
<parent>/
  Girsa/          the library app
  Ksav/           the writing app
  sefer-crates/   this repository
```

The two applications depend on these crates **differently**, and the asymmetry
is deliberate:

- **Girsa** carries both `version` and `path` in its manifests. `path` points at
  the sibling checkout for day-to-day work; when these crates are published,
  cargo uses `version` and ignores `path`. It feels like one workspace without
  being one.
- **Ksav** carries `version` plus **`git` + `rev`**. A bare `path` to a sibling
  of its checkout root broke `git clone ksav && cargo build` at `cargo
  metadata`, before a compiler ever ran. For work that spans the seam,
  `Ksav/.cargo/config.toml.example` restores the sibling layout with a `paths`
  override — and explains, at length, why `paths` and not `[patch]`.

The short version of that explanation, because you will want it: `[patch]`
re-resolves the dependency graph and rewrites the lock file; `paths` substitutes
sources for crates that are *already resolved* and leaves the lock file
byte-identical. When the thing you are verifying is "does this tree build the
app," you do not want the resolver moving underneath you.

### The check that pays for the split

```sh
bash tools/check-dependents.sh
```

Run it before you push anything that changes a public item. It builds and tests
**both** applications against your working tree, so a break surfaces in this
repository's pull request rather than weeks later inside an app when someone
finally bumps a pin.

To convince yourself it works: rename a public item in `girsa-hebrew` and run
it. Both applications go red, here, before the change can reach either of them.

Mechanically, the script installs a `paths` override into each dependent's
`.cargo/config.toml`, **asserts via `cargo metadata` that the override actually
took**, builds and tests, then removes the override on exit — including on
Ctrl-C. That assertion is not paranoia. The script spent part of its life
building Ksav against the last *pushed* commit rather than the working tree, and
reported success for it.

## 7. The checks that are not compilers

`tools/check-dependents.sh` does four things beyond building. Knowing they exist
will save you a confusing red run:

1. **The version pins agree.** The workspace declares one version and seven
   dependency lines pin it exactly. Seven hand-written strings that must move
   together; nothing said so until a bump left six behind.
2. **The override took**, per dependent, by reading package ids out of `cargo
   metadata` and confirming each is a `path+file://` into *this* checkout rather
   than a `git+https://...?rev=...` pin.
3. **A self-test of that comparison**, run before anything is built, against
   known-good and known-bad ids from both platforms. Every failure mode of a
   path-prefix match is silent: a pattern matching nothing reports "the override
   did not take," and one matching everything reports success without having
   compared anything. Both read like a working script.
4. **"Both applications" is true where it is claimed.** A crate whose doc
   comment says it is compiled into both applications must be named by both
   applications' manifests — direct dependencies only. This check exists because
   `girsa-cite`'s header made that claim for a long time and Ksav had never
   named it.

## 8. Cutting a release

Every crate shares one version and moves as a unit. A consumer pins `=X.Y.Z`, so
a mismatched pair is a *resolution error* rather than a runtime surprise.

1. Bump `version` under `[workspace.package]` in [`Cargo.toml`](../Cargo.toml).
2. Bump **all seven** `girsa-* = { version = "=X.Y.Z", ... }` lines under
   `[workspace.dependencies]` in the same file. `check-dependents.sh` will catch
   you if you miss one, but catch yourself first.
3. `cargo build` so `Cargo.lock` picks up the new version. Commit it.
4. Write the [CHANGELOG](../CHANGELOG.md) entry. Look at the existing ones for
   the register: what was wrong, why the old reading was reasonable, what the
   new rule is, and — for anything behavioural — a measured before/after table.
5. `bash tools/check-dependents.sh`, and let it go green.
6. Push. CI runs both the `crates` job and the `dependents` job.
7. Update the pins in Girsa and Ksav **deliberately**, one at a time. That is
   the point of exact pins: taking a new release is an act, not a drift.

## 9. Licence, and what you may not copy from

MIT OR Apache-2.0. The dual licence is forced by the code shared with Ksav.

Zayit, HebMorph and Sefaria-ElasticSearch are all copyleft. Nothing may be
copied from any of them into this repository. If you are reaching for prior art
on Hebrew morphology, check the licence first.

## 10. Where to go next

- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) — skim the headings now, so you
  recognise a symptom when you meet it.
- [`../CHANGELOG.md`](../CHANGELOG.md) — the best available history of *why*
  things are the way they are.
- The `//!` header of whichever crate you are about to touch. They are long on
  purpose, and they answer questions this page does not.
