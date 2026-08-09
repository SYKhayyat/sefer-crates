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
| `girsa-post` | The loopback between them: token-gated, localhost only, and the `girsa://` links |
| `girsa-ksav` | Writing real Ksav markup — the document language, so Girsa's buffer and Ksav agree |

## The cost of three repositories, and how it is paid

Girsa and Ksav each stand alone: neither is unbuildable or unreleasable because
of the other. The price is that a breaking change here is no longer one atomic
commit that compile-checks both applications — which is the drift the shared
crates exist to prevent in the first place.

Three things pay it, and none of them are optional:

1. **Exact version pins.** Both apps depend on `=0.5.3`, not `^0.5`. Taking a
   new version is a deliberate act on each side.
2. **CI here builds both dependents.** `tools/check-dependents.sh`, run by
   `.github/workflows/ci.yml`, builds and tests Girsa and Ksav against the
   proposed change, so a break surfaces in *this* repository's pull request
   rather than weeks later inside an app.
3. **The Source Packet carries a schema version.** A mismatched pair fails
   loudly at the handshake instead of quietly mis-rendering a citation.

## 0.5.3 — the seam stops keying on English prose

### `PostError::code()`, and two frontends that stop reading English

`PostError` is the one error type that **crosses between the repositories**, and
both applications have to say something about it to a Hebrew reader. Both did it
by regular expression over its English `Display` — four character-identical
regexes, `/could not reach|timed out|timeout/i`, `/refused it\b/`,
`/permission denied|access is denied/i`, `/no such file|os error 2\b/i`, in
`Girsa/app/src/trouble.ts` and `Ksav/app/src/diagnostics.ts`.

Every word of these strings was load-bearing API between two repositories, in
the crate that exists so the two sides need not agree in prose. Reword one and
both halves stay green while a reader stops being told what happened.

Girsa had already written this fix for its **own** error type and tested it:
`girsa_app::trouble::Code` prints as `no-index: there is no index here`, its
frontend keys on the name before the colon, and `trouble.test.mjs` asserts
*"rewording the prose changes nothing a reader sees."* It had never been applied
here.

- **`PostError::code() -> Option<&'static str>`**, and the code is the first
  thing `Display` prints, because what crosses to a frontend is a *string*.
- **`PostError::CODES`**, so each side can sweep it. A frontend with no line for
  a code prints English into a Hebrew UI, which is the bug Girsa's `presence.ts`
  and `trouble.ts` both name as their reason for existing.
- **`Io` and `Json` return `None` on purpose.** They forward the operating
  system's failure and serde's. Naming them `post-io` would claim a vocabulary
  this crate does not own, and would stop a frontend reading the words that
  actually separate `permission denied` from `no such file` — which a reader
  needs, and which only the OS's own string carries. Matching somebody else's
  prose is honest; doing it to your own is what this ends.

Both sides now key on the code and both hold `CODES` against their tables from
Rust: `Ksav/ksav/engine/tests/from_girsa.rs` and Girsa's
`the_rules_this_repository_wrote_down.rs`.


### The escape list, made checkable

`escape`'s character list is `pub const MARKUP` now. It had ten characters and
Ksav's editor had five — `\ [ ] # $` against `# [ ] \ $ * _ < > @` — and both
write `#מראה_מקום(מקור: …)[…]` out of the same Girsa `display` string. The five
missing are `*` (strong), `_` (emph), `<`/`>` (a label) and `@` (a ref), all of
which occur in Sefaria titles: one source, two doors, two different documents.

The list itself did not move here. It lives in Ksav's `engine/src/escape.rs`,
because this crate is a **native-only** dependency there — a browser build has no
loopback to Girsa — while the escaper is needed in every build. Ksav's
`engine/tests/from_girsa.rs` holds the two lists together, feeding the whole
character set through both functions rather than comparing two constants.

### The marks table, made reachable

`girsa-hebrew` had the right answer about what a Hebrew word boundary is, and
Ksav could not get at it. Five items were `pub(crate)`: `fold_final`,
`fold_quote_mark`, `FINAL_FORMS`, `CANONICAL_GERESH`, `CANONICAL_GERSHAYIM`.
Two predicates did not exist at all — `is_geresh` and `is_gershayim`, kept
separate because Hebrew uses the two marks differently and a reader that cannot
tell them apart gets both rules wrong.

