import { useEffect, useRef, useState } from "react";
import { setWindowSize } from "../lib/api";
import type { Preferences, PreferencesPatch } from "../lib/quota";
import { useDisplayName } from "../lib/useDisplayName";
import { CloseIcon, MonitorIcon, MoonIcon, SunIcon } from "./Icons";

interface Props {
  preferences: Preferences;
  preferencesError: string | null;
  source: string | null;
  onClose: () => void;
  onUpdate: (patch: PreferencesPatch) => Promise<void>;
  onHide: () => void;
  onQuit: () => void;
}

function Switch({ on, onChange, label }: { on: boolean; onChange: () => void; label: string }) {
  return (
    <button
      type="button"
      className="switch"
      role="switch"
      aria-checked={on}
      aria-label={label}
      data-on={on}
      data-interactive
      onClick={onChange}
    >
      <span className="switch-thumb" />
    </button>
  );
}

const THEMES = [
  { value: "system" as const, label: "System", Icon: MonitorIcon },
  { value: "light" as const, label: "Light", Icon: SunIcon },
  { value: "dark" as const, label: "Dark", Icon: MoonIcon },
];

export function SettingsSheet({
  preferences,
  preferencesError,
  source,
  onClose,
  onUpdate,
  onHide,
  onQuit,
}: Props) {
  const closeRef = useRef<HTMLButtonElement>(null);
  const [displayName, setDisplayName] = useDisplayName();
  const [inputValue, setInputValue] = useState(displayName);

  useEffect(() => {
    closeRef.current?.focus();
  }, []);

  return (
    <div className="sheet" role="dialog" aria-modal="true" aria-label="Widget settings">
      <div className="sheet-head">
        <span className="sheet-title">Settings</span>
        <button
          ref={closeRef}
          type="button"
          className="icon-button"
          data-interactive
          title="Close settings"
          aria-label="Close settings"
          onClick={onClose}
        >
          <CloseIcon />
        </button>
      </div>

      <div className="sheet-body">
        <div className="field">
          <span className="field-label">Appearance</span>
          <div className="segmented" role="group" aria-label="Theme">
            {THEMES.map(({ value, label, Icon }) => (
              <button
                key={value}
                type="button"
                className={`segment${preferences.theme === value ? " is-selected" : ""}`}
                aria-pressed={preferences.theme === value}
                data-interactive
                onClick={() => void onUpdate({ theme: value })}
              >
                <Icon size={13} />
                {label}
              </button>
            ))}
          </div>
        </div>

        <div className="field">
          <div className="field-copy">
            <span className="field-label">Preferred Name</span>
            <span className="field-hint">Personalizes the greeting displayed on the widget.</span>
          </div>
          <input
            type="text"
            className="text-input"
            placeholder="Enter your name"
            value={inputValue}
            maxLength={32}
            data-interactive
            aria-label="Preferred display name"
            onChange={(e) => {
              const val = e.target.value;
              setInputValue(val);
              setDisplayName(val);
            }}
          />
        </div>

        <div className="field">
          <div className="field-copy">
            <span className="field-label">Widget Width ({preferences.width ?? 540}px)</span>
            <span className="field-hint">Adjust the horizontal span of the desktop widget.</span>
          </div>
          <div className="segmented" role="group" aria-label="Width presets">
            {[500, 540, 620, 720].map((w) => (
              <button
                key={w}
                type="button"
                className={`segment${(preferences.width ?? 540) === w ? " is-selected" : ""}`}
                aria-pressed={(preferences.width ?? 540) === w}
                data-interactive
                onClick={() => {
                  void onUpdate({ width: w });
                  void setWindowSize(w, 380);
                }}
              >
                {w}px
              </button>
            ))}
          </div>
        </div>

        <div className="field-row">
          <div className="field-copy">
            <span className="field-label">Lock Position</span>
            <span className="field-hint">Prevent moving or dragging the widget on the desktop.</span>
          </div>
          <Switch
            on={Boolean(preferences.locked)}
            label="Lock Position"
            onChange={() => void onUpdate({ locked: !preferences.locked })}
          />
        </div>

        <div className="field-row">
          <div className="field-copy">
            <span className="field-label">Always on top</span>
            <span className="field-hint">Keep the widget above other windows.</span>
          </div>
          <Switch
            on={preferences.alwaysOnTop}
            label="Always on top"
            onChange={() => void onUpdate({ alwaysOnTop: !preferences.alwaysOnTop })}
          />
        </div>

        <div className="field-row">
          <div className="field-copy">
            <span className="field-label">Start with Windows</span>
            <span className="field-hint">Open the widget when you sign in.</span>
          </div>
          <Switch
            on={preferences.startWithWindows}
            label="Start with Windows"
            onChange={() => void onUpdate({ startWithWindows: !preferences.startWithWindows })}
          />
        </div>

        <div className="source">
          <span className="field-label">Source</span>
          <span className="source-value">{source ?? "Antigravity local language server"}</span>
          <p className="source-note">
            Quotas are read on this PC from Antigravity's own language server. Session tokens stay
            in memory and are never stored, logged, or sent anywhere.
          </p>
        </div>

        {preferencesError ? <p className="field-error">{preferencesError}</p> : null}
      </div>

      <div className="sheet-actions">
        <button type="button" className="text-button" data-interactive onClick={onHide}>
          Hide widget
        </button>
        <button type="button" className="text-button" data-interactive onClick={onQuit}>
          Quit Quoty
        </button>
      </div>
    </div>
  );
}
