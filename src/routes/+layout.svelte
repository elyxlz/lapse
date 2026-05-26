<script lang="ts">
  import "../app.css";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import { theme } from "$lib/store";
  import { onMount } from "svelte";

  let { children } = $props();

  onMount(() => {
    const t = document.documentElement.getAttribute("data-theme");
    if (t === "dark" || t === "light") theme.set(t);
  });
</script>

<div class="app-shell">
  <TitleBar />
  <div class="app-body">
    {@render children()}
  </div>
</div>

<style>
  .app-shell {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .app-body {
    flex: 1;
    overflow: hidden;
    min-height: 0;
  }
</style>
