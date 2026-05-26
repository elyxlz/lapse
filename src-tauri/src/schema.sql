-- lapse deck schema v1
--
-- One .db file per deck. Opinionated minimal: front / back / audio / tags.
-- FSRS state lives in real columns, not JSON blobs.

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
    tags          TEXT    NOT NULL DEFAULT '',

    -- FSRS state
    state         INTEGER NOT NULL DEFAULT 0,  -- 0=new, 1=learning, 2=review, 3=relearning
    due           INTEGER NOT NULL DEFAULT 0,  -- unix ms; 0 = never reviewed (new)
    stability     REAL    NOT NULL DEFAULT 0,
    difficulty    REAL    NOT NULL DEFAULT 0,
    reps          INTEGER NOT NULL DEFAULT 0,
    lapses        INTEGER NOT NULL DEFAULT 0,
    last_review   INTEGER  -- unix ms; null if never reviewed
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

INSERT OR IGNORE INTO meta(key, value) VALUES ('schema_version', '1');
