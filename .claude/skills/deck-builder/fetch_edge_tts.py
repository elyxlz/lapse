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

ARABIC_RE = re.compile(r"[؀-ۿ]")
DEFAULT_VOICE = "ar-LB-LaylaNeural"

# edge-tts can transiently fail; retry a handful of times before giving up
# on a single string. Failures don't block the deck — the card just keeps
# its NULL audio and a future run can pick it back up.
MAX_RETRIES = 3
RETRY_DELAY = 1.5


def extract_arabic(*fields: str) -> str | None:
    """Return the first line that contains Arabic characters, trimmed."""
    for text in fields:
        if not text:
            continue
        for line in text.split("\n"):
            line = line.strip()
            if line and ARABIC_RE.search(line):
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

    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row

    rows = conn.execute(
        "SELECT id, front, back FROM cards WHERE audio IS NULL"
    ).fetchall()

    # Group cards by the Arabic string they share (vocab is bidirectional,
    # so the same Arabic word appears on two cards — we synthesize it once
    # and attach to both).
    arabic_to_cards: dict[str, list[int]] = {}
    skipped = 0
    for row in rows:
        ar = extract_arabic(row["front"], row["back"])
        if ar is None:
            skipped += 1
            continue
        arabic_to_cards.setdefault(ar, []).append(row["id"])

    total_strings = len(arabic_to_cards)
    total_cards = sum(len(ids) for ids in arabic_to_cards.values())
    print(
        f"voice={voice}  unique_strings={total_strings}  cards_to_update={total_cards}  no_arabic={skipped}"
    )
    if total_strings == 0:
        print("nothing to do.")
        return 0

    start = time.monotonic()
    done = 0
    failed: list[tuple[str, str]] = []

    for i, (text, card_ids) in enumerate(arabic_to_cards.items(), 1):
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
            "UPDATE cards SET audio = ?, audio_mime = 'audio/mpeg' WHERE id = ?",
            [(audio, cid) for cid in card_ids],
        )
        conn.commit()
        done += 1
        elapsed = time.monotonic() - start
        rate = done / elapsed if elapsed > 0 else 0
        eta = (total_strings - i) / rate if rate > 0 else 0
        print(
            f"[{i}/{total_strings}] ok   {text[:30]:30s}  {len(audio):>6}B  ({len(card_ids)} cards)  eta {eta:5.0f}s",
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
