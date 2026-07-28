# sefer-crates

The contract shared by **[Girsa](https://github.com/SYKhayyat/girsa)** (the
library) and **[Ksav](https://github.com/SYKhayyat/ksav)** (the writing app).

Both applications compile these crates. That is the entire reason the two-app
system works: one citation formatter compiled into both means the app that
*produces* citations and the app that *prints* them cannot disagree — precisely
the class of bug that would destroy trust in the pairing.

| Crate | Purpose |
|---|---|
| `girsa-source` | The Source Packet. The wire contract. |
| `girsa-ref` | Refs, citation parsing, offline resolution, the redirect table |
| `girsa-hebrew` | Normalization — nikud, prefixes, ktiv male/chaser, gershayim, abbreviations |
| `girsa-cite` | Citation formatting — one implementation, both apps |

## The cost of three repositories, and how it is paid

Girsa and Ksav each stand alone: neither is unbuildable or unreleasable because
of the other. The price is that a breaking change here is no longer one atomic
commit that compile-checks both applications — which is the drift the shared
crates exist to prevent in the first place.

Three things pay it, and none of them are optional:

1. **Exact version pins.** Both apps depend on `=0.1.0`, not `^0.1`. Taking a
   new version is a deliberate act on each side.
2. **CI here builds both dependents.** `tools/check-dependents.sh`, run by
   `.github/workflows/ci.yml`, builds and tests Girsa and Ksav against the
   proposed change, so a break surfaces in *this* repository's pull request
   rather than weeks later inside an app.
3. **The Source Packet carries a schema version.** A mismatched pair fails
   loudly at the handshake instead of quietly mis-rendering a citation.

## Develop

The three repositories sit side by side:

```
Videos/
  Girsa/          the library app
  Ksav/           the writing app
  sefer-crates/   this repository
```

Each app's dependency carries both `version` and `path`, so day-to-day work
across all three feels like one workspace without them being one. The `path`
points at the sibling checkout; when these crates are published, `version` is
what cargo uses and `path` is what it ignores.

## Verify

```sh
cargo build --all-targets
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
bash tools/check-dependents.sh     # the cross-repo check, run exactly as CI runs it
```

`check-dependents.sh` is the one that matters. To see it work, rename a public
item in `girsa-hebrew` and run it: Girsa goes red here, in this repository,
before the change can reach either app.

## Licence

MIT OR Apache-2.0 — see [`LICENSE`](LICENSE). Forced by the sharing with Ksav,
and worth understanding before adding a dependency: Zayit, HebMorph and
Sefaria-ElasticSearch are all copyleft and none of them may be copied from.
