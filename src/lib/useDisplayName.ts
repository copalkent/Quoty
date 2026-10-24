import { useCallback, useEffect, useState } from "react";

const STORAGE_KEY = "quoty_display_name";

/**
 * Custom hook to get and set the user's preferred display name,
 * stored locally in localStorage.
 */
export function useDisplayName(): [string, (name: string) => void] {
  const [displayName, setDisplayNameState] = useState<string>(() => {
    try {
      return localStorage.getItem(STORAGE_KEY) || "";
    } catch {
      return "";
    }
  });

  useEffect(() => {
    const handleStorage = (event: StorageEvent) => {
      if (event.key === STORAGE_KEY) {
        setDisplayNameState(event.newValue || "");
      }
    };
    window.addEventListener("storage", handleStorage);
    return () => window.removeEventListener("storage", handleStorage);
  }, []);

  const setDisplayName = useCallback((nextName: string) => {
    const trimmed = nextName.trim();
    setDisplayNameState(trimmed);
    try {
      if (trimmed) {
        localStorage.setItem(STORAGE_KEY, trimmed);
      } else {
        localStorage.removeItem(STORAGE_KEY);
      }
      // Dispatch custom event for same-window updates if needed
      window.dispatchEvent(new Event("quoty-name-changed"));
    } catch {
      // Ignore storage errors (e.g. private mode or quota exceeded)
    }
  }, []);

  useEffect(() => {
    const handleCustom = () => {
      try {
        setDisplayNameState(localStorage.getItem(STORAGE_KEY) || "");
      } catch {
        // ignore
      }
    };
    window.addEventListener("quoty-name-changed", handleCustom);
    return () => window.removeEventListener("quoty-name-changed", handleCustom);
  }, []);

  return [displayName, setDisplayName];
}
