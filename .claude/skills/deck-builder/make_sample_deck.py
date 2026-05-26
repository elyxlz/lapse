#!/usr/bin/env python3
"""Generate a tiny sample.lapse deck conforming to the lapse v3 schema.

Run: python3 .claude/skills/deck-builder/make_sample_deck.py [output.lapse]

The output file is a self-contained lapse deck you can open in the app.
"""
from __future__ import annotations

import sqlite3
import sys
from pathlib import Path

SCHEMA = """
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
    state         INTEGER NOT NULL DEFAULT 0,
    due           INTEGER NOT NULL DEFAULT 0,
    stability     REAL    NOT NULL DEFAULT 0,
    difficulty    REAL    NOT NULL DEFAULT 0,
    reps          INTEGER NOT NULL DEFAULT 0,
    lapses        INTEGER NOT NULL DEFAULT 0,
    last_review   INTEGER,
    learn_step    INTEGER                        -- learning ladder index; NULL once graduated
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
"""

CARDS = [
    ("marhaba", "hello (general)", "greetings"),
    ("شكراً", "thank you", "greetings politeness"),
    ("كيفك؟", "how are you?", "greetings"),
    ("yalla", "let's go / come on", "interjection"),
    ("habibi", "my dear (m.)", "endearment"),
]


def main() -> int:
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "sample.lapse")
    if out.exists():
        out.unlink()

    conn = sqlite3.connect(out)
    conn.executescript(SCHEMA)
    conn.execute(
        "INSERT OR REPLACE INTO meta(key, value) VALUES ('schema_version', '3')"
    )
    conn.execute(
        "INSERT OR REPLACE INTO meta(key, value) VALUES ('name', 'sample')"
    )
    conn.executemany(
        "INSERT INTO cards(front, back, tags) VALUES (?, ?, ?)",
        CARDS,
    )
    conn.commit()
    conn.close()
    print(f"wrote {out} ({len(CARDS)} cards)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
