#!/usr/bin/env python3
"""Convert the anki-lebanese Python data into a lapse v1 deck.

Reads:  ~/Repos/anki-lebanese/data_{vocab,conjugations,grammar}.py
Writes: lebanese.db (lapse schema)

Vocab cards are bidirectional (EN -> AR and AR -> EN, two cards per entry).
Conjugation cards: one card per verb-tense table (English topic -> formatted
plain-text paradigm).  Grammar cards: front -> stripped-HTML back.

Run from anywhere:
  python3 scripts/import_lebanese.py [output.db]
"""
from __future__ import annotations

import html
import re
import sqlite3
import sys
from pathlib import Path

# Make the anki-lebanese package importable
ANKI_LEBANESE = Path.home() / "Repos" / "anki-lebanese"
sys.path.insert(0, str(ANKI_LEBANESE))

# These imports come from anki-lebanese
from data_vocab import RAW_VOCAB  # type: ignore
from data_conjugations import CONJUGATIONS, PRONOUNS, TENSE_LABELS  # type: ignore
from data_grammar import GRAMMAR_CARDS  # type: ignore


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
    last_review   INTEGER
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
"""


# --- Lebanese pronunciation normalization (mirrors anki-lebanese/build_deck.py) ---
def normalize_lebanese(translit: str) -> str:
    s = translit
    s = s.replace("qu", "2u").replace("qa", "2a").replace("qi", "2i").replace("qo", "2o")
    s = s.replace("Qu", "2u").replace("Qa", "2a").replace("Qi", "2i").replace("Qo", "2o")
    s = s.replace("q'", "2'").replace("Q'", "2'")
    s = re.sub(r"\bq", "2", s)
    s = re.sub(r"q\b", "2", s)
    s = s.replace("q", "2").replace("Q", "2")
    s = re.sub(r"H", "7", s)
    s = re.sub(r"S", "9", s)
    s = re.sub(r"T", "6", s)
    # Restore DH (ظ) which the H->7 substitution above would have mangled.
    s = s.replace("D7", "DH")
    return s


# --- HTML stripping for grammar cards ---
_TAG_RE = re.compile(r"<[^>]+>")
_WS_RE = re.compile(r"[ \t]+")


def html_table_to_text(s: str) -> str:
    """Crude HTML -> plain text: <tr> -> newline, <th>/<td> -> tab-separated."""
    s = re.sub(r"</tr>", "\n", s, flags=re.I)
    s = re.sub(r"<(th|td)[^>]*>", "", s, flags=re.I)
    s = re.sub(r"</(th|td)>", "\t", s, flags=re.I)
    s = re.sub(r"<p[^>]*>", "\n", s, flags=re.I)
    s = re.sub(r"</p>", "\n", s, flags=re.I)
    s = re.sub(r"<br\s*/?>", "\n", s, flags=re.I)
    s = _TAG_RE.sub("", s)
    s = html.unescape(s)
    lines = [ln.rstrip(" \t") for ln in s.splitlines()]
    lines = [ln for ln in lines if ln.strip()]
    return "\n".join(lines)


def build_vocab_cards(rows: list[tuple]) -> list[tuple[str, str, str]]:
    """Yield (front, back, tags) for each direction."""
    out: list[tuple[str, str, str]] = []
    for ar, translit, en, category in rows:
        lev = normalize_lebanese(translit)
        en_clean = en.strip()
        ar_text = ar.strip()
        # EN -> AR
        back_en2ar = f"{ar_text}\n{lev}"
        out.append((en_clean, back_en2ar, f"vocab {category} en2ar"))
        # AR -> EN
        front_ar2en = f"{ar_text}\n{lev}"
        out.append((front_ar2en, en_clean, f"vocab {category} ar2en"))
    return out


def format_conjugation_table(forms: list[tuple[str, str]]) -> str:
    """forms is 8 (arabic, translit) entries aligned with PRONOUNS."""
    if len(forms) != len(PRONOUNS):
        raise ValueError(
            f"expected {len(PRONOUNS)} conjugation forms, got {len(forms)}: {forms!r}"
        )
    rows = []
    for (pronoun, gloss), (ar, tr) in zip(PRONOUNS, forms):
        lev = normalize_lebanese(tr)
        rows.append(f"{pronoun} ({gloss})\t{ar}\t{lev}")
    return "\n".join(rows)


def build_conjugation_cards() -> list[tuple[str, str, str]]:
    out: list[tuple[str, str, str]] = []
    for verb_en, verb_data in CONJUGATIONS.items():
        for tense_code, forms in verb_data.items():
            if tense_code in ("infinitive_ar", "infinitive_tr", "source", "note"):
                continue
            # Skip if forms isn't a list of (ar, tr) tuples (defensive)
            if not (isinstance(forms, list) and forms and isinstance(forms[0], tuple)):
                continue
            label = TENSE_LABELS.get(tense_code, tense_code)
            front = f"{verb_en} — {label}"
            back = format_conjugation_table(forms)
            tags = f"conjugation {tense_code} {verb_en.replace(' ', '_')}"
            out.append((front, back, tags))
    return out


def build_grammar_cards() -> list[tuple[str, str, str]]:
    out: list[tuple[str, str, str]] = []
    for front, back_html, tags in GRAMMAR_CARDS:
        back_text = html_table_to_text(back_html)
        # tags list -> space-separated
        if isinstance(tags, (list, tuple)):
            tag_str = " ".join(tags)
        else:
            tag_str = str(tags)
        out.append((front.strip(), back_text, f"grammar {tag_str}"))
    return out


def main() -> int:
    out_path = Path(sys.argv[1] if len(sys.argv) > 1 else "lebanese.db")
    if out_path.exists():
        # Warn loudly if we're about to clobber a deck that already has
        # audio — re-running this script deletes the file outright and
        # any TTS work has to be redone from scratch.
        try:
            existing = sqlite3.connect(out_path)
            (audio_count,) = existing.execute(
                "SELECT COUNT(*) FROM cards WHERE audio IS NOT NULL"
            ).fetchone()
            existing.close()
            if audio_count > 0:
                print(
                    f"warning: {out_path} has {audio_count} cards with audio "
                    "that will be deleted. Re-run fetch_edge_tts.py after "
                    "this to repopulate.",
                    file=sys.stderr,
                )
        except sqlite3.Error:
            # Not a valid SQLite file or table missing — nothing to warn about.
            pass
        out_path.unlink()

    conn = sqlite3.connect(out_path)
    conn.executescript(SCHEMA)

    all_cards: list[tuple[str, str, str]] = []
    all_cards += build_vocab_cards(RAW_VOCAB)
    all_cards += build_conjugation_cards()
    all_cards += build_grammar_cards()

    conn.executemany(
        "INSERT INTO cards(front, back, tags) VALUES (?, ?, ?)",
        all_cards,
    )
    conn.execute("INSERT OR REPLACE INTO meta(key, value) VALUES ('schema_version', '2')")
    conn.execute(
        "INSERT OR REPLACE INTO meta(key, value) VALUES ('name', 'Lebanese Arabic')"
    )
    conn.commit()
    conn.close()
    print(f"wrote {out_path} ({len(all_cards)} cards)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
