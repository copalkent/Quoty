import { useCallback, useEffect, useRef, useState } from "react";
import {
  fetchSpotifyStatus,
  spotifyLaunch,
  spotifyNext,
  spotifyPlayPause,
  spotifyPrev,
  type SpotifyStatus,
} from "./api";

function formatDuration(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  const mins = Math.floor(s / 60);
  const secs = s % 60;
  return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
}

export function useSpotify() {
  const [status, setStatus] = useState<SpotifyStatus>({
    isRunning: false,
    isPlaying: false,
    artist: "",
    title: "",
  });

  const [positionSecs, setPositionSecs] = useState(0);
  const [durationSecs, setDurationSecs] = useState(216); // default 3:36
  const lastTitleRef = useRef("");

  // Poll status periodically
  const pollStatus = useCallback(async () => {
    try {
      const res = await fetchSpotifyStatus();
      if (res.title && res.title !== lastTitleRef.current) {
        lastTitleRef.current = res.title;
        setPositionSecs(0);
        setDurationSecs(210);
      }
      setStatus(res);
    } catch {
      // In web preview or non-tauri context, keep last status
    }
  }, []);

  useEffect(() => {
    void pollStatus();
    const interval = window.setInterval(pollStatus, 1800);
    return () => window.clearInterval(interval);
  }, [pollStatus]);

  // Advance playback timer when active
  useEffect(() => {
    if (!status.isPlaying) return;

    const timer = window.setInterval(() => {
      setPositionSecs((prev) => {
        if (prev >= durationSecs) return 0;
        return prev + 1;
      });
    }, 1000);

    return () => window.clearInterval(timer);
  }, [status.isPlaying, durationSecs]);

  const playPause = useCallback(async () => {
    try {
      await spotifyPlayPause();
      // Optimistic toggle
      setStatus((prev) => ({ ...prev, isPlaying: !prev.isPlaying }));
      setTimeout(pollStatus, 400);
    } catch {
      setStatus((prev) => ({ ...prev, isPlaying: !prev.isPlaying }));
    }
  }, [pollStatus]);

  const next = useCallback(async () => {
    try {
      await spotifyNext();
      setTimeout(pollStatus, 600);
    } catch {
      // ignore
    }
  }, [pollStatus]);

  const prev = useCallback(async () => {
    try {
      await spotifyPrev();
      setTimeout(pollStatus, 600);
    } catch {
      // ignore
    }
  }, [pollStatus]);

  const launch = useCallback(async () => {
    try {
      await spotifyLaunch();
      setTimeout(pollStatus, 1500);
    } catch {
      // ignore
    }
  }, [pollStatus]);

  const seek = useCallback(
    (fraction: number) => {
      const clamped = Math.max(0, Math.min(1, fraction));
      setPositionSecs(Math.round(clamped * durationSecs));
    },
    [durationSecs],
  );

  const remainingSecs = Math.max(0, durationSecs - positionSecs);
  const elapsedStr = formatDuration(positionSecs);
  const remainingStr = `-${formatDuration(remainingSecs)}`;
  const progressPercent = durationSecs > 0 ? (positionSecs / durationSecs) * 100 : 0;

  return {
    status,
    positionSecs,
    durationSecs,
    elapsedStr,
    remainingStr,
    progressPercent,
    playPause,
    next,
    prev,
    launch,
    seek,
  };
}