That mattered more than an ordinary missing accessor, because the crate was
**already inside Ksav's binary** — resolved through `girsa-source` → `girsa-ref`
— while `ksav/engine/src/spell/hebrew.rs` hand-wrote `is_hebrew_mark` as the
whole `U+0591–U+05C7` block with nothing excluded. Four characters in that block
separate words: ־ maqaf, ׀ paseq, ׃ sof pasuq, ׆ nun hafukha. Stripping them
glued `אֶת־הַשָּׁמַיִם` into `אתהשמים`, and since sof pasuq ends every verse, the
Hebrew speller silently declined to check **every unpointed pasuk**. Its lexicon
builder had made the identical omission in Python, so the shipped dictionary
carried the glue as vocabulary and the two wrong copies agreed with each other.

Nothing in this crate was wrong. It was unreachable, which turned out to be the
same thing.

## 0.5.2 — an English Ksav document has a shape too

`read()` matched **Hebrew command names only**. Every command in Ksav is bound
twice — `#כותרת1` and `#h1`, `#רשימה` and `#bullets` — and an English document
uses the second throughout, so an English sefer came off the shelf as an
undifferentiated run of paragraphs: no headings, therefore no levels in the
address; no items, no rows, and every footnote spliced back into the middle of
its sentence.

Nothing errored. `Role::Inline` is the *correct* answer for a name nobody knows
— it is what keeps a new style command in Ksav from losing a word — and that is
precisely what made this silent. It is the same defect the module was written
against, one language over.

- **`ALIASES`** and **`PARAM_ALIASES`**, both public. Thirty-eight structural
  commands and the two argument names, in both spellings. Names are normalised
  through them before any decision is made, so `role()` stays a single Hebrew
  match rather than growing a second arm per row.
- The settings family is a **prefix in Hebrew and a suffix in English** —
  `#הגדרות_כותרות` is `#headings_config` — so its English half could not be a
  row of the table and is matched as `_config`.
- The two tables are kept apart, because `ממוספרת` is a command (`numbered`)
  *and* a parameter (`numbered:`), and one flat table over both would answer
  whichever it met first.

The pairs are checked where they can be: **Ksav's `engine/tests/from_girsa.rs`**
holds every row against `typst/ksav.typ`, which is the thing that actually binds
both spellings. Girsa cannot run that check — it has no prelude — so the test
lives in the dependent rather than the dependency. Here, the fence is a
translation: the same document in both spellings has to read to the same blocks,
and the Hebrew half is asserted to be more than a flat run of paragraphs, or the
equality would hold for the wrong reason.

Also: `html_root_url` carried a hand-written version in all six crates, and all
six had already drifted from the workspace version. The version is gone rather
than corrected — docs.rs resolves the crate to its latest — so there is nothing
left to go stale.

## 0.5.1 — which characters of the place a quote actually was

Additive: one optional field on the Source Packet, one optional argument on the
markup writer. No schema bump — `PACKET_SCHEMA_VERSION` stays 1, because an
optional field is exactly what serde fills in for an older producer.

- **`SourcePacket::range`.** A ref names *places*. A reader who highlights half
  a se'if and presses Ctrl+C gets the words they highlighted, and the packet
  said so in `text` and said nothing about **which** words those were — so the
  promise that makes the whole pairing worth building, *regenerate every quote
  against a corrected edition without touching the prose*, regenerated the whole
  se'if. Two sentences in the specification, both true, contradicting each other
  at the regeneration step. `Range { from, to }` counts characters of the text
  **as it was shown** — the only offsets the two ends can agree about, because
  they are the ones the reader was looking at when they dragged. `None` is *no
  producer ever recorded one*; `Some(Range::all())` is *the reader chose the
  whole place*, and the two are different answers to *what should regenerating
  hand back*. [`SourcePacket::part`] is the constructor that takes one.
- **`mekor` writes it into the document.** `#מראה_מקום(מקור: "girsa:…",
  תווים: "4-19")`, half-open, `"4-"` for *to the end*, and **absent for the
  whole place** — which is what every document written before this argument
  existed already says, and why they are all still right. A packet that never
  reached a document has nowhere to keep its range, and *where did I use this*
  would have been the only thing the field was ever good for.
- **`cited_in` reads it back**, and `refs_in` is now that with the ranges
  dropped and the repeats removed. Two scanners over the same markup would be
  two answers to *what does this document cite*, and they would disagree the
  first time either grew an argument.

## 0.5.0 — the transport's security model, and thousands

Three security defects in `girsa-post` and two correctness ones, from the
2026-07-30 grade. **Breaking:** `Desk::serve` takes `&mut self`.

- **`Desk::drop` left the listener bound.** `serve` cloned the `Arc<Server>` into
  a detached thread; dropping the `Desk` dropped one `Arc` and the thread held the
  other, so the endpoint file was withdrawn and the port stayed open — a live
  loopback listener with a token written down nowhere, reachable by anything that
  scans ports and **not revocable**, because there was nothing left to revoke.
  The whole model of this crate is *"a token in a file only you can read"*, and a
  listener outside that model is the one thing it cannot survive. `Drop` now calls
  `Server::unblock` and joins, so the handle has to be kept and `serve` takes
  `&mut self`. That is the breaking change, and it is one word at each of the two
  call sites.
