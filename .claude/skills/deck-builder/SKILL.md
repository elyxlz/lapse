---
name: deck-builder
description: Build and audio-fill lapse `.db` decks. Use when the user asks to create a new deck, convert external data into the lapse schema, embed TTS audio, or change voices.
---

# deck-builder

Scripts for producing lapse `.db` files. Decks are self-contained SQLite
databases in the v1 schema; the app reads them from the persistent deck
folder or via the "Open external .db…" picker.

Schema is canonical at `src-tauri/src/schema.sql`. Any new script you
write here must mirror that schema exactly.

## Layout

```
.claude/skills/deck-builder/
├── SKILL.md                 (this file)
├── make_sample_deck.py      ← minimal example deck (smoke test)
├── import_lebanese.py       ← example: converts an external Python corpus
└── fetch_edge_tts.py        ← embeds neural TTS audio in any language
```

## Scripts

### `make_sample_deck.py` — minimal example deck
Five-card deck with the SCHEMA inlined and a few hardcoded
`(front, back, tags)` tuples. Use it as a template when adding a new
builder, or to confirm the app still opens a fresh deck after schema
changes.

```bash
python3 .claude/skills/deck-builder/make_sample_deck.py sample.lapse
```

### `import_lebanese.py` — example data-import script
One concrete example of converting an external Python corpus (lists of
tuples / nested dicts) into a lapse deck. Bidirectional vocab entries
become two cards (forward and reverse), nested conjugation tables get
flattened to plain text, and HTML tables in grammar entries are stripped.

Use it as a reference when writing an importer for your own data
source. The patterns to copy:
- bidirectional cards share an entry → emit two rows per source row
- multi-line content (tables, paradigms) goes into `back` as `\n`-joined
  text — the app renders it verbatim
- tags are space-separated and lowercase

```bash
python3 .claude/skills/deck-builder/import_lebanese.py output.db
```

### `fetch_edge_tts.py` — neural TTS audio
Populates `audio` BLOB + `audio_mime` for every card whose front or back
contains text in a target script. The defaults are tuned for Arabic
(scans for `U+0600..U+06FF`); change `ARABIC_RE` and `DEFAULT_VOICE` to
target a different language.

Uses Microsoft's free `edge-tts` endpoint via the MIT-licensed Python
wrapper — no signup, no key, no rate cap beyond the upstream service's
throttling.

```bash
uv run .claude/skills/deck-builder/fetch_edge_tts.py deck.lapse
# Pick a different voice:
uv run .claude/skills/deck-builder/fetch_edge_tts.py deck.lapse fr-FR-DeniseNeural
# Enumerate all voices:
uv run edge-tts --list-voices
```

**Resumable.** The script only updates cards where `audio IS NULL`, so
interrupting and re-running picks up the remainder. Failed strings stay
NULL and a re-run retries them.

**Dedup-aware.** When multiple cards share the same target text
(bidirectional vocab pairs do), the script synthesizes once and attaches
the same BLOB to every matching card ID.

**To replace audio on an existing deck** (e.g. switching voices):
```sql
UPDATE cards SET audio = NULL, audio_mime = NULL;
```
then re-run the script with the new voice argument.

## Deploying a deck

The app scans a persistent deck folder on the home screen. Drop your
finished `.db` there:

```bash
# Linux
cp deck.lapse ~/.local/share/dev.lapse.app/decks/

# macOS
cp deck.lapse ~/Library/Application\ Support/dev.lapse.app/decks/
```

## Writing a new deck-builder script

1. Mirror the SCHEMA constant from `make_sample_deck.py` exactly (or
   read `src-tauri/src/schema.sql` and `executescript` it).
2. Insert rows with `INSERT INTO cards(front, back, tags) VALUES (?, ?, ?)`
   and leave the FSRS columns at their defaults — the app initializes
   them on first review.
3. Don't set `audio` from the builder script unless you're sure of the
   mime type. Prefer running `fetch_edge_tts.py` as a separate pass so
   the audio policy stays in one place.
4. Tags are **space-separated strings** — not arrays, not JSON.
   Lowercase, no commas. Multi-word tags use underscores.
5. Set `meta.name` if you want the home-screen list to show a friendly
   name instead of the filename stem.

## Common voices

`fetch_edge_tts.py` works with any `edge-tts` voice ID. A few examples:

| Voice ID                | Language |
|-------------------------|----------|
| `en-US-AriaNeural`      | English (US) — female |
| `en-GB-RyanNeural`      | English (UK) — male |
| `fr-FR-DeniseNeural`    | French |
| `es-ES-ElviraNeural`    | Spanish (Castilian) |
| `de-DE-KatjaNeural`     | German |
| `it-IT-ElsaNeural`      | Italian |
| `ja-JP-NanamiNeural`    | Japanese |
| `zh-CN-XiaoxiaoNeural`  | Mandarin |
| `ar-SA-ZariyahNeural`   | Arabic (MSA reference) |
| `ar-EG-SalmaNeural`     | Egyptian Arabic |
| `ar-LB-LaylaNeural`     | Levantine Arabic (female) |
| `ar-LB-RamiNeural`      | Levantine Arabic (male) |

Run `uv run edge-tts --list-voices` for the full list.
