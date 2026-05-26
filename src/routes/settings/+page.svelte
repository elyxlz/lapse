<script lang="ts">
  import { theme } from "$lib/store";

  function setTheme(t: "dark" | "light") {
    theme.set(t);
  }
</script>

<main class="settings">
  <header>
    <a class="back ghost" href="/" aria-label="Back">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M19 12H5M12 19l-7-7 7-7"></path>
      </svg>
    </a>
    <div class="title">Settings</div>
  </header>

  <section>
    <div class="row-item">
      <div class="label">
        <div>Theme</div>
        <div class="dim small">Appearance of the app.</div>
      </div>
      <div class="row toggle">
        <button
          class:active={$theme === "dark"}
          onclick={() => setTheme("dark")}>Dark</button>
        <button
          class:active={$theme === "light"}
          onclick={() => setTheme("light")}>Light</button>
      </div>
    </div>
  </section>

  <footer class="dim small">
    <span>lapse · v0.1.0</span>
  </footer>
</main>

<style>
  .settings {
    height: 100vh;
    display: grid;
    grid-template-rows: auto 1fr auto;
    padding: var(--s-6);
    max-width: 640px;
    margin: 0 auto;
    width: 100%;
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    margin-bottom: var(--s-8);
  }
  .back {
    padding: var(--s-2);
    line-height: 0;
    color: var(--text-dim);
  }
  .back:hover { color: var(--text); background: var(--bg-hover); }
  .title { font-size: var(--t-lg); font-weight: 500; }

  .row-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--s-4) 0;
    border-bottom: 1px solid var(--border);
  }
  .label > div:first-child {
    font-size: var(--t-base);
  }
  .small { font-size: var(--t-sm); }

  .toggle {
    display: inline-flex;
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-radius: var(--r);
    padding: 2px;
  }
  .toggle button {
    padding: var(--s-1) var(--s-3);
    font-size: var(--t-sm);
    color: var(--text-dim);
    border-radius: var(--r-sm);
    border: none;
  }
  .toggle button.active {
    background: var(--bg-hover);
    color: var(--text);
  }

  footer {
    padding-top: var(--s-4);
    text-align: center;
    font-size: var(--t-sm);
  }
</style>
