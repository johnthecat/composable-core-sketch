// The extension points this host implements itself: storage in memory, and a
// confirmation the user answers on this page.

import type {
  ProductStorage,
  UserConfirmation,
  UserConfirmationReview,
} from "@sketch/host";

/** How the page answers a confirmation. */
export type ConfirmationMode = "ask" | "approve" | "deny";

/** Shows a review and resolves with the user's answer. */
export type AskUser = (review: UserConfirmationReview) => Promise<boolean>;

/** Storage in one in-memory map. */
export function memoryStorage(): ProductStorage {
  const values = new Map<string, Uint8Array>();
  return {
    read: async (key) => values.get(key),
    write: async (key, value) => {
      values.set(key, value);
    },
  };
}

/** A confirmation answered by `mode()`, asking the user when it is `ask`. */
export function pageConfirmation(
  mode: () => ConfirmationMode,
  ask: AskUser,
): UserConfirmation {
  return {
    confirmUserAction: async (review) => {
      switch (mode()) {
        case "approve":
          return true;
        case "deny":
          return false;
        case "ask":
          return ask(review);
      }
    },
  };
}