- **The token file was created world-readable and then chmodded.** `fs::write`
  followed by `set_permissions(0o600)` means the file is born `0o666 & !umask` —
  0644 on a stock system — with a window in which any local user could read the
  token. It is now created at 0600 with `create_new`, and the directory holding it
  is 0700 rather than `create_dir_all`'s 0755.
- **A body of unknown length was half-accepted.** `body_length().unwrap_or(0)`
  reads chunked-with-no-`Content-Length` as zero, so it passed the 413 check and
  was then silently cut by `take(MAX_BODY)`. For the plain-text bodies this
  crate's own tests use, a truncated quote arrives looking like a complete one.
  Now a `411`. `Content-Length` is also no longer trusted: the read takes one byte
  past the ceiling and refuses if it gets there.
- **The client capped responses at nothing.** `read_to_string` with no limit,
  guarded only by a 400 ms *per-read* timeout, which a sender producing a byte
  every 300 ms never trips. There is now a `MAX_ANSWER` mirroring the desk's
  `MAX_BODY` and a `WHOLE_EXCHANGE` deadline as well as the per-read one.
- **Deep links: the scheme is case-insensitive and the decode is not lossy.** RFC
  3986 makes a scheme case-insensitive and PDF viewers normalise it, so
  `GIRSA:bavli/berakhot/2a:1` was refused as *"a misconfigured machine"*. And
  `from_utf8_lossy` turned `%FF` into U+FFFD silently, so a corrupted link became a
  *different, valid-looking* ref — in the crate whose governing rule is that a
  wrong ref is worse than no ref. Both fixed; `decode` returns `Option`.
- **`girsa-ref`: numbers past a thousand are written in Hebrew.** `to_hebrew` gave
  up at 1,000 and returned Arabic digits, switching alphabets inside a citation
  without saying so, in a formatter whose promise is *"how it is written in a
  sefer"*. The reasoning behind the ceiling was sound and its premise was wrong:
  measured over the real corpus, **43,076 of 5,000,545 addresses (0.86%) carry a
  component ≥ 1,000**, the first `girsa:bavli/maadaney-yom-tov-on-berakhot/1000`.
  `parse_hebrew` had always read `א'תתקצ"ט`; `to_hebrew` now writes it. A *round*
  thousand still goes in digits, because 1,000 and 1 are both `א'` in the notation
  and this crate refuses ambiguity rather than guessing at it —
  `is_written_in_digits` says which of the two a number gets. The round-trip test
  runs to 20,000.

## 0.4.0 — a Ksav document read for its shape, not only its words

`girsa_ksav::read` turns a document into blocks — heading, paragraph, quote, list
item, table row, footnote — and `to_text` is that reading rendered flat, so there
is one parser and not two.

`to_text` used to take the command names and their arguments off and keep what
was between the brackets. Measured against Ksav's own sample document, that lost:

```
#כותרת1[מבוא]                  → "מבוא" as body text, not a heading
#רשימה(פריט[א], פריט[ב])        → nothing at all
#טבלה(עמודות: 2, תא[א], תא[ב])  → nothing at all
סוף#הערה[הערה].                 → "סוף הערה ." — spliced into the sentence
```

A list's items and a table's cells live in the *arguments* — Ksav writes
`#רשימה(פריט[…])`, not `#רשימה[…]` — and arguments were skipped because arguments
are usually settings. So a document's lists and tables were absent from anything
reading it, and it did not look like a loss.

Of Ksav's 104 commands it knows the forty that are structure. Everything else is
inline and its content is kept, so a new style command in Ksav needs no change
here and **cannot lose a word by being unknown** — which is exactly how the tables
were lost. Blocks come out flat in reading order, with an item carrying its depth
and a note carrying the marker left in the text; a faithful tree would be one
nothing addressing it could name.

## 0.3.0 — `girsa-cite`, `girsa-post`, and a whole sefer you can point at

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

### `girsa-post` — the loopback

New crate, and it is here for the same reason the others are: the two ends of a
protocol that live in different repositories drift, and a protocol both ends
compile cannot.

```text
Girsa ──POST /insert──▶ Ksav      a source, into the open document
Ksav  ──POST /open  ──▶ Girsa     a citation, to the page it names
Ksav  ──POST /cite  ──▶ Girsa     print this ref in that style
Ksav  ──POST /quote ──▶ Girsa     the words again, from the corpus as it stands
                                  (with the packet's `range`, or the whole place)
either ─GET  /health──▶ other     is my sibling there?
```

