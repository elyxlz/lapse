#!/usr/bin/env python3
"""Fill cards.example with hand-authored Lebanese usage sentences.

Match strategy:
    1. Find the Arabic line in each card (front or back).
    2. Look that Arabic up in EXAMPLES (exact match).
    3. Write the formatted example into cards.example.

EXAMPLES is a dict keyed by the Arabic head-form, valued by a three-line
string: <arabic sentence>\\n<lebanese chat-alphabet>\\n<english>. Add more
entries over time — the script is idempotent (only fills NULL cells).

Run:
  python3 .claude/skills/deck-builder/fill_examples.py deck.lapse
"""
from __future__ import annotations

import re
import sqlite3
import sys
from pathlib import Path

ARABIC_RE = re.compile(r"[؀-ۿݐ-ݿࢠ-ࣿﭐ-﷿ﹰ-﻿]+")


def first_arabic_token(text: str | None) -> str | None:
    """Return the first standalone Arabic line in the text, stripped."""
    if not text:
        return None
    for line in text.split("\n"):
        line = line.strip()
        if line and ARABIC_RE.search(line):
            # Strip a trailing punctuation char if any
            return line.rstrip("؟?!.,،")
    return None


# ----------------------------------------------------------------------
# Hand-authored examples. Lebanese chat-alphabet conventions:
#   2 = ق (glottal stop, Lebanese drops the qaaf)
#   7 = ح
#   9 = ص
#   6 = ط
#   3 = ع
#   gh = غ, kh = خ, sh = ش, dh = ذ, th = ث, dh = ض / ظ (context)
# Tense markers: b- = imperfect indicative ("I do"), 3am- = continuous,
# ra7- = future, kaan = past auxiliary.
# Format per entry: "<arabic sentence>\n<translit>\n<english>"
# ----------------------------------------------------------------------
EXAMPLES: dict[str, str] = {
    # --- Greetings & politeness ---
    "مرحبا": "مرحبا، كيفك اليوم؟\nMar7aba, kifak el-yom?\nHello, how are you today?",
    "أهلا": "أهلا وسهلا، تفضل\nAhla w sahla, tfaDDal\nWelcome, come in",
    "أهلين": "أهلين! كيف الأحوال؟\nAhlayn! Kif el-a7waal?\nHi! How's everything?",
    "صباح الخير": "صباح الخير، كيف نمت؟\nSabaa7 el-khayr, kif nemet?\nGood morning, how did you sleep?",
    "مساء الخير": "مساء الخير، شو الأخبار؟\nMasa el-khayr, shu el-akhbaar?\nGood evening, what's new?",
    "تصبح على خير": "تصبح على خير وأحلام حلوة\nTousba7 3ala khayr w a7laam 7elwe\nGood night and sweet dreams",
    "مع السلامة": "مع السلامة، بشوفك بكرا\nMa3 es-salameh, bshufak bukra\nGoodbye, see you tomorrow",
    "وداعا": "وداعا يا صديقي\nWada3an ya 9adii2i\nFarewell, my friend",
    "شكراً": "شكراً كتير لمساعدتك\nShukran kteer la-musa3adtak\nThanks a lot for your help",
    "عفواً": "عفواً، ما في مشكلة\n3afwan, ma fi mishkleh\nYou're welcome, no problem",
    "لو سمحت": "لو سمحت، ممكن كاسة مي؟\nLaw sama7t, mumken kaaset mayy?\nPlease, may I have a glass of water?",
    "من فضلك": "من فضلك، إنطرني شوي\nMin faDlak, inTorni shway\nPlease, wait for me a bit",
    "آسف": "آسف، ما كان قصدي\nAasef, ma kaan a9di\nSorry, I didn't mean to",
    "معلش": "معلش، بتصير\nMa3lesh, btsiir\nNo worries, it happens",
    # --- How are you / responses ---
    "كيفك": "كيفك اليوم؟ منيح؟\nKifak el-yom? Mneeh?\nHow are you today? Good?",
    "منيح": "أنا منيح، الحمدلله\nAna mneeh, el-7amdullah\nI'm good, thank god",
    "الحمدلله": "الحمدلله، كل شي تمام\nEl-7amdullah, kell shi tamam\nThank god, everything's fine",
    "تمام": "تمام، خلينا نروح\nTamam, khalleena nrou7\nAlright, let's go",
    "بخير": "أنا بخير، شكراً\nAna bi-khayr, shukran\nI'm fine, thanks",
    # --- Yes / no / basics ---
    "نعم": "نعم، بدي ساعدك\nNa3am, baddi saa3dak\nYes, I want to help you",
    "لا": "لا، ما بقدر هلق\nLa, ma b2der halla2\nNo, I can't right now",
    "آه": "آه، فهمت عليك\nAah, fhemet 3alayk\nYeah, I understood you",
    # --- Question words ---
    "شو": "شو عم تعمل؟\nShu 3am ta3mel?\nWhat are you doing?",
    "وين": "وين كنت مبارح؟\nWen kenet mbaare7?\nWhere were you yesterday?",
    "لوين": "لوين رايح؟\nLawen raaye7?\nWhere are you going?",
    "كم": "كم ساعة بدك؟\nKam saa3a baddak?\nHow many hours do you need?",
    "بكم": "بكم هاي؟\nBkam hayy?\nHow much is this?",
    "ليش": "ليش متأخر؟\nLeesh mit2akhkhar?\nWhy are you late?",
    "كيف": "كيف بدنا نروح؟\nKif badna nrou7?\nHow are we going to go?",
    "إيمتى": "إيمتى منلتقي؟\n2eemta mnelta2i?\n2eemta = Lebanese for \"when\". When do we meet?",
    "مين": "مين بالباب؟\nMeen bel-baab?\nWho's at the door?",
    # --- Self-introduction ---
    "اسمي": "اسمي إيليو، تشرفت\nIsmi Elio, tsharrafet\nMy name is Elio, pleased to meet you",
    "من": "أنا من لبنان\nAna min Lebnan\nI'm from Lebanon",
    # --- Want / can / must ---
    "بدي": "بدي قهوة من فضلك\nBaddi ahweh min faDlak\nI want coffee please",
    "ممكن": "ممكن أسألك شي؟\nMumken as2alak shi?\nMay I ask you something?",
    "لازم": "لازم أروح هلق\nLazem rou7 halla2\nI have to go now",
    "بقدر": "بقدر ساعدك بكرا\nB2der saa3dak bukra\nI can help you tomorrow",
    "بحب": "بحب القهوة كتير\nB7eb el-ahweh kteer\nI love coffee a lot",
    "بحبك": "بحبك يا حياتي\nB7ebbak ya 7ayaati\nI love you, my life",
    # --- Common verbs ---
    "بفهم": "بفهم عربي شوي\nBefhem 3arabi shway\nI understand Arabic a little",
    "بتحكي": "بتحكي إنكليزي؟\nBte7ki Ingleezi?\nDo you speak English?",
    "بشتغل": "بشتغل في بيروت\nBeshteghel fi Bayrut\nI work in Beirut",
    "بدرس": "بدرس عربي كل يوم\nBedres 3arabi kell yom\nI study Arabic every day",
    "بأكل": "بأكل تبولة\nBaakol tabbouleh\nI eat tabbouleh",
    "بشرب": "بشرب مي كتير\nBeshrab mayy kteer\nI drink a lot of water",
    "بنام": "بنام بكير\nBnaam bekkeer\nI sleep early",
    "بروح": "بروح ع الشغل\nBrou7 3al shighl\nI go to work",
    "بيجي": "بيجي بكرا\nByeji bukra\nHe's coming tomorrow",
    # --- Speed / quantity ---
    "كتير": "في ناس كتير هون\nFi naas kteer hon\nThere are a lot of people here",
    "شوي": "إنطرني شوي\nInTorni shway\nWait for me a bit",
    "بسيط": "هاي مهمة بسيطة\nHayy mehimme baseeTa\nThis is a simple task",
    "كبير": "هاد بيت كبير\nHaad bayt kbiir\nThis is a big house",
    "صغير": "أخي الصغير\nAkhi e9-9ghiir\nMy little brother",
    "حلو": "هاي قهوة حلوة\nHayy ahweh 7elwe\nThis is nice coffee",
    "زاكي": "الأكل كان زاكي\nEl-akel kaan zaaki\nThe food was delicious",
    # --- Time ---
    "اليوم": "اليوم عندي اجتماع\nEl-yom 3indi ijtimaa3\nI have a meeting today",
    "بكرا": "بكرا منشوفك\nBukra mnshufak\nWe'll see you tomorrow",
    "مبارح": "مبارح كنت تعبان\nMbaare7 kenet ta3baan\nYesterday I was tired",
    "هلق": "هلق بدي روح\nHalla2 baddi rou7\nI want to go now",
    "بعدين": "نحكي بعدين\nNe7ki ba3den\nWe'll talk later",
    "دايماً": "دايماً متأخر\nDayman mit2akhkhar\nHe's always late",
    "أبداً": "ما بحب القهوة أبداً\nMa b7eb el-ahweh abadan\nI never like coffee at all",
    # --- Common nouns ---
    "مي": "بدي كاسة مي\nBaddi kaaset mayy\nI want a glass of water",
    "أكل": "الأكل جاهز\nEl-akel jahez\nThe food is ready",
    "قهوة": "بشرب قهوة الصبح\nBeshrab ahweh e9-9eb7\nI drink coffee in the morning",
    "شاي": "بدك شاي ولا قهوة؟\nBaddak shay walla ahweh?\nDo you want tea or coffee?",
    "بيت": "بيتي قريب\nBayti 2arrayyeb\nMy house is nearby",
    "شغل": "الشغل كتير اليوم\nEsh-shighl kteer el-yom\nWork is a lot today",
    "سيارة": "سيارتي معطلة\nSayyaarti m3aTTleh\nMy car is broken down",
    "مدرسة": "الولاد في المدرسة\nEl-wlaad fil madrasi\nThe kids are at school",
    "شمس": "الشمس قوية اليوم\nEsh-shams 2awiyyeh el-yom\nThe sun is strong today",
    "بحر": "البحر قريب من البيت\nEl-ba7r 2arrayyeb min el-bayt\nThe sea is near the house",
    "ناس": "في ناس كتير بالسوق\nFi naas kteer bes-soo2\nThere are lots of people in the market",
    "ولد": "هاد ولد ذكي\nHaad walad zaki\nThis is a smart boy",
    "بنت": "البنت عم تدرس\nEl-bint 3am tedres\nThe girl is studying",
    "رجال": "هاد رجال طيب\nHaad rijjaal Tayyeb\nThis is a kind man",
    "مرة": "المرة عم تطبخ\nEl-mara 3am toTbokh\nThe woman is cooking",
    "كتاب": "هاد الكتاب حلو\nHaad el-kteb 7elu\nThis book is good",
    "مفتاح": "وين المفتاح؟\nWen el-mifteh?\nWhere is the key?",
    "تلفون": "تلفوني معطل\nTelefoone m3aTTal\nMy phone is broken",
    "فلوس": "ما معي فلوس\nMa ma3i fluus\nI don't have money on me",
    "وقت": "ما عندي وقت\nMa 3indi wa2t\nI don't have time",
    # --- Family ---
    "أبي": "أبي بالبيت\nAbi bel-bayt\nMy father is at home",
    "أمي": "أمي عم تطبخ\nImmi 3am toTbokh\nMy mom is cooking",
    "أخ": "هاد أخي الكبير\nHaad akhi el-kbiir\nThis is my older brother",
    "أخت": "أختي عم تدرس\nEkhti 3am tedres\nMy sister is studying",
    "صديق": "صديقي إجا يزورني\n9adii2i ija yzuurni\nMy friend came to visit me",
    # --- Misc useful ---
    "يلا": "يلا منروح!\nYalla, mnrou7!\nLet's go!",
    "خلص": "خلص، ما في وقت\nKhalas, ma fi wa2t\nEnough, there's no time",
    "إيش": "إيش رأيك؟\nEesh ra2yak?\nWhat do you think?",
    "حياة": "هاي حياتي\nHayy 7ayaati\nThis is my life",
    "حب": "بحبك من كل قلبي\nB7ebbak min kell 2albi\nI love you with all my heart",
    "قلب": "قلبي معك\n2albi ma3ak\nMy heart is with you",
    "بطل": "بطل تعمل هيك\nBaTTel ta3mel hayk\nStop doing that",
    "مش": "أنا مش جوعان\nAna mish jou3aan\nI'm not hungry",
    "جوعان": "أنا جوعان كتير\nAna jou3aan kteer\nI'm very hungry",
    "عطشان": "أنا عطشان، بدي مي\nAna 3aTshaan, baddi mayy\nI'm thirsty, I want water",
    "تعبان": "اليوم تعبان كتير\nEl-yom ta3baan kteer\nI'm very tired today",
    "مبسوط": "أنا مبسوط هون\nAna mabsuuT hon\nI'm happy here",
    "حلوة": "هاي اللحظة حلوة\nHayy el-la7Za 7elwe\nThis moment is sweet",
}


