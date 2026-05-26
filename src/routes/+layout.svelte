<script lang="ts">
  import "../app.css";
  import { theme } from "$lib/store";
  import { onMount } from "svelte";

  let { children } = $props();

  onMount(() => {
    // Apply persisted theme on first paint
    const t = document.documentElement.getAttribute("data-theme");
    if (t === "dark" || t === "light") theme.set(t);
  });
</script>

{@render children()}
