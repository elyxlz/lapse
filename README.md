# lapse

A minimal spaced-repetition flashcard app. Tauri + Svelte + SQLite.

Built because Anki is powerful but its desktop UX is rough. lapse keeps the
parts that matter — FSRS scheduling, audio playback, your data on disk — and
strips the rest.

## What lapse is

- **One deck = one `.lapse` file** (SQLite under the hood). Open it, review,
  close. No collection database, no profiles, no syncing service. Legacy
  `.db` files still open for backwards compatibility.
- **Opinionated schema.** Every card has four fields: `front`, `back`, `audio`
  (BLOB, optional), `tags`. No note types, no card templates, no Mustache.
- **FSRS scheduling.** Modern algorithm via [rs-fsrs](https://github.com/open-spaced-repetition/rs-fsrs).
- **Keyboard-first.** Space to flip, `1`/`2`/`3`/`4` to rate, `r` to replay
  audio, `Esc` to leave.

## What lapse is not

- An Anki importer. lapse only reads `.db` files in its own schema.
- An editor. Decks are built outside the app (Python, scripts, whatever).
- A sync service. Your deck is one file — back it up however you back up
  any file.

## Schema

```sql
meta(key TEXT PK, value TEXT)         -- 'schema_version', 'name', etc.

cards(
  id            INTEGER PRIMARY KEY,
  front         TEXT NOT NULL,
  back          TEXT NOT NULL,
  audio         BLOB,
  audio_mime    TEXT,                  -- e.g. 'audio/mpeg'
  tags          TEXT NOT NULL DEFAULT '',  -- space-separated
  state         INTEGER NOT NULL DEFAULT 0, -- 0=new, 1=learning, 2=review, 3=relearning
  due           INTEGER NOT NULL DEFAULT 0, -- unix ms
  stability     REAL NOT NULL DEFAULT 0,
  difficulty    REAL NOT NULL DEFAULT 0,
  reps          INTEGER NOT NULL DEFAULT 0,
  lapses        INTEGER NOT NULL DEFAULT 0,
  last_review   INTEGER                 -- unix ms, nullable
)

review_log(id, card_id, reviewed_at, rating, elapsed_days, scheduled_days, state_before)
```

`src-tauri/src/schema.sql` is the source of truth.

## Building a deck

Anything that writes SQLite in the v1 schema can build a deck. The repo
ships builder scripts as a Claude Code skill under
`.claude/skills/deck-builder/` (see its `SKILL.md` for full docs). The
quick path:

```bash
# Tiny 5-card sample (smoke test)
python3 .claude/skills/deck-builder/make_sample_deck.py sample.lapse

# Convert anki-lebanese Python data into a lapse deck
python3 .claude/skills/deck-builder/import_lebanese.py lebanese.lapse

# Add free neural TTS audio (Microsoft edge-tts, ar-LB-LaylaNeural by default)
uv run .claude/skills/deck-builder/fetch_edge_tts.py lebanese.lapse
```

The TTS script is resumable — it only synthesizes cards whose `audio`
column is currently NULL.

To make a deck available inside the app, drop it into the persistent
deck folder:

```bash
# Linux
cp lebanese.lapse ~/.local/share/dev.lapse.app/decks/
# macOS
cp lebanese.lapse ~/Library/Application\ Support/dev.lapse.app/decks/
```

## Development

Prerequisites:

- Rust 1.93+ (stable)
- Node 22+
- Fedora: `sudo dnf install librsvg2-devel webkit2gtk4.1-devel openssl-devel gtk3-devel`
- Debian/Ubuntu: `sudo apt install librsvg2-dev libwebkit2gtk-4.1-dev libssl-dev libgtk-3-dev`

```bash
npm install
npm run tauri dev
```

## License

MIT. See `LICENSE`.