def main() -> int:
    db_path = Path(sys.argv[1] if len(sys.argv) > 1 else "lebanese.lapse")
    if not db_path.exists():
        print(f"no such file: {db_path}", file=sys.stderr)
        return 1
    conn = sqlite3.connect(db_path, timeout=30.0)

    # Make the script self-sufficient: add the `example` column if the
    # deck was created with an older schema. The app's open() also
    # migrates, but the script may run on a fresh export before the app
    # has touched the file.
    cols = {row[1] for row in conn.execute("PRAGMA table_info(cards)").fetchall()}
    if "example" not in cols:
        conn.execute("ALTER TABLE cards ADD COLUMN example TEXT")
        conn.commit()

    rows = conn.execute(
        "SELECT id, front, back FROM cards WHERE example IS NULL"
    ).fetchall()

    def strip_harakat(s: str) -> str:
        return "".join(c for c in s if c not in "ًٌٍَُِّْـٰ")

    def lookup(ar: str) -> str | None:
        # 1. Exact match.
        if ar in EXAMPLES:
            return EXAMPLES[ar]
        # 2. With harakat / shadda stripped.
        s = strip_harakat(ar)
        if s != ar and s in EXAMPLES:
            return EXAMPLES[s]
        # 3. With the definite article ال- stripped.
        if s.startswith("ال") and s[2:] in EXAMPLES:
            return EXAMPLES[s[2:]]
        # 4. With ال- added (card had bare word, dict has definite form).
        if ("ال" + s) in EXAMPLES:
            return EXAMPLES["ال" + s]
        return None

    updated = 0
    no_match = 0
    for cid, front, back in rows:
        ar = first_arabic_token(front) or first_arabic_token(back)
        if ar is None:
            continue
        ex = lookup(ar)
        if ex is None:
            no_match += 1
            continue
        conn.execute("UPDATE cards SET example = ? WHERE id = ?", (ex, cid))
        updated += 1
    conn.commit()
    conn.close()
    print(f"filled {updated} cards. {no_match} had no matching example.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
