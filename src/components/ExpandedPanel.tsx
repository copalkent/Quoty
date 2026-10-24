import React, { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type {
  Preferences,
  PreferencesPatch,
  QuotaProblem,
  QuotaSnapshot,
} from "../lib/quota";
import { useWindowDrag } from "../lib/useWindowDrag";
import { ClockHeader } from "./ClockHeader";
import { RainmeterQuota } from "./RainmeterQuota";
import { SettingsSheet } from "./SettingsSheet";
import { SpotifyPlayer } from "./SpotifyPlayer";

interface Props {
  snapshot: QuotaSnapshot | null;
  problem: QuotaProblem | null;
  refreshing: boolean;
  preferences: Preferences;
  preferencesError: string | null;
  leaving?: boolean;
  onRefresh: () => void;
  onHide: () => void;
  onQuit: () => void;
  onUpdate: (patch: PreferencesPatch) => Promise<void>;
}

export const ExpandedPanel: React.FC<Props> = ({
  snapshot,
  problem,
  refreshing,
  preferences,
  preferencesError,
  leaving = false,
  onRefresh,
  onHide,
  onQuit,
  onUpdate,
}) => {
  const [sheetOpen, setSheetOpen] = useState(false);
  const isLocked = Boolean(preferences.locked);
  const drag = useWindowDrag(undefined, isLocked);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen("open-settings", () => {
      setSheetOpen(true);
    }).then((cleanup) => {
      unlisten = cleanup;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setSheetOpen(false);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <div
      className={`panel${leaving ? " is-leaving" : ""}${isLocked ? " is-locked" : " is-unlocked"}`}
      onPointerDown={isLocked ? undefined : drag.onPointerDown}
    >
      <div className="rainmeter-main-widget">
        {/* 1. Large Saturday Weekday with Time & Date on Right */}
        <ClockHeader onPointerDown={isLocked ? undefined : drag.onPointerDown} />

        {/* 2. Spotify Media Player */}
        <SpotifyPlayer />

        {/* 3. Antigravity Quotas (non-scrollable, pure transparent) */}
        <RainmeterQuota
          snapshot={snapshot}
          problem={problem}
          refreshing={refreshing}
          onRefresh={onRefresh}
        />
      </div>

      {sheetOpen ? (
        <SettingsSheet
          preferences={preferences}
          preferencesError={preferencesError}
          source={snapshot?.source ?? null}
          onClose={() => setSheetOpen(false)}
          onUpdate={onUpdate}
          onHide={onHide}
          onQuit={onQuit}
        />
      ) : null}
    </div>
  );
};
