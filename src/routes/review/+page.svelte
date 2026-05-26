<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { goto } from "$app/navigation";
  import { api, audioBlobToUrl, type Card, type Rating } from "$lib/api";
  import { deck, stats } from "$lib/store";

  // Binary rating mode: Easy → FSRS Good(3), Hard → FSRS Again(1).
  const EASY: Rating = 3;
  const HARD: Rating = 1;

  let current = $state<Card | null>(null);
  let flipped = $state(false);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let audioUrl: string | null = null;
  let audioEl: HTMLAudioElement;
  let reviewed = $state(0);
  let flash = $state<{ label: string; tone: "easy" | "hard"; key: number } | null>(null);
  let undoable = $state<{ card: Card; wasFlipped: boolean } | null>(null);
  let showStats = $state(false);

  function isLikelyRtl(text: string): boolean {
    return /[֐-ࣿיִ-ﻼ]/.test(text);
  }

  async function loadAudio(id: number) {
    revokeAudio();
    try {
      const blob = await api.cardAudio(id);
      if (!blob) return;
      audioUrl = audioBlobToUrl(blob);
      if (audioEl) {
        audioEl.src = audioUrl;
        audioEl.play().catch(() => {});
      }
    } catch {
      // audio failure shouldn't block review
    }
  }

  function revokeAudio() {
    if (audioUrl) {
      URL.revokeObjectURL(audioUrl);
      audioUrl = null;
    }
  }

  function setCard(c: Card | null, opts: { flipped?: boolean } = {}) {
    current = c;
    flipped = opts.flipped ?? false;
    if (c?.has_audio) {
      loadAudio(c.id);
    } else {
      revokeAudio();
    }
  }

  function replayAudio() {
    if (audioEl && audioUrl) {
      audioEl.currentTime = 0;
      audioEl.play().catch(() => {});
    }
  }

  function flip() {
    if (!flipped && current) flipped = true;
  }

  async function rate(r: Rating) {
    if (!flipped || !current || loading) return;
    const cardSnapshot = current;
    loading = true;
    flash = {
      label: r === EASY ? "Easy" : "Hard",
      tone: r === EASY ? "easy" : "hard",
      key: Date.now(),
    };
    try {
      const next = await api.rateCard(cardSnapshot.id, r);
      undoable = { card: cardSnapshot, wasFlipped: true };
      reviewed += 1;
      setCard(next);
      if ($deck) stats.set(await api.deckStats());
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function undo() {
    if (!undoable || loading) return;
    loading = true;
    try {
      await api.undoRating(undoable.card);
      setCard(undoable.card, { flipped: undoable.wasFlipped });
      reviewed = Math.max(0, reviewed - 1);
      undoable = null;
      if ($deck) stats.set(await api.deckStats());
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function done() { goto("/"); }

  onMount(async () => {
    if (!$deck) { goto("/"); return; }
    try {
      const c = await api.nextCard();
      setCard(c);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  });

  onDestroy(() => { revokeAudio(); });

  function onKeydown(e: KeyboardEvent) {
    if (e.repeat) return;
    if ((e.ctrlKey || e.metaKey) && e.key === "z") {
      e.preventDefault();
      undo();
      return;
    }
    if (e.key === "Escape") { done(); return; }
    if (e.key === "t" || e.key === "T") {
      e.preventDefault();
      showStats = !showStats;
      return;
    }
    if (e.code === "Space" || e.key === " " || e.key === "Enter") {
      e.preventDefault();
      if (!flipped) flip();
      else rate(EASY);
      return;
    }
    if (!flipped) return;
    if (e.key === "f" || e.key === "F") rate(HARD);
    else if (e.key === "r") replayAudio();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<audio bind:this={audioEl} preload="auto"></audio>

<main class="review">
  <header>
    <button class="ghost back" onclick={done} aria-label="Back to home">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M19 12H5M12 19l-7-7 7-7"></path>
      </svg>
    </button>
    <div class="progress">
      <span class="dim">{$deck?.name ?? ""}</span>
      <span class="faint">·</span>
      <span class="mono">{reviewed} done · {$stats?.due ?? 0} left</span>
    </div>
    <button class="ghost stats-toggle" onclick={() => (showStats = !showStats)} aria-label="Toggle stats" title="Stats (t)">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="18" y1="20" x2="18" y2="10"/><line x1="12" y1="20" x2="12" y2="4"/><line x1="6" y1="20" x2="6" y2="14"/>
      </svg>
    </button>
  </header>

  <section class="card-area">
    {#if current}
      {@const rtlFront = isLikelyRtl(current.front)}
      {@const rtlBack = isLikelyRtl(current.back)}
      <div class="card">
        <div class="face front" class:rtl={rtlFront}>{current.front}</div>
        {#if current.has_audio}
          <button class="audio-btn ghost" onclick={replayAudio} aria-label="Replay audio">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
            <span class="dim faint-key"><kbd>r</kbd></span>
          </button>
        {/if}

        {#if flipped}
          <hr class="divider"/>
          <div class="face back" class:rtl={rtlBack}>{current.back}</div>
        {/if}
      </div>
    {:else if loading}
      <div class="empty dim">Loading…</div>
    {:else}
      <div class="empty">
        <div class="empty-title">All done.</div>
        <div class="dim">No more cards due right now.</div>
        <button class="outline" onclick={done}>Back</button>
      </div>
    {/if}
  </section>

  <footer>
    {#if current && !flipped}
      <div class="hint dim"><kbd>space</kbd> to flip</div>
    {:else if current && flipped}
      <div class="ratings binary">
        <button class="rate hard" onclick={() => rate(HARD)}>
          <span class="rate-label">Hard</span>
          <span class="rate-key"><kbd>f</kbd></span>
        </button>
        <button class="rate easy" onclick={() => rate(EASY)}>
          <span class="rate-label">Easy</span>
          <span class="rate-key"><kbd>space</kbd></span>
        </button>
      </div>
    {/if}
    {#if error}<div class="error">{error}</div>{/if}
  </footer>

  {#if flash}
    {#key flash.key}
      <div class="rating-flash {flash.tone}">{flash.label}</div>
    {/key}
  {/if}

  {#if showStats && $stats}
    <button class="stats-backdrop" onclick={() => (showStats = false)} aria-label="Close stats"></button>
    <div class="stats-card" role="dialog" aria-label="Session statistics">
      <div class="stats-row"><span class="stats-num">{reviewed}</span><span class="stats-label">reviewed this session</span></div>
      <div class="stats-row"><span class="stats-num">{$stats.due}</span><span class="stats-label">due remaining</span></div>
      <div class="stats-row"><span class="stats-num">{$stats.new}</span><span class="stats-label">new</span></div>
      <div class="stats-row"><span class="stats-num">{$stats.learning}</span><span class="stats-label">in learning</span></div>
      <div class="stats-row"><span class="stats-num">{$stats.total}</span><span class="stats-label">total in deck</span></div>
      <div class="stats-hint dim">press <kbd>t</kbd> or click outside to close</div>
    </div>
  {/if}

  {#if undoable}
    <div class="undo-hint dim">
      <kbd>⌘</kbd>+<kbd>Z</kbd> to undo last
    </div>
  {/if}
</main>

<style>
  .review {
    height: 100%;
    display: grid;
    grid-template-rows: auto 1fr auto;
    position: relative;
  }

  header {
    display: flex;
    align-items: center;
    padding: var(--s-3) var(--s-6);
    border-bottom: 1px solid var(--border);
    gap: var(--s-3);
  }
  .back, .stats-toggle {
    padding: var(--s-2);
    line-height: 0;
  }
  .progress {
    flex: 1;
    text-align: center;
    font-size: var(--t-sm);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--s-2);
  }

  .card-area {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--s-6) var(--s-12);
    overflow: auto;
  }

  .card {
    max-width: 680px;
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s-6);
    text-align: center;
  }

  .face {
    font-size: var(--t-2xl);
    line-height: 1.2;
    font-weight: 500;
    letter-spacing: -0.01em;
    word-break: break-word;
    user-select: text;
  }
  .face.rtl {
    font-family: var(--font-rtl);
    font-size: 56px;
    direction: rtl;
  }
  .face.back {
    font-size: var(--t-xl);
    font-weight: 400;
    color: var(--text-dim);
  }
  .face.back.rtl {
    font-size: var(--t-2xl);
    color: var(--text);
  }

  .divider {
    width: 64px;
    border-top: 1px solid var(--border-strong);
    margin: 0;
  }

  .audio-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--s-2);
    padding: var(--s-1) var(--s-3);
    font-size: var(--t-xs);
    color: var(--text-dim);
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s-3);
  }
  .empty-title {
    font-size: var(--t-xl);
    font-weight: 500;
  }

  footer {
    padding: var(--s-4) var(--s-6) var(--s-6);
    min-height: 96px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .hint { font-size: var(--t-sm); }

  .ratings.binary {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--s-3);
    width: 100%;
    max-width: 520px;
  }
  .rate {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s-1);
    padding: var(--s-4) var(--s-4);
    border: 1px solid var(--border-strong);
    border-radius: var(--r);
    background: var(--bg-elev);
    transition: border-color var(--fast), background var(--fast), transform 80ms ease;
  }
  .rate:hover  { background: var(--bg-hover); }
  .rate:active { transform: translateY(1px); }
  .rate-label  { font-size: var(--t-lg); font-weight: 500; }
  .rate-key    { font-size: var(--t-xs); color: var(--text-dim); }

  .rate.hard:hover { border-color: var(--r-again); color: var(--r-again); }
  .rate.easy:hover { border-color: var(--r-good);  color: var(--r-good);  }

  .error {
    color: var(--r-again);
    font-size: var(--t-sm);
    margin-top: var(--s-2);
  }

  .rating-flash {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    font-size: var(--t-3xl);
    font-weight: 600;
    letter-spacing: -0.03em;
    pointer-events: none;
    animation: flash-pulse 520ms var(--ease) forwards;
    z-index: 10;
  }
  .rating-flash.hard { color: var(--r-again); }
  .rating-flash.easy { color: var(--r-good); }

  @keyframes flash-pulse {
    0%   { opacity: 0; transform: translate(-50%, -50%) scale(0.7); }
    25%  { opacity: 1; transform: translate(-50%, -50%) scale(1.0); }
    100% { opacity: 0; transform: translate(-50%, -50%) scale(1.15); }
  }

  .stats-backdrop {
    position: absolute;
    inset: 0;
    background: transparent;
    border: none;
    z-index: 20;
    cursor: default;
  }
  .stats-card {
    position: absolute;
    top: var(--s-12);
    right: var(--s-6);
    z-index: 21;
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-lg);
    padding: var(--s-4) var(--s-6);
    min-width: 260px;
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
  }
  .stats-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--s-4);
  }
  .stats-num {
    font-size: var(--t-xl);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .stats-label {
    font-size: var(--t-sm);
    color: var(--text-dim);
  }
  .stats-hint {
    margin-top: var(--s-2);
    font-size: var(--t-xs);
    text-align: center;
  }

  .undo-hint {
    position: absolute;
    bottom: var(--s-2);
    left: 50%;
    transform: translateX(-50%);
    font-size: var(--t-xs);
    opacity: 0.7;
    pointer-events: none;
  }
</style>
