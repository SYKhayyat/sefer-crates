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

1. **Exact version pins.** Both apps depend on `=0.3.0`, not `^0.2`. Taking a
   new version is a deliberate act on each side.
2. **CI here builds both dependents.** `tools/check-dependents.sh`, run by
   `.github/workflows/ci.yml`, builds and tests Girsa and Ksav against the
   proposed change, so a break surfaces in *this* repository's pull request
   rather than weeks later inside an app.
3. **The Source Packet carries a schema version.** A mismatched pair fails
   loudly at the handshake instead of quietly mis-rendering a citation.

## 0.3.0 — `girsa-cite`, and a whole sefer you can point at

`girsa-cite` was a scaffold with an enum in it. It is now the formatter, and
both applications compile it: **one implementation of what a citation is**, so
the app that produces one and the app that prints one cannot disagree.

```rust
cite(&sefer, &r, CiteStyle::HebrewFull)   // שולחן ערוך, אורח חיים סימן א' סעיף א'
cite(&sefer, &r, CiteStyle::HebrewShort)  // שולחן ערוך, אורח חיים א', א'
cite(&sefer, &r, CiteStyle::English)      // Shulchan Arukh, Orach Chayim 1:1
```

The test that governs it is `every_citation_printed_here_reads_back_as_the_ref_it_came_from`:
every citation the formatter prints is put through `resolve` and has to come
back as the ref it was printed from. The formatter and the resolver are two
halves of one claim — that a mareh makom in a Ksav document means a place in
the library — and a printed form the resolver cannot read is a citation this
system cannot follow.

It refuses two things. It **does not invent an abbreviation**: `שו"ע או"ח` is
printed only if the caller supplies it as the title, because nothing in the
data says which of a work's 44 title variants a citation should use. And it
**does not invent the word for a level** — `סימן`, `סעיף`, `דף` come from the
schema's `heSectionNames`, and a work whose schema never said is cited by
number, which is an ordinary way to write a mekor.

Two things in `girsa-ref` moved to make that hold, and both were found by
writing the formatter:

- **Section words are the corpus's, all 42 of them.** `SECTION_WORDS` was a
  list of nine Hebrew words somebody thought of. Across Sefaria's 6,595 schemas
  there are 42 distinct `heSectionNames`, and the formatter prints citations
  with those. `שורה` was missing, so `ברכות דף ב. שורה א'` — a citation Girsa
  itself had just printed — resolved to `2a:שורה:1`. Three levels, one of them
  a word, and it resolved. Five are deliberately still absent (`תורה`,
  `תלמוד`, `ספר`, `תפילה`, `מדרש`): they are level names *and* how seforim are
  called, and dropping them turns `משנה תורה הלכות תפילה` into a citation of
  nothing in particular. The 2,970-citation regression corpus stays at 100.00%.
- **A whole sefer can be written down.** `girsa:bavli/berakhot` cannot mean the
  masechta — the last component is the address, always, so it reads back as the
  work `bavli` at a section called `berakhot`, with no error. A **trailing
  slash** now says the address is absent: `girsa:bavli/berakhot/`. The form
  without it keeps meaning exactly what it meant, so nothing already written
  down changes underfoot.

## 0.2.0 — the hyphen

`girsa-ref` read a hyphen anywhere in an address as the separator between the
two ends of a span. Section names contain hyphens: Sefaria really does call one
`כסלו-טבת` and another `שער חמישי - שער ייחוד המעשה`. So
`girsa:tur/orach-chayim:240:1` read back as a **range from `orach` to
`chayim:240:1`** — a place-shaped thing that is not a place, resolving without
an error to somewhere nobody asked for.

Girsa found this in W8 and worked around it on its own side, by never writing a
hyphen into an id. That left the misreading here, waiting for the next caller.
Fixed at the source instead:

- **A hyphen separates two addresses only when the side after it is addressed
  entirely by number** — a daf, a siman, a se'if, a perek — and the side before
  it reads as an address at all. `2a:1-2b:4` is still a span.

  The two sides are deliberately not treated alike, and the corpus is why.
  Requiring both to be numbered was the first rule, and it refused 11,806
  distinct citations shaped like `Abarbanel on Torah, Exodus 27:20:1-14` —
  comments 1 to 14 on one pasuk, opening on a *named* level, because a
  commentary on Chumash is divided by book before it is divided by anything
  numbered. The **end** of a span is what has to be unmistakable, and it always
  is: a closing end is written short and numeric, while the second half of a
  name never is.
- `resolve` applies the same rule, so it cannot hand back a ref the parser will
  read differently.
- **`Ref::is_well_formed`** — whether a ref survives being written down and read
  back. The grammar has three separators and no escape, so a caller building a
  ref out of text it did not choose (a schema's section name, a heading from a
  file) can find out *before* the ref is stored in a document.

The version is a coordinated bump: every crate here moves to 0.2.0 together and
both apps take the new pin in the same change.

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
