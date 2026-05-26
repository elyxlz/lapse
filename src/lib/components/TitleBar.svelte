<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";

  const win = getCurrentWindow();
  let maximized = $state(false);

  async function refreshMaximized() {
    maximized = await win.isMaximized();
  }

  // Track maximized state for the toggle icon
  win.onResized(() => refreshMaximized());
  refreshMaximized();

  async function minimize() { await win.minimize(); }
  async function toggleMax() {
    if (await win.isMaximized()) await win.unmaximize();
    else await win.maximize();
    refreshMaximized();
  }
  async function close() { await win.close(); }
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>lapse</div>
  <div class="controls">
    <button class="ctl min" onclick={minimize} aria-label="Minimize" title="Minimize">
      <svg width="10" height="10" viewBox="0 0 10 10"><line x1="2" y1="5" x2="8" y2="5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>
    </button>
    <button class="ctl max" onclick={toggleMax} aria-label="Maximize" title={maximized ? "Restore" : "Maximize"}>
      {#if maximized}
        <svg width="10" height="10" viewBox="0 0 10 10"><rect x="2.5" y="3.5" width="5" height="4" fill="none" stroke="currentColor" stroke-width="1"/><rect x="3.5" y="2.5" width="4" height="1" fill="none" stroke="currentColor" stroke-width="1"/></svg>
      {:else}
        <svg width="10" height="10" viewBox="0 0 10 10"><rect x="2" y="2" width="6" height="6" fill="none" stroke="currentColor" stroke-width="1"/></svg>
      {/if}
    </button>
    <button class="ctl close" onclick={close} aria-label="Close" title="Close">
      <svg width="10" height="10" viewBox="0 0 10 10"><line x1="2.5" y1="2.5" x2="7.5" y2="7.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/><line x1="7.5" y1="2.5" x2="2.5" y2="7.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>
    </button>
  </div>
</div>

<style>
  .titlebar {
    height: 32px;
    background: var(--bg);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 0 0 var(--s-4);
    -webkit-app-region: drag;
    user-select: none;
    flex-shrink: 0;
  }
  .brand {
    font-size: var(--t-xs);
    color: var(--text-faint);
    letter-spacing: 0.04em;
    font-weight: 500;
    pointer-events: none;
  }
  .controls {
    display: flex;
    -webkit-app-region: no-drag;
  }
  .ctl {
    width: 36px;
    height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 0;
    padding: 0;
    color: var(--text-faint);
    transition: background var(--fast), color var(--fast);
  }
  .ctl:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .ctl.close:hover {
    background: #e81123;
    color: #fff;
  }
</style>
