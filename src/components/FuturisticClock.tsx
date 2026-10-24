import React, { useEffect, useMemo, useState } from "react";
import { FuturisticDay } from "./FuturisticDay";

interface Props {
  onPointerDown?: (e: React.PointerEvent<HTMLElement>) => void;
}

export const FuturisticClock: React.FC<Props> = ({ onPointerDown }) => {
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), 1000);
    return () => window.clearInterval(id);
  }, []);

  const weekday = useMemo(() => {
    try {
      return now.toLocaleDateString("en-US", { weekday: "long" }).toUpperCase();
    } catch {
      return "FRIDAY";
    }
  }, [now]);

  // Formats as: "14 SEPTEMBER, 2018."
  const dateFormatted = useMemo(() => {
    try {
      const day = now.getDate();
      const month = now.toLocaleDateString("en-US", { month: "long" }).toUpperCase();
      const year = now.getFullYear();
      return `${day} ${month}, ${year}.`;
    } catch {
      return "";
    }
  }, [now]);

  // Formats as: "- 11:30 AM -"
  const timeFormatted = useMemo(() => {
    try {
      const timeStr = now.toLocaleTimeString("en-US", {
        hour: "numeric",
        minute: "2-digit",
        hour12: true,
      });
      return `- ${timeStr} -`;
    } catch {
      return "- 12:00 PM -";
    }
  }, [now]);

  return (
    <div
      className="rainmeter-clock-block"
      onPointerDown={onPointerDown}
      role="region"
      aria-label="Clock and Date"
    >
      {/* 1. Large Stylized Weekday */}
      <FuturisticDay dayName={weekday} />

      {/* 2. Complete Date Beneath: "14 SEPTEMBER, 2018." */}
      <div className="rainmeter-date-line">{dateFormatted}</div>

      {/* 3. Centered Time with Dashes: "- 11:30 AM -" */}
      <div className="rainmeter-time-line">{timeFormatted}</div>
    </div>
  );
};
