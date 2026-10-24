import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { fetchPreferences, savePreferences } from "./api";
import type { Preferences, PreferencesPatch } from "./quota";

interface PreferencesState {
  preferences: Preferences | null;
  update: (patch: PreferencesPatch) => Promise<void>;
  error: string | null;
}

export function usePreferences(): PreferencesState {
  const [preferences, setPreferences] = useState<Preferences | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;

    fetchPreferences()
      .then((loaded) => {
        if (alive) setPreferences(loaded);
      })
      .catch(() => {
        if (alive) setError("Couldn't read saved preferences.");
      });

    const unlisten = listen<Preferences>("preferences-changed", (event) => {
      setPreferences(event.payload);
    });

    return () => {
      alive = false;
      void unlisten.then((off) => off());
    };
  }, []);

  const update = useCallback(async (patch: PreferencesPatch) => {
    setPreferences((current) => (current ? { ...current, ...patch } : current));
    try {
      const saved = await savePreferences(patch);
      setPreferences(saved);
      setError(null);
    } catch {
      setError("That setting couldn't be saved.");
      // Re-sync with the backend's authoritative state.
      try {
        setPreferences(await fetchPreferences());
      } catch {
        /* keep the optimistic value */
      }
    }
  }, []);

  return { preferences, update, error };
}

/** Applies the theme to the document, following the OS when set to system. */
export function useTheme(preferences: Preferences | null): void {
  useEffect(() => {
    if (!preferences) return;
    const root = document.documentElement;
    const media = window.matchMedia("(prefers-color-scheme: dark)");

    const apply = () => {
      const resolved =
        preferences.theme === "system" ? (media.matches ? "dark" : "light") : preferences.theme;
      root.dataset.theme = resolved;
      root.style.colorScheme = resolved;
    };

    apply();
    if (preferences.theme !== "system") return;

    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [preferences]);
}
