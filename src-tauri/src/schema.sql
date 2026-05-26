-- lapse deck schema v3
--
-- One .db file per deck. Opinionated minimal: front / back / audio / tags.
-- FSRS state lives in real columns, not JSON blobs.
--
-- v2: added cards.audio_side (anti-spoiler autoplay timing).
-- v3: added cards.learn_step for Anki-style stepped learning. The
--     scheduler walks new/learning cards through [1m, 10m] before
--     graduating them to FSRS-managed Review state. learn_step is the
--     index into LEARN_STEPS_MS; NULL once the card has graduated.

CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS cards (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    front         TEXT    NOT NULL,
    back          TEXT    NOT NULL,
    audio         BLOB,
    audio_mime    TEXT,
    audio_side    TEXT,                          -- 'front' | 'back' | 'both' | NULL
    tags          TEXT    NOT NULL DEFAULT '',

    -- FSRS / scheduler state
    state         INTEGER NOT NULL DEFAULT 0,
    due           INTEGER NOT NULL DEFAULT 0,
    stability     REAL    NOT NULL DEFAULT 0,
    difficulty    REAL    NOT NULL DEFAULT 0,
    reps          INTEGER NOT NULL DEFAULT 0,
    lapses        INTEGER NOT NULL DEFAULT 0,
    last_review   INTEGER,
    learn_step    INTEGER                        -- index into LEARN_STEPS_MS; NULL = graduated
);

CREATE INDEX IF NOT EXISTS idx_cards_due   ON cards(due);
CREATE INDEX IF NOT EXISTS idx_cards_state ON cards(state);

CREATE TABLE IF NOT EXISTS review_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    card_id         INTEGER NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    reviewed_at     INTEGER NOT NULL,
    rating          INTEGER NOT NULL,
    elapsed_days    REAL    NOT NULL,
    scheduled_days  REAL    NOT NULL,
    state_before    INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_review_log_card ON review_log(card_id);
CREATE INDEX IF NOT EXISTS idx_review_log_time ON review_log(reviewed_at);

INSERT OR IGNORE INTO meta(key, value) VALUES ('schema_version', '3');
