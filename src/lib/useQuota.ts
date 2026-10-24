import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ensureOnScreen, fetchSnapshot } from "./api";
import type { QuotaProblem, QuotaSnapshot } from "./quota";

const REFRESH_INTERVAL_MS = 60_000;

interface QuotaState {
  snapshot: QuotaSnapshot | null;
  problem: QuotaProblem | null;
  refreshing: boolean;
  refresh: () => Promise<void>;
}

export function useQuota(): QuotaState {
  const [snapshot, setSnapshot] = useState<QuotaSnapshot | null>(null);
  const [problem, setProblem] = useState<QuotaProblem | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const inFlight = useRef(false);
  const mounted = useRef(true);

  const refresh = useCallback(async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    setRefreshing(true);

    try {
      const next = await fetchSnapshot();
      if (!mounted.current) return;
      // Never keep showing old numbers as if they were live: the backend
      // reports unavailability explicitly.
      setSnapshot(next);
      setProblem(next.status === "unavailable" ? next.error : null);
    } catch {
      if (!mounted.current) return;
      setSnapshot(null);
      setProblem({
        code: "widget_unavailable",
        title: "Quota check failed",
        detail:
          "The widget couldn't reach its local quota service. Try again, or restart Antigravity.",
      });
    } finally {
      inFlight.current = false;
      if (mounted.current) setRefreshing(false);
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    void refresh();

    const interval = window.setInterval(() => {
      void refresh();
    }, REFRESH_INTERVAL_MS);

    const onVisibility = () => {
      if (!document.hidden) {
        void ensureOnScreen();
        void refresh();
      }
    };
    document.addEventListener("visibilitychange", onVisibility);

    const unlisten = listen("refresh-quota", () => {
      void refresh();
    });

    return () => {
      mounted.current = false;
      window.clearInterval(interval);
      document.removeEventListener("visibilitychange", onVisibility);
      void unlisten.then((off) => off());
    };
  }, [refresh]);

  return { snapshot, problem, refreshing, refresh };
}
