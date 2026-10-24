import React, { useEffect, useMemo, useState } from "react";

interface ClockHeaderProps {
  onPointerDown?: ((e: React.PointerEvent<HTMLElement>) => void) | undefined;
}

export const ClockHeader: React.FC<ClockHeaderProps> = ({ onPointerDown }) => {
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    const timerId = window.setInterval(() => {
      setNow(new Date());
    }, 1000);
    return () => window.clearInterval(timerId);
  }, []);

  const { timeString, secondsString, periodString } = useMemo(() => {
    try {
      const formatter = new Intl.DateTimeFormat(undefined, {
        hour: "numeric",
        minute: "2-digit",
        second: "2-digit",
        hour12: true,
      });
      const parts = formatter.formatToParts(now);
      const hour = parts.find((p) => p.type === "hour")?.value ?? "";
      const minute = parts.find((p) => p.type === "minute")?.value ?? "";
      const second = parts.find((p) => p.type === "second")?.value ?? "";
      const period = parts.find((p) => p.type === "dayPeriod")?.value ?? "";

      return {
        timeString: `${hour}:${minute}`,
        secondsString: `:${second}`,
        periodString: period.toUpperCase(),
      };
    } catch {
      return {
        timeString: "12:00",
        secondsString: ":00",
        periodString: "PM",
      };
    }
  }, [now]);

  const weekday = useMemo(() => {
    try {
      return now.toLocaleDateString(undefined, { weekday: "long" }).toUpperCase();
    } catch {
      return "SATURDAY";
    }
  }, [now]);

  const fullDate = useMemo(() => {
    try {
      return now.toLocaleDateString(undefined, {
        month: "long",
        day: "numeric",
        year: "numeric",
      });
    } catch {
      return "October 24, 2026";
    }
  }, [now]);

  return (
    <header
      className="rainmeter-clock-section rainmeter-clock-section--horizontal"
      onPointerDown={onPointerDown}
      role="region"
      aria-label="Clock and Date"
    >
      {/* Left side: Huge bold weekday (e.g. SATURDAY) */}
      <div className="rainmeter-clock-weekday-huge">{weekday}</div>

      {/* Right side of the Saturday text: Stacked Time and Date */}
      <div className="rainmeter-clock-right-col">
        <div className="rainmeter-clock-time-row">
          <span className="rainmeter-clock-time-digits">{timeString}</span>
          <span className="rainmeter-clock-time-seconds">{secondsString}</span>
          {periodString ? (
            <span className="rainmeter-clock-time-period">{periodString}</span>
          ) : null}
        </div>
        <div className="rainmeter-clock-date">{fullDate}</div>
      </div>
    </header>
  );
};
