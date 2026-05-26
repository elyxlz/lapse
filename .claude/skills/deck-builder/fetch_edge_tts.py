# /// script
# requires-python = ">=3.10"
# dependencies = ["edge-tts>=7.0"]
# ///
"""Populate the `audio` BLOB column of a lapse deck using Microsoft's
free `edge-tts` neural voices.

Default voice is ar-LB-LaylaNeural (Lebanese female). Override via the
second positional arg or the LAPSE_TTS_VOICE env var.

Run:
  uv run scripts/fetch_edge_tts.py lebanese.db                # default voice
  uv run scripts/fetch_edge_tts.py lebanese.db ar-LB-RamiNeural

Skips cards that already have audio, so reruns resume where a previous
run was interrupted.
"""
from __future__ import annotations

import asyncio
import os
import re
import sqlite3
import sys
import time
from pathlib import Path

import edge_tts

# Covers Arabic, Arabic Supplement, Arabic Extended-A, and both
# Arabic Presentation Forms blocks. Adapt for other languages.
ARABIC_RE = re.compile(
    r"[؀-ۿݐ-ݿࢠ-ࣿﭐ-﷿ﹰ-﻿]"
)
DEFAULT_VOICE = "ar-LB-LaylaNeural"

# edge-tts can transiently fail; retry a handful of times before giving up
# on a single string. Failures don't block the deck — the card just keeps
# its NULL audio and a future run can pick it back up.
MAX_RETRIES = 3
RETRY_DELAY = 1.5


def first_arabic_line(text: str | None) -> str | None:
    """First trimmed line in `text` that contains target-script chars."""
    if not text:
        return None
    for line in text.split("\n"):
        line = line.strip()
        if line and ARABIC_RE.search(line):
            return line
    return None


def extract_arabic_side(
    front: str | None, back: str | None
) -> tuple[str | None, str | None]:
    """Return (text, side) where side is 'front' | 'back' | 'both' | None.

    Used both for picking what to synthesize and for tagging the card with
    which side the audio belongs to (so the app knows whether autoplay on
    appearance would spoil the prompt).
    """
    f = first_arabic_line(front)
    b = first_arabic_line(back)
    if f and b:
        # Same word on both sides (rare) — pick the front so the audio
        # text is stable, but mark both so playback fires twice.
        return f, "both"
    if f:
        return f, "front"
    if b:
        return b, "back"
    return None, None


def extract_arabic(*fields: str) -> str | None:
    """Back-compat shim."""
    for text in fields:
        line = first_arabic_line(text)
        if line:
            return line
    return None


async def synth(voice: str, text: str) -> bytes:
    communicate = edge_tts.Communicate(text, voice)
    buf = bytearray()
    async for chunk in communicate.stream():
        if chunk["type"] == "audio":
            buf.extend(chunk["data"])
    if not buf:
        raise RuntimeError(f"empty audio response for {text!r}")
    return bytes(buf)


async def main() -> int:
    if len(sys.argv) < 2:
        print("usage: fetch_edge_tts.py <deck.db> [voice]", file=sys.stderr)
        return 2
    db_path = Path(sys.argv[1])
    voice = (
        sys.argv[2]
        if len(sys.argv) > 2
        else os.environ.get("LAPSE_TTS_VOICE", DEFAULT_VOICE)
    )
    if not db_path.exists():
        print(f"no such file: {db_path}", file=sys.stderr)
        return 1

    # `timeout` lets the script wait if the lapse app currently has the
    # deck open in WAL mode rather than crashing on SQLITE_BUSY.
    conn = sqlite3.connect(db_path, timeout=30.0)
    conn.row_factory = sqlite3.Row

    # Backfill audio_side for cards that already have audio from a previous
    # run (pre-v2 decks). Cheap one-off pass before the main synthesis loop.
    backfill = conn.execute(
        "SELECT id, front, back FROM cards "
        "WHERE audio IS NOT NULL AND audio_side IS NULL"
    ).fetchall()
    backfilled = 0
    for row in backfill:
        _, side = extract_arabic_side(row["front"], row["back"])
        if side:
            conn.execute(
                "UPDATE cards SET audio_side = ? WHERE id = ?",
                (side, row["id"]),
            )
            backfilled += 1
    if backfilled:
        conn.commit()
        print(f"backfilled audio_side on {backfilled} pre-existing cards")

    rows = conn.execute(
        "SELECT id, front, back FROM cards WHERE audio IS NULL"
    ).fetchall()

    # Group cards by the target string they share. Each group also tracks
    # per-card (id, side) so the same audio attached to two bidirectional
    # cards still gets the correct side flag for each.
    target_to_cards: dict[str, list[tuple[int, str]]] = {}
    skipped = 0
    for row in rows:
        text, side = extract_arabic_side(row["front"], row["back"])
        if text is None or side is None:
            skipped += 1
            continue
        target_to_cards.setdefault(text, []).append((row["id"], side))

    total_strings = len(target_to_cards)
    total_cards = sum(len(ids) for ids in target_to_cards.values())
    print(
        f"voice={voice}  unique_strings={total_strings}  cards_to_update={total_cards}  no_target={skipped}"
    )
    if total_strings == 0:
        print("nothing to do.")
        conn.close()
        return 0

    start = time.monotonic()
    done = 0
    failed: list[tuple[str, str]] = []

    for i, (text, card_entries) in enumerate(target_to_cards.items(), 1):
        last_err: str | None = None
        audio: bytes | None = None
        for attempt in range(1, MAX_RETRIES + 1):
            try:
                audio = await synth(voice, text)
                break
            except Exception as e:  # noqa: BLE001
                last_err = str(e)
                if attempt < MAX_RETRIES:
                    await asyncio.sleep(RETRY_DELAY * attempt)
        if audio is None:
            failed.append((text, last_err or "unknown"))
            print(f"[{i}/{total_strings}] FAIL  {text}  ({last_err})", flush=True)
            continue
        conn.executemany(
            "UPDATE cards SET audio = ?, audio_mime = 'audio/mpeg', "
            "audio_side = ? WHERE id = ?",
            [(audio, side, cid) for cid, side in card_entries],
        )
        conn.commit()
        done += 1
        elapsed = time.monotonic() - start
        rate = done / elapsed if elapsed > 0 else 0
        eta = (total_strings - i) / rate if rate > 0 else 0
        print(
            f"[{i}/{total_strings}] ok   {text[:30]:30s}  {len(audio):>6}B  ({len(card_entries)} cards)  eta {eta:5.0f}s",
            flush=True,
        )

    conn.close()
    print(f"done. {done} synthesized, {len(failed)} failed.")
    if failed:
        print("Failed strings (rerun the script to retry just these):")
        for text, err in failed[:20]:
            print(f"  {text}  --  {err}")
    return 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
