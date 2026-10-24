import { useCallback, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { ExpandedPanel } from "./components/ExpandedPanel";
import { hideWidget, quitWidget, setWindowExpanded, setWindowSize } from "./lib/api";
import { usePreferences, useTheme } from "./lib/usePreferences";
import { useQuota } from "./lib/useQuota";

export default function App() {
  const { preferences, update, error: preferencesError } = usePreferences();
  useTheme(preferences);

  const { snapshot, problem, refreshing, refresh } = useQuota();

  useEffect(() => {
    void setWindowExpanded(true);
    const targetWidth = preferences?.width ?? 540;
    void setWindowSize(targetWidth, 380);
  }, [preferences?.width]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen("refresh-quota", () => {
      void refresh();
    }).then((cleanup) => {
      unlisten = cleanup;
    });
    return () => {
      unlisten?.();
    };
  }, [refresh]);

  useEffect(() => {
    const preventContextMenu = (event: MouseEvent) => event.preventDefault();
    document.addEventListener("contextmenu", preventContextMenu);
    return () => document.removeEventListener("contextmenu", preventContextMenu);
  }, []);

  const hide = useCallback(() => {
    void hideWidget();
  }, []);

  const quit = useCallback(() => {
    void quitWidget();
  }, []);

  if (!preferences) {
    return <div className="boot" />;
  }

  return (
    <div className="stage">
      <ExpandedPanel
        snapshot={snapshot}
        problem={problem}
        refreshing={refreshing}
        preferences={preferences}
        preferencesError={preferencesError}
        onRefresh={() => void refresh()}
        onHide={hide}
        onQuit={quit}
        onUpdate={update}
      />
    </div>
  );
}
