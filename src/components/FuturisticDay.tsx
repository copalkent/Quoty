import React from "react";

interface Props {
  dayName: string;
}

// Geometric futuristic vector glyphs matching the reference image's stenciled typography
const GLYPHS: Record<string, string> = {
  F: "M2 4h18v4H7v4h10v4H7v12H2V4z",
  R: "M2 4h12c4.4 0 7 2.6 7 6.5 0 3-1.6 5.2-4.2 6.1L22 28h-5.5l-4.7-10.5H7V28H2V4zm5 9.5h6.5c1.7 0 3-.9 3-2.5s-1.3-2.5-3-2.5H7v5z",
  I: "M4 4h5v24H4V4z",
  D: "M2 4h10c6.5 0 11 4.5 11 12s-4.5 12-11 12H2V4zm5 19.5h5c3.8 0 6.5-2.8 6.5-7.5s-2.7-7.5-6.5-7.5H7v15z",
  A: "M12 4l9.5 24h-4.8l-2.4-6.5H9.7L7.3 28H2.5L12 4zm0 8.5L10.3 18h3.4L12 12.5z",
  Y: "M2 4h5l5 8.5L17 4h5l-7.5 12.5V28h-5V16.5L2 4z",
  S: "M19 8.5h-4.5c0-1.5-1.2-2.5-3-2.5s-3 1-3 2.2c0 3.8 11.5 2.5 11.5 11.8 0 4.5-3.5 8-8.5 8s-9-3.5-9-8h4.5c0 2 1.8 3.5 4.5 3.5s4-1.2 4-2.8c0-4.2-11.5-2.8-11.5-12C4 4.5 7.5 2 11.5 2s8 2.8 8 6.5z",
  T: "M2 4h20v4.5h-7.5V28h-5V8.5H2V4z",
  U: "M2 4h5v15c0 3.5 2.5 5 5 5s5-1.5 5-5V4h5v15c0 6.5-4.5 9-10 9s-10-2.5-10-9V4z",
  M: "M2 4h5l5 10L17 4h5v24h-5V13l-4 8h-2l-4-8v15H2V4z",
  O: "M12 2c6.6 0 11 4.5 11 12s-4.4 12-11 12S1 21.5 1 14 5.4 2 12 2zm0 19.5c3.8 0 6.5-3.2 6.5-7.5s-2.7-7.5-6.5-7.5S5.5 9.7 5.5 14s2.7 7.5 6.5 7.5z",
  N: "M2 4h5l10 14.5V4h5v24h-5L7 13.5V28H2V4z",
  W: "M2 4h5l4 14.5 4-14.5h5l-6 24h-4l-4-15-4 15H2L2 4z",
  E: "M2 4h18v4.5H7v5h11v4H7v6h13V28H2V4z",
  H: "M2 4h5v9.5h10V4h5v24h-5v-10H7v10H2V4z",
};

export const FuturisticDay: React.FC<Props> = ({ dayName }) => {
  const letters = dayName.toUpperCase().split("");

  return (
    <div className="futuristic-day-container" aria-label={dayName}>
      {letters.map((char, index) => {
        const path = GLYPHS[char];
        if (!path) {
          return (
            <span key={index} className="futuristic-fallback-char">
              {char}
            </span>
          );
        }
        return (
          <svg
            key={index}
            viewBox="0 0 24 32"
            className="futuristic-glyph"
            aria-hidden="true"
          >
            <path d={path} fill="currentColor" />
          </svg>
        );
      })}
    </div>
  );
};
