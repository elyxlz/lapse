# lapse — project guide

A minimal spaced-repetition flashcard app. Tauri 2 + SvelteKit (SPA mode) + SQLite + FSRS.

This file is the source of truth for design intent, conventions, and architectural
choices. Read it before making changes.

## What lapse is

- **One deck = one `.db` file.** Self-contained: cards + audio BLOBs + FSRS state + review log.
- **Persistent deck folder.** The app scans `<app_data_dir>/decks/` on the home screen
  and lists every valid `.db` it finds. Drop a file in there, restart, it shows up.
  Resolved via Tauri's `app.path().app_data_dir()` — e.g.
  `~/.local/share/dev.lapse.app/decks/` on Linux,
  `~/Library/Application Support/dev.lapse.app/decks/` on macOS.
- **External decks too.** An "Open external .db…" button picks any `.db` from
  anywhere on disk without copying it in.
- **Opinionated, fixed schema.** Four fields per card: `front`, `back`, `audio`, `tags`. No
  note types, no card templates, no Mustache, no field interpolation.
- **FSRS scheduling** via `rs-fsrs 1.2`. **Binary rating** (Easy / Hard) in the UI;
  internally Easy maps to FSRS rating 3 (Good), Hard maps to FSRS rating 1 (Again).
  The 4-button FSRS mode (Hard=2, Easy=4) is intentionally NOT exposed — the user
  prefers low cognitive load over fine-grained scheduling precision.
- **Keyboard-first.** See keymap below.

## What lapse explicitly is not

- **An importer.** No `.apkg`, no CSV, no JSON. Only `.db` files in lapse's own schema.
- **An editor.** Decks are built outside the app — Python, scripts, whatever writes
  SQLite. The app is a pure reviewer.
- **A sync service.** Your deck is one file; back it up like any other file.
- **A TTS engine.** Audio must be pre-baked into the BLOB column. Live TTS is out of scope.
- **Multi-deck.** One deck open at a time. No deck tree, no nested subdecks.

If a requested feature would compromise these constraints, push back before
implementing.

## Schema (v1)

See `src-tauri/src/schema.sql` for the canonical definition.

```sql
meta(key TEXT PK, value TEXT)
cards(
  id, front, back,
  audio BLOB, audio_mime,
  tags TEXT,                        -- space-separated
  state INT, due INT,               -- FSRS state machine
  stability REAL, difficulty REAL,
  reps INT, lapses INT, last_review INT
)
review_log(id, card_id, reviewed_at, rating, elapsed_days, scheduled_days, state_before)
```

- `state`: 0=new, 1=learning, 2=review, 3=relearning
- `due` and `last_review`: unix milliseconds
- `tags`: lowercase, space-separated, no commas

The Python sample generator (`scripts/make_sample_deck.py`) duplicates the schema —
keep it in sync with `schema.sql` when changing either.

## Design system

References: Linear, Raycast, Things 3, Mochi. Aesthetic: monochrome, hairline borders
(not boxes), hierarchy from size + weight (not color), generous whitespace, restraint
with motion, keyboard-first.

Tokens live in `src/app.css` as CSS custom properties. **Use the tokens. Do not
hard-code colors, sizes, or spacing in component styles.**

### Color
- Monochromatic palette. Dark by default (`--bg: #0b0b0b`, near-pure black).
- Hairline borders (`--border: #1d1d1d`), not boxes.
- Accent is *pure white* (`--accent: #ffffff`) — used only for the primary CTA.
- Rating colors (`--r-again` / `--r-hard` / `--r-good` / `--r-easy`) appear **only on
  the rating row**. Never elsewhere.
- Light theme via `[data-theme="light"]`.

### Type
- Font: Inter (system fallback). Mono: JetBrains Mono. RTL: SF Arabic / Noto Naskh Arabic.
- Scale: `--t-xs` 11 / `--t-sm` 13 / `--t-base` 15 / `--t-lg` 18 / `--t-xl` 24 /
  `--t-2xl` 36 / `--t-3xl` 56.
