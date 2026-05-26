---
name: deck-builder
description: Build and audio-fill lapse `.db` decks. Use when the user asks to create a new deck, regenerate the Lebanese deck, add TTS audio to a deck, change voices, or convert outside data into the lapse v1 schema.
---

# deck-builder

Scripts for producing lapse `.db` files. Decks are self-contained SQLite
databases in the v1 schema; the app reads them from the persistent deck
folder or via the "Open external .db…" picker.

Schema is canonical at `src-tauri/src/schema.sql`. Any new script you
write here must mirror that schema exactly (or use it via `include_str!`
equivalent).

## Layout

```
.claude/skills/deck-builder/
├── SKILL.md                 (this file)
├── make_sample_deck.py      ← tiny 5-card deck for smoke-testing
├── import_lebanese.py       ← anki-lebanese Python data → lapse.db
└── fetch_edge_tts.py        ← edge-tts neural audio → audio BLOBs
```

## When to use each script

### `make_sample_deck.py` — sanity-check the pipeline
A 5-card Lebanese phrase deck. Use it after schema or app changes to
confirm the app still opens a fresh deck.

```bash
python3 .claude/skills/deck-builder/make_sample_deck.py sample.db
```

### `import_lebanese.py` — rebuild the Lebanese deck
Reads `~/Repos/anki-lebanese/data_{vocab,conjugations,grammar}.py` and
emits ~1823 cards:
- Vocab is bidirectional: each entry becomes `EN→AR` and `AR→EN` cards.
- Conjugations: one card per verb × tense, formatted as plain-text tables.
- Grammar: HTML tables flattened to tab/newline text.

Lebanese pronunciation normalization (qaaf→2, emphatics→7/9/6, DH
restoration) lives in the script — keep it in sync with the original
`build_deck.py` in the anki-lebanese repo if that ever updates.

```bash
python3 .claude/skills/deck-builder/import_lebanese.py lebanese.db
```

### `fetch_edge_tts.py` — embed neural audio
Populates `audio` BLOB + `audio_mime` for every card whose front or back
contains Arabic text (any character in `U+0600..U+06FF`). Uses
Microsoft's free `edge-tts` endpoint via the MIT-licensed Python wrapper.
Default voice is `ar-LB-LaylaNeural` (Lebanese female neural).

```bash
uv run .claude/skills/deck-builder/fetch_edge_tts.py lebanese.db
# Male voice:
uv run .claude/skills/deck-builder/fetch_edge_tts.py lebanese.db ar-LB-RamiNeural
# Other languages: edge-tts --list-voices  (e.g. fr-FR-DeniseNeural, es-ES-ElviraNeural)
```

**Resumable.** The script only updates cards where `audio IS NULL`, so
interrupting and re-running picks up where it left off.

**Bidirectional-aware.** A vocab entry produces two cards sharing the
same Arabic string; the script synthesizes once and attaches the same
BLOB to both card IDs.

**To replace audio on an existing deck** (e.g. switching voices):
```sql
UPDATE cards SET audio = NULL, audio_mime = NULL;
```
then re-run the script with the new voice.

## Deploying a deck

The app scans the persistent deck folder on the home screen. Drop your
finished `.db` there:

```bash
# Linux
cp lebanese.db ~/.local/share/dev.lapse.app/decks/

# macOS
cp lebanese.db ~/Library/Application\ Support/dev.lapse.app/decks/
```

## Writing a new deck-builder script

Mirror the SCHEMA constant from `make_sample_deck.py` exactly (or read
`src-tauri/src/schema.sql` and `execute_script` it). Insert rows with
`INSERT INTO cards(front, back, tags) VALUES (?, ?, ?)` and leave the
FSRS columns at their defaults — the app initializes them on first
review.

Don't set `audio` directly from a builder script unless you're sure of
the mime type. Prefer running `fetch_edge_tts.py` as a separate pass so
the audio policy stays in one place.

Tags are **space-separated strings** (not arrays, not JSON). Lowercase,
no commas. Multi-word tags use underscores.

## Common voices

| Voice ID                | Language | Notes |
|-------------------------|----------|-------|
| `ar-LB-LaylaNeural`     | Lebanese | Female. Default. |
| `ar-LB-RamiNeural`      | Lebanese | Male. |
| `ar-EG-SalmaNeural`     | Egyptian | Useful for MSA-leaning content. |
| `ar-SA-ZariyahNeural`   | Saudi/MSA | Standard MSA reference. |
| `fr-FR-DeniseNeural`    | French   | If you build a French deck. |
| `es-ES-ElviraNeural`    | Spanish  | If you build a Spanish deck. |

Run `uv run edge-tts --list-voices` for the full list.
