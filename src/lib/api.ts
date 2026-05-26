import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

export interface Card {
  id: number;
  front: string;
  back: string;
  has_audio: boolean;
  audio_side: "front" | "back" | "both" | null;
  tags: string[];
  state: number;        // 0 new, 1 learning, 2 review, 3 relearning
  due: number;          // unix ms
  stability: number;
  difficulty: number;
  reps: number;
  lapses: number;
  last_review: number | null;
}

export interface DeckStats {
  total: number;
  due: number;
  new: number;
  learning: number;
}

export interface DeckMeta {
  name: string;
  schema_version: string;
  path: string;
}

export interface DeckSummary {
  path: string;
  name: string;
  due: number;
  new: number;
  total: number;
}

export interface AudioBlob {
  data: number[];
  mime: string;
}

export type Rating = 1 | 2 | 3 | 4; // Again, Hard, Good, Easy

export const RATING_LABEL: Record<Rating, string> = {
  1: "Again",
  2: "Hard",
  3: "Good",
  4: "Easy",
};

export async function pickDeckFile(): Promise<string | null> {
  const result = await openDialog({
    multiple: false,
    directory: false,
    filters: [{ name: "lapse deck", extensions: ["db"] }],
  });
  return typeof result === "string" ? result : null;
}

export const api = {
  openDeck: (path: string) => invoke<DeckMeta>("open_deck", { path }),
  closeDeck: () => invoke<void>("close_deck"),
  currentDeck: () => invoke<DeckMeta | null>("current_deck"),
  deckStats: () => invoke<DeckStats>("deck_stats"),
  nextCard: () => invoke<Card | null>("next_card"),
  rateCard: (id: number, rating: Rating) =>
    invoke<Card | null>("rate_card", { id, rating }),
  undoRating: (card: Card) => invoke<void>("undo_rating", { card }),
  cardAudio: (id: number) => invoke<AudioBlob | null>("card_audio", { id }),
  listDecks: () => invoke<DeckSummary[]>("list_decks"),
  deckDir: () => invoke<string>("deck_dir"),
};

export function audioBlobToUrl(blob: AudioBlob): string {
  const arr = new Uint8Array(blob.data);
  return URL.createObjectURL(new Blob([arr], { type: blob.mime }));
}