- Weights: 400 body, 500 emphasis, 600 headings only. Never bolder.
- Card front text auto-detects RTL via `֐-￼` regex and switches font + direction.

### Spacing
- 4px base scale: `--s-1` 4 / `--s-2` 8 / `--s-3` 12 / `--s-4` 16 / `--s-6` 24 /
  `--s-8` 32 / `--s-12` 48 / `--s-16` 64 / `--s-24` 96.
- Use multiples of 4. Never 5, 7, 9, etc.

### Radii
- `--r-sm` 4 / `--r` 6 / `--r-lg` 10 / `--r-xl` 16. Default to `--r`.

### Motion
- Hovers: `120ms cubic-bezier(0.4, 0, 0.2, 1)` (`--fast`).
- View transitions / card flips: `200ms` (`--mid`).
- No bounces, no spring animations, no decorative motion.

### Chrome rules
- No box shadows in dark mode (they look smudged on black).
- Light theme can use one subtle shadow on elevated surfaces, no more.
- No filled buttons except the primary CTA (white on black).
- Secondary actions: `.ghost` (transparent, dim text, hover lifts bg). Outline only
  when secondary action needs more presence than ghost (`.outline`).
- Keyboard hints use `<kbd>` styled monochrome, smaller than body text.

### Layout
- **Window:** 760×760 default (square), `decorations: false`. The app draws its
  own title bar; no native chrome. Min size 480×480.
- **Custom title bar** (`src/lib/components/TitleBar.svelte`) lives in
  `+layout.svelte`, so every route sits below it. Drag region uses both
  `data-tauri-drag-region` (Tauri 2 native) and `-webkit-app-region: drag` (Linux
  WebKit fallback). Controls (min / max / close) are right-aligned, 36×32, no
  border-radius. Close button hovers `#e81123`.
- **Review screen** is the most opinionated: header (small, hairline border-bottom),
  card-area (centered, max-width 680, generous padding), footer (rating row or hint).
- **Home:** settings icon top-right, content centered. Title bar shows "lapse" —
  don't duplicate the brand in the page header.
- **Settings:** max-width 640, centered. List items separated by `--border`, not cards.

### Keyboard map (review screen)
- `space` / `Enter` — flip on front; rates **Easy** on back (default action).
- `f` — rates **Hard** on back.
- `r` — replay audio.
- `t` — toggle session-stats overlay (due / new / learning / total).
- `Ctrl+Z` / `Cmd+Z` — undo last rating (restores card state + drops the most
  recent review_log entry). Single-level undo only — only the immediately
  preceding rating is reversible.
- `Esc` — back to home.

### Card ordering
New cards are pulled in **random order** (`ORDER BY ... random()` in the
new-card branch of next_due_card). This stops bidirectional vocab pairs
(EN→AR card N and AR→EN card N+1) from appearing back-to-back. Learning and
review cards still order by `due` ascending.

## Code conventions

### Rust (`src-tauri/src/`)
- One responsibility per module: `db.rs` (rusqlite), `scheduler.rs` (FSRS wrapper),
  `commands.rs` (Tauri command handlers), `lib.rs` (wires it together).
- `AppState { conn: Mutex<Option<(Connection, PathBuf)>> }` — one deck at a time;
  `None` means no deck open.
- Commands return `Result<T, String>` to the frontend. Internally use `anyhow::Result`
  and map at the boundary.
- Time: always `chrono::DateTime<Utc>`. Store as `i64` unix ms in SQLite.
- Pinned deps: `rusqlite = "0.37"` (NOT 0.38+ — uses unstable `cfg_select`),
  `rs-fsrs = "1.2"`, `tauri = "2"`.

### Svelte (`src/`)
- Svelte 5 runes (`$state`, `$derived`, `$props`). No legacy `let x = y` reactivity.
- One route per screen (`+page.svelte`). Shared state in `$lib/store.ts`.
- IPC wrappers in `$lib/api.ts` — typed, all `invoke()` calls go through `api.*`.
- Each `+page.svelte` carries its own `<style>` block. No global CSS beyond `app.css`.
- Keyboard handlers attach via `<svelte:window onkeydown={...}>`.
- For audio: fetch BLOB via `api.cardAudio(id)`, create blob URL with
  `audioBlobToUrl()`, **revoke previous URL when card changes** (memory leak otherwise).
