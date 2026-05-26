<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { goto } from "$app/navigation";
  import { api, audioBlobToUrl, type Card, type Rating } from "$lib/api";
  import { deck, stats } from "$lib/store";

  let current = $state<Card | null>(null);
  let flipped = $state(false);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let audioUrl: string | null = null;
  let audioEl: HTMLAudioElement;
  let reviewed = $state(0);

  function isLikelyRtl(text: string): boolean {
    return /[֐-ࣿיִ-ﻼ]/.test(text);
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
    } catch (e) {
      // audio failure shouldn't block review
    }
  }

  function revokeAudio() {
    if (audioUrl) {
      URL.revokeObjectURL(audioUrl);
      audioUrl = null;
    }
  }

  function setCard(c: Card | null) {
    current = c;
    flipped = false;
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
    // `loading` guard prevents double-submission on rapid keypresses /
    // clicks: a second event fires before the first invoke resolves, but
    // after `loading = true` is observable.
    if (!flipped || !current || loading) return;
    const id = current.id;
    loading = true;
    try {
      const next = await api.rateCard(id, r);
      reviewed += 1;
      setCard(next);
      if ($deck) {
        stats.set(await api.deckStats());
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function done() {
    goto("/");
  }

  onMount(async () => {
    if (!$deck) {
      goto("/");
      return;
    }
    try {
      const c = await api.nextCard();
      setCard(c);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  });

  onDestroy(() => {
    revokeAudio();
  });

  function onKeydown(e: KeyboardEvent) {
    if (e.repeat) return;
    if (e.key === "Escape") {
      done();
      return;
    }
    if (e.code === "Space" || e.key === " ") {
      e.preventDefault();
      if (!flipped) flip();
      else rate(3); // Anki convention: space after flip = Good
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      if (!flipped) flip();
      else rate(3);
      return;
    }
    if (!flipped) return;
    if (e.key === "1") rate(1);
    else if (e.key === "2") rate(2);
    else if (e.key === "3") rate(3);
    else if (e.key === "4") rate(4);
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
      <span class="mono">{reviewed} reviewed · {$stats?.due ?? 0} left</span>
    </div>
    <div class="spacer-end"></div>
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
      <div class="ratings">
        <button class="rate again" onclick={() => rate(1)}>
          <span class="rate-key"><kbd>1</kbd></span>
          <span class="rate-label">Again</span>
        </button>
        <button class="rate hard" onclick={() => rate(2)}>
          <span class="rate-key"><kbd>2</kbd></span>
          <span class="rate-label">Hard</span>
        </button>
        <button class="rate good" onclick={() => rate(3)}>
          <span class="rate-key"><kbd>3</kbd></span>
          <span class="rate-label">Good</span>
        </button>
        <button class="rate easy" onclick={() => rate(4)}>
          <span class="rate-key"><kbd>4</kbd></span>
          <span class="rate-label">Easy</span>
        </button>
      </div>
    {/if}
    {#if error}<div class="error">{error}</div>{/if}
  </footer>
</main>

<style>
  .review {
    height: 100vh;
    display: grid;
    grid-template-rows: auto 1fr auto;
  }

  header {
    display: flex;
    align-items: center;
    padding: var(--s-3) var(--s-6);
    border-bottom: 1px solid var(--border);
  }
  .back {
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
  .spacer-end { width: 30px; }

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
    min-height: 88px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .hint {
    font-size: var(--t-sm);
  }

  .ratings {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--s-3);
    width: 100%;
    max-width: 680px;
  }

  .rate {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s-1);
    padding: var(--s-3) var(--s-4);
    border: 1px solid var(--border-strong);
    border-radius: var(--r);
    background: var(--bg-elev);
    transition: border-color var(--fast), background var(--fast), transform 80ms ease;
  }
  .rate:hover    { background: var(--bg-hover); }
  .rate:active   { transform: translateY(1px); }
  .rate-key      { line-height: 1; }
  .rate-label    { font-size: var(--t-sm); font-weight: 500; }

  .rate.again:hover { border-color: var(--r-again); color: var(--r-again); }
  .rate.hard:hover  { border-color: var(--r-hard);  color: var(--r-hard);  }
  .rate.good:hover  { border-color: var(--r-good);  color: var(--r-good);  }
  .rate.easy:hover  { border-color: var(--r-easy);  color: var(--r-easy);  }

  .error {
    color: var(--r-again);
    font-size: var(--t-sm);
    margin-top: var(--s-2);
  }
</style>