`127.0.0.1` on a port the operating system picks, published in a per-user file
with a token minted from the operating system's randomness. **Localhost is not
private** — every process on the machine can reach a loopback port, and so can
a web page — so the token is required on every path including `/health`, it
travels in a header rather than a URL, and the desk sends no CORS headers and
answers no preflight, which is what stops a browser tab that guessed both.

`presence()` *asks*: an endpoint file left behind by a crash is `Stale` with
the reason, not `Live`. That is what lets each application show whether its
sibling is there, so an affordance is never offered when it would fail.

And the deep links, whose short form is the interesting one: `girsa://open?ref=…`
works, and so does a bare `girsa:bavli/berakhot/2a:1` — because **a ref is
already a URI**. The string a Ksav document has been storing all along is a
link that lands, which is why the citation in Girsa's HTML clipboard flavour is
an `<a href="girsa:…">`: paste a quote into Word, print it to PDF, and the mekor
in the PDF opens the page.

### `girsa-ksav` — the document language

Girsa has a Ksav buffer in it (spec.md §10.3), and *lightweight means the UI,
not the format*: the buffer writes real Ksav markup from the first keystroke.
For that to be true, the two applications have to agree about what a quote
block **is** — and an agreement in prose between two repositories is the thing
this repository exists to replace. So `#ציטוט[…]`, `#מראה_מקום[…]`,
`#הערת_עורך[…]` and the escaping are written here, and Ksav's own `source`
module is a wrapper around them.

The split of labour is exact: this crate asserts that it *wrote* `#ציטוט[…]`;
only Ksav can assert that **Typst accepts it**, and those tests stay there,
compiling with the real engine.

### The ref in the document, and what it makes possible

`girsa-ksav`'s `mekor` now writes

```
#מראה_מקום(מקור: "girsa:shulchan-arukh/orach-chayim/1:3")[שולחן ערוך, אורח חיים סימן א' סעיף ג']
```

The ref is **stored and not printed**, which is what spec.md §10.2 has always
said and what the markup was quietly not doing. Three things in the two
applications are that one change seen from different sides: a mareh mekomos is
a sort and a print, *where did I use this* is a scan rather than a guess, and a
citation in a compiled PDF is a link that opens the page it names.

`live_citation` is what linkify writes — the words print exactly as they were
typed, with the ref underneath — and `to_text` reads a document back as the
words somebody wrote, which is how Girsa puts your own writing on the shelf
without indexing `#כותרת1[`.

### Two things `girsa-ref` moved

Both were found by writing the formatter:

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

The two apps depend on these crates differently, and the difference is
deliberate rather than an oversight anybody is getting round to.

**Girsa** carries `version` and `path` — the `path` points at the sibling
checkout, and when these crates are published `version` is what cargo uses and
`path` is what it ignores. Day-to-day work across the two feels like one
workspace without them being one.

**Ksav** carries `version` and **`git` + `rev`**. A path to a sibling of its
checkout root meant `git clone ksav && cargo build` failed at `cargo metadata` —
before a compiler ran, naming a directory the reader had never heard of — and
four of its five CI jobs carried a second `actions/checkout` to fake the desk
layout. For the days you are moving the seam, `Ksav/.cargo/config.toml.example`
restores the sibling checkout with a `paths` override; the file explains at
length why `paths` and not `[patch]`.

## Verify

```sh
cargo build --all-targets
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
bash tools/check-dependents.sh     # the cross-repo check, run exactly as CI runs it
```

`check-dependents.sh` is the one that matters. To see it work, rename a public
item in `girsa-hebrew` and run it: both apps go red here, in this repository,
before the change can reach either of them.

That used to be one app. Ksav pins by rev, so the script was building it against
**the last pushed commit** rather than against the tree it exists to check —
renaming a public item and running it went green. Two of the three dependents'
worth of safety net, quietly absent, in the script whose own header calls itself
the whole reason the split is affordable.

It installs the `paths` override itself now, prepended to whatever
`.cargo/config.toml` a dependent already has (Girsa's is tracked and carries a
linker choice) and removed on exit, including on Ctrl-C. And it **asserts the
override took**, by asking `cargo metadata` where each girsa package actually
came from: a `path+file://` under this checkout, or the run fails before a
compiler starts. An override that silently stopped applying would be this script
going green for the second time in its life for the same reason.

## Licence

MIT OR Apache-2.0 — see [`LICENSE`](LICENSE). Forced by the sharing with Ksav,
and worth understanding before adding a dependency: Zayit, HebMorph and
Sefaria-ElasticSearch are all copyleft and none of them may be copied from.
