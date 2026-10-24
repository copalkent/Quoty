import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement> & { size?: number };

function base({ size = 16, ...props }: IconProps) {
  return {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.5,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    "aria-hidden": true,
    ...props,
  };
}

/** Orbit mark used for the widget and the app icon. */
export function Mark({ size = 20, ...props }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" aria-hidden {...props}>
      <path
        d="M18.93 8A8 8 0 1 1 16 5.07"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
      <circle cx="18.4" cy="5.7" r="2.2" fill="currentColor" />
    </svg>
  );
}

export function RefreshIcon({ size = 15, ...props }: IconProps) {
  return (
    <svg {...base({ size, ...props })}>
      <path d="M20 11a8 8 0 1 0-.6 4" />
      <path d="M20 5v6h-6" />
    </svg>
  );
}

export function SlidersIcon({ size = 15, ...props }: IconProps) {
  return (
    <svg {...base({ size, ...props })}>
      <path d="M5 8h9M18 8h1M5 16h2M11 16h8" />
      <circle cx="16" cy="8" r="2" />
      <circle cx="9" cy="16" r="2" />
    </svg>
  );
}

export function ChevronIcon({ size = 14, ...props }: IconProps) {
  return (
    <svg {...base({ size, ...props })}>
      <path d="m6 9 6 6 6-6" />
    </svg>
  );
}

export function PinIcon({ size = 15, ...props }: IconProps) {
  return (
    <svg {...base({ size, ...props })}>
      <path d="M9 4h6l-1 6 3 3H7l3-3-1-6Z" />
      <path d="M12 13v7" />
    </svg>
  );
}

export function MinusIcon({ size = 15, ...props }: IconProps) {
  return (
    <svg {...base({ size, ...props })}>
      <path d="M5 12h14" />
    </svg>
  );
}

export function CloseIcon({ size = 14, ...props }: IconProps) {
  return (
    <svg {...base({ size, ...props })}>
      <path d="m6 6 12 12M18 6 6 18" />
    </svg>
  );
}

export function SunIcon({ size = 14, ...props }: IconProps) {
  return (
    <svg {...base({ size, ...props })}>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
    </svg>
  );
}

export function MoonIcon({ size = 14, ...props }: IconProps) {
  return (
    <svg {...base({ size, ...props })}>
      <path d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5Z" />
    </svg>
  );
}

export function MonitorIcon({ size = 14, ...props }: IconProps) {
  return (
    <svg {...base({ size, ...props })}>
      <rect x="3" y="5" width="18" height="12" rx="1.5" />
      <path d="M9 21h6M12 17v4" />
    </svg>
  );
}

export function AlertIcon({ size = 15, ...props }: IconProps) {
  return (
    <svg {...base({ size, ...props })}>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 8v4.5M12 16h.01" />
    </svg>
  );
}

export function LockIcon({ size = 14, ...props }: IconProps) {
  return (
    <svg {...base({ size, ...props })}>
      <rect x="5" y="11" width="14" height="10" rx="2" />
      <path d="M8 11V7a4 4 0 0 1 8 0v4" />
    </svg>
  );
}

export function UnlockIcon({ size = 14, ...props }: IconProps) {
  return (
    <svg {...base({ size, ...props })}>
      <rect x="5" y="11" width="14" height="10" rx="2" />
      <path d="M8 11V7a4 4 0 0 1 7.8-1.2" />
    </svg>
  );
}