- HTML in card content is rendered as **text**, not interpolated. If we ever need
  rich content, add it explicitly behind a flag — don't `{@html}` user-supplied input.

### Tauri capabilities
- Custom window controls need explicit permissions in
  `src-tauri/capabilities/default.json`:
  `core:window:allow-minimize`, `allow-maximize`, `allow-unmaximize`,
  `allow-is-maximized`, `allow-close`, `allow-start-dragging`.
- Don't grant capabilities you don't use — keep the list minimal.

### Deck builders & audio

All deck-building scripts live under `.claude/skills/deck-builder/` as
a Claude Code skill. See that folder's `SKILL.md` for full usage. Three
scripts ship:

- `make_sample_deck.py` — 5-card smoke-test deck.
- `import_lebanese.py` — converts the anki-lebanese Python data files
  into the lapse v1 schema (~1823 cards, bidirectional vocab +
  conjugations + grammar).
- `fetch_edge_tts.py` — fills the `audio` BLOB column via Microsoft
  `edge-tts`. Default `ar-LB-LaylaNeural`. Free, no signup. Resumable.

Audio policy: the app plays whatever's in `cards.audio` verbatim — no
live TTS. Scripts pre-bake audio. They locate the Arabic string by
scanning front/back lines for the U+0600..U+06FF Unicode block, then
attach the synthesized BLOB to every card sharing that string
(bidirectional vocab → 2 cards updated per synthesis).

**Autoplay side detection (anti-spoiler).** The review screen does NOT
autoplay audio on the front side of a card where the audio's language
appears only on the back — that would spoil a recall prompt. The proxy
is RTL detection: if the front contains RTL characters, audio plays on
appearance; if only the back does, audio waits until flip. Manual replay
(`r` or the ▶ button) works on either side. See `audioSide()` in
`src/routes/review/+page.svelte`.

To swap voices on an existing deck:
```sql
UPDATE cards SET audio = NULL, audio_mime = NULL;
```
then re-run `fetch_edge_tts.py` with the new voice argument.

### Undo

`commands::undo_rating(card: Card)` takes a full Card snapshot from the
frontend (the state as it was BEFORE the last rate_card call) and:
1. Restores all FSRS columns on that row.
2. Deletes the most recent `review_log` entry for that card.

The frontend holds the snapshot in a single `undoable` slot — only the most
recent rating is undoable. Don't add a multi-level stack; the FSRS algorithm
isn't reversible enough for it to be useful, and the UX is clearer with one
level.

### Schema changes
- Bump `meta.schema_version`. The app should refuse to open decks with a higher
  schema version than it knows. (Not yet implemented — add when v2 lands.)
- Update both `schema.sql` AND `scripts/make_sample_deck.py` — they must stay in
  sync because the sample script is the second source of truth for the schema.

## Out-of-scope reminders (don't drift)

- Don't add `.apkg` import. It was explicitly rejected.
- Don't add live TTS (edge-tts / web-speech / piper). Audio is BLOB-only.
- Don't add card editing in the app. Decks are external artifacts.
- Don't add note types or templates. Schema is fixed.
- Don't add a deck tree / multi-deck view. One deck open at a time.
- Don't add sync. File-based, period.
- Don't introduce CSS frameworks (Tailwind, UnoCSS, etc.). Vanilla CSS + tokens.

If the user asks for any of these, surface the original decision before implementing —
they may have changed their mind, but make the trade-off explicit.

## Useful commands

```bash
# Dev (needs librsvg2-devel on Fedora)
npm install
npm run tauri dev

# Type-check
cd src-tauri && cargo check
npm run check

# Sample deck for testing
python3 scripts/make_sample_deck.py sample.db
```
