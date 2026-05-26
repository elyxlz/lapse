import { writable } from "svelte/store";
import type { DeckMeta, DeckStats } from "./api";

export const deck = writable<DeckMeta | null>(null);
export const stats = writable<DeckStats | null>(null);

const THEME_KEY = "lapse:theme";
type Theme = "dark" | "light";

function readTheme(): Theme {
  if (typeof localStorage === "undefined") return "dark";
  const v = localStorage.getItem(THEME_KEY);
  return v === "light" ? "light" : "dark";
}

export const theme = writable<Theme>(readTheme());

theme.subscribe((t) => {
  if (typeof document === "undefined") return;
  document.documentElement.setAttribute("data-theme", t);
  if (typeof localStorage !== "undefined") localStorage.setItem(THEME_KEY, t);
});
