/**
 * Secure key/value store for the server URL and the session token.
 *
 * Inside Tauri this goes through the Rust `secure_*` commands (Windows Credential
 * Manager today, Android keystore later). In a plain browser (`npm run dev` for UI
 * work) it falls back to localStorage so the app stays usable — never ship that path.
 */

const KEYS = {
  serverUrl: "server-url",
  token: "session-token",
} as const;

export type SecureKey = (typeof KEYS)[keyof typeof KEYS];
export const SECURE_KEYS = KEYS;

function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

const LS_PREFIX = "renewal.secure.";

export async function secureGet(key: SecureKey): Promise<string | null> {
  if (inTauri()) return invoke<string | null>("secure_get", { key });
  try {
    return localStorage.getItem(LS_PREFIX + key);
  } catch {
    return null;
  }
}

export async function secureSet(key: SecureKey, value: string): Promise<void> {
  if (inTauri()) return invoke<void>("secure_set", { key, value });
  try {
    localStorage.setItem(LS_PREFIX + key, value);
  } catch {
    // ignore: private mode / storage disabled
  }
}

export async function secureDelete(key: SecureKey): Promise<void> {
  if (inTauri()) return invoke<void>("secure_delete", { key });
  try {
    localStorage.removeItem(LS_PREFIX + key);
  } catch {
    // ignore
  }
}

export async function platform(): Promise<string> {
  if (inTauri()) return invoke<string>("platform");
  return "browser";
}
