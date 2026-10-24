import React, { useRef } from "react";
import { useSpotify } from "../lib/useSpotify";

export const SpotifyPlayer: React.FC = () => {
  const {
    status,
    elapsedStr,
    remainingStr,
    progressPercent,
    playPause,
    next,
    prev,
    launch,
    seek,
  } = useSpotify();

  const progressRef = useRef<HTMLDivElement>(null);

  const handleProgressClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!progressRef.current) return;
    const rect = progressRef.current.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const fraction = Math.max(0, Math.min(1, clickX / rect.width));
    seek(fraction);
  };

  const isRealTrack = Boolean(status.title && status.artist);
  const isPlaying = status.isPlaying;

  if (!status.isRunning || !isRealTrack) {
    return (
      <div className="rainmeter-player-block rainmeter-player-block--inactive" aria-label="Spotify Player">
        <div className="player-idle-row">
          <svg className="spotify-brand-icon" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
            <path d="M12 0C5.373 0 0 5.373 0 12s5.373 12 12 12 12-5.373 12-12S18.627 0 12 0zm5.498 17.306c-.215.353-.675.466-1.028.25-2.817-1.722-6.362-2.112-10.538-1.157-.404.093-.812-.162-.905-.566-.093-.404.162-.812.566-.905 4.575-1.047 8.498-.601 11.655 1.332.353.215.466.675.25 1.028zm1.468-3.26c-.27.44-.847.58-1.287.31-3.224-1.982-8.14-2.556-11.954-1.398-.498.151-1.029-.133-1.18-.63-.151-.498.133-1.029.63-1.18 4.364-1.325 9.774-.683 13.481 1.597.44.27.58.847.31 1.287zm.126-3.41c-3.864-2.294-10.245-2.506-13.935-1.385-.593.18-1.222-.156-1.402-.749-.18-.593.156-1.222.749-1.402 4.238-1.287 11.284-1.037 15.733 1.604.533.316.708 1.01.392 1.543-.316.533-1.01.708-1.543.392z"/>
          </svg>
          <span className="player-idle-text">
            {status.isRunning ? "Spotify Paused" : "Spotify Inactive"}
          </span>
          <button
            type="button"
            className="player-idle-btn"
            onClick={() => void (status.isRunning ? playPause() : launch())}
            data-interactive
            title={status.isRunning ? "Resume playback" : "Launch Spotify"}
          >
            <svg viewBox="0 0 24 24" className="control-glyph-small" aria-hidden="true">
              <polygon points="8,5 19,12 8,19" fill="currentColor" />
            </svg>
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="rainmeter-player-block" aria-label="Spotify Player">
      {/* Spotify Brand Tag */}
      <div className="spotify-brand-tag">
        <svg className="spotify-brand-icon" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
          <path d="M12 0C5.373 0 0 5.373 0 12s5.373 12 12 12 12-5.373 12-12S18.627 0 12 0zm5.498 17.306c-.215.353-.675.466-1.028.25-2.817-1.722-6.362-2.112-10.538-1.157-.404.093-.812-.162-.905-.566-.093-.404.162-.812.566-.905 4.575-1.047 8.498-.601 11.655 1.332.353.215.466.675.25 1.028zm1.468-3.26c-.27.44-.847.58-1.287.31-3.224-1.982-8.14-2.556-11.954-1.398-.498.151-1.029-.133-1.18-.63-.151-.498.133-1.029.63-1.18 4.364-1.325 9.774-.683 13.481 1.597.44.27.58.847.31 1.287zm.126-3.41c-3.864-2.294-10.245-2.506-13.935-1.385-.593.18-1.222-.156-1.402-.749-.18-.593.156-1.222.749-1.402 4.238-1.287 11.284-1.037 15.733 1.604.533.316.708 1.01.392 1.543-.316.533-1.01.708-1.543.392z"/>
        </svg>
        <span className="spotify-brand-label">SPOTIFY</span>
      </div>
      {/* 1. Progress timeline scrubber */}
      <div className="player-timeline-row">
        <span className="player-time player-time--elapsed">{elapsedStr}</span>

        <div
          ref={progressRef}
          className="player-progress-track"
          onClick={handleProgressClick}
          role="slider"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(progressPercent)}
          aria-label="Track progress"
          data-interactive
        >
          <div
            className="player-progress-fill"
            style={{ width: `${progressPercent}%` }}
          />
          <div
            className="player-scrubber-dot"
            style={{ left: `${progressPercent}%` }}
          />
        </div>

        <span className="player-time player-time--remaining">{remainingStr}</span>
      </div>

      {/* 2. Artist & Title Display */}
      <div className="player-meta-group">
        <div className="player-artist-name" title={status.artist}>
          {status.artist.toUpperCase()}
        </div>
        <div className="player-track-name" title={status.title}>
          {status.title}
        </div>
      </div>

      {/* 3. Outline Playback Controls */}
      <div className="player-controls-row">
        {/* Previous / Rewind: << */}
        <button
          type="button"
          className="player-btn player-btn--prev"
          onClick={() => void prev()}
          title="Previous Track"
          aria-label="Previous Track"
          data-interactive
        >
          <svg viewBox="0 0 28 20" className="control-glyph" aria-hidden="true">
            <path
              d="M13 3L5 10L13 17 M23 3L15 10L23 17"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.2"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </button>

        {/* Play / Pause */}
        <button
          type="button"
          className="player-btn player-btn--play"
          onClick={() => void playPause()}
          title={isPlaying ? "Pause" : "Play"}
          aria-label={isPlaying ? "Pause" : "Play"}
          data-interactive
        >
          {isPlaying ? (
            <svg viewBox="0 0 24 24" className="control-glyph" aria-hidden="true">
              <rect x="5" y="3" width="4.5" height="18" rx="2" fill="none" stroke="currentColor" strokeWidth="2.2" />
              <rect x="14.5" y="3" width="4.5" height="18" rx="2" fill="none" stroke="currentColor" strokeWidth="2.2" />
            </svg>
          ) : (
            <svg viewBox="0 0 24 24" className="control-glyph" aria-hidden="true">
              <polygon points="7,4 20,12 7,20" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinejoin="round" />
            </svg>
          )}
        </button>

        {/* Next / Fast-Forward: >> */}
        <button
          type="button"
          className="player-btn player-btn--next"
          onClick={() => void next()}
          title="Next Track"
          aria-label="Next Track"
          data-interactive
        >
          <svg viewBox="0 0 28 20" className="control-glyph" aria-hidden="true">
            <path
              d="M5 3L13 10L5 17 M15 3L23 10L15 17"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.2"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </button>
      </div>
    </div>
  );
};
