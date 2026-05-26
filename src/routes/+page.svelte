<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { api, pickDeckFile, type DeckSummary } from "$lib/api";
  import { deck, stats } from "$lib/store";

  let decks = $state<DeckSummary[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let opening = $state<string | null>(null);

  onMount(async () => {
    let d: Awaited<ReturnType<typeof api.currentDeck>> = null;
    try {
      d = await api.currentDeck();
    } catch (e) {
      error = String(e);
    }
    if (d) {
      deck.set(d);
      try {
        stats.set(await api.deckStats());
      } catch (e) {
        error = String(e);
      }
    } else {
      try {
        decks = await api.listDecks();
      } catch (e) {
        error = String(e);
      }
    }
  });

  async function refreshList() {
    decks = await api.listDecks();
  }

  async function openByPath(path: string) {
    if (opening) return;
    opening = path;
    error = null;
    try {
      const d = await api.openDeck(path);
      deck.set(d);
      stats.set(await api.deckStats());
    } catch (e) {
      error = String(e);
    } finally {
      opening = null;
    }
  }

  async function openExternal() {
    const path = await pickDeckFile();
    if (!path) return;
    await openByPath(path);
  }

  async function closeDeck() {
    error = null;
    await api.closeDeck();
    deck.set(null);
    stats.set(null);
    try {
      await refreshList();
    } catch (e) {
      error = String(e);
    }
  }

  function startReview() {
    goto("/review");
  }
</script>

<main class="page">
  <header>
    <div class="spacer"></div>
    <a class="settings-link" href="/settings" aria-label="Settings">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="3"></circle>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
      </svg>
    </a>
  </header>

  <section class="content">
    {#if $deck && $stats}
      <div class="deck-card">
        <div class="deck-name">{$deck.name}</div>
        <div class="deck-stats">
          <div class="stat">
            <span class="stat-num">{$stats.due}</span>
            <span class="stat-label">due</span>
          </div>
          <div class="stat">
            <span class="stat-num">{$stats.new}</span>
            <span class="stat-label">new</span>
          </div>
          <div class="stat">
            <span class="stat-num">{$stats.total}</span>
            <span class="stat-label">total</span>
          </div>
        </div>

        {#if $stats.due > 0}
          <button class="primary big" onclick={startReview}>
            Start review
            <kbd>↵</kbd>
          </button>
        {:else}
          <div class="empty-state">Nothing due. Come back later.</div>
        {/if}

        <button class="ghost small" onclick={closeDeck}>Close deck</button>
      </div>
    {:else if decks.length > 0}
      <div class="list-wrap">
        <ul class="deck-list">
          {#each decks as d (d.path)}
            <li>
              <button class="deck-row" onclick={() => openByPath(d.path)} disabled={opening === d.path}>
                <span class="row-name">{d.name}</span>
                <span class="row-stats dim">
                  <span>{d.due} due</span>
                  <span class="dot">·</span>
                  <span>{d.new} new</span>
                  <span class="dot">·</span>
                  <span>{d.total} total</span>
                </span>
              </button>
            </li>
          {/each}
        </ul>
        <button class="ghost small external" onclick={openExternal} disabled={!!opening}>
          {opening ? "Opening…" : "Open external .db…"}
        </button>
      </div>
    {:else}
      <div class="welcome">
        <div class="welcome-title">No decks yet</div>
        <div class="welcome-sub">
          Drop a <span class="mono">.db</span> file into the lapse deck folder,
          or open one from anywhere.
        </div>
        <button class="primary big" onclick={openExternal} disabled={loading}>
          {loading ? "Opening…" : "Open deck…"}
        </button>
      </div>
    {/if}

    {#if error}
      <div class="error">{error}</div>
    {/if}
  </section>
</main>

<svelte:window onkeydown={(e) => {
  if (e.key === "Enter" && $deck && $stats && $stats.due > 0) startReview();
}} />

<style>
  .page {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: var(--s-6);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .settings-link {
    color: var(--text-dim);
    padding: var(--s-2);
    border-radius: var(--r);
    line-height: 0;
    transition: color var(--fast), background var(--fast);
  }
  .settings-link:hover {
    color: var(--text);
    background: var(--bg-hover);
  }

  .content {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-direction: column;
    gap: var(--s-6);
  }

  .welcome {
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s-4);
    max-width: 420px;
  }
  .welcome-title {
    font-size: var(--t-xl);
    font-weight: 500;
  }
  .welcome-sub {
    color: var(--text-dim);
    font-size: var(--t-sm);
    margin-bottom: var(--s-4);
    line-height: 1.5;
  }

  .deck-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s-6);
    max-width: 480px;
    width: 100%;
  }

  .deck-name {
    font-size: var(--t-2xl);
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .deck-stats {
    display: flex;
    gap: var(--s-12);
  }

  .stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s-1);
  }
  .stat-num {
    font-size: var(--t-xl);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .stat-label {
    font-size: var(--t-xs);
    color: var(--text-dim);
    text-transform: lowercase;
    letter-spacing: 0.05em;
  }

  .list-wrap {
    width: 100%;
    max-width: 520px;
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
  }

  .deck-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--s-1);
  }

  .deck-row {
    width: 100%;
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--s-4);
    padding: var(--s-3) var(--s-4);
    border: 1px solid var(--border);
    border-radius: var(--r);
    background: var(--bg-elev);
    text-align: left;
    transition: border-color var(--fast), background var(--fast);
  }
  .deck-row:hover:not(:disabled) {
    border-color: var(--border-strong);
    background: var(--bg-hover);
  }
  .deck-row:disabled {
    opacity: 0.5;
    cursor: wait;
  }
  .row-name {
    font-size: var(--t-base);
    font-weight: 500;
  }
  .row-stats {
    font-size: var(--t-sm);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    gap: var(--s-2);
  }
  .dot { color: var(--text-faint); }

  .external {
    align-self: center;
    margin-top: var(--s-2);
  }

  .big {
    padding: var(--s-3) var(--s-8);
    font-size: var(--t-base);
    display: inline-flex;
    align-items: center;
    gap: var(--s-3);
  }

  .small {
    font-size: var(--t-sm);
    padding: var(--s-1) var(--s-3);
  }

  .empty-state {
    color: var(--text-dim);
    font-size: var(--t-sm);
  }

  .error {
    color: var(--r-again);
    font-size: var(--t-sm);
    margin-top: var(--s-4);
    text-align: center;
  }
</style>
