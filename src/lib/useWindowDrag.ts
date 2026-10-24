import { useCallback, useRef } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { beginDrag, ensureOnScreen, persistPosition } from "./api";

const DRAG_THRESHOLD_PX = 4;

/**
 * Starts a native window drag once the pointer moves past a small threshold.
 * A press that never moves is treated as a click, so the widget can both be
 * dragged and clicked without a separate handle.
 */
export function useWindowDrag(onTap?: () => void, disabled = false) {
  const active = useRef(false);

  const onPointerDown = useCallback(
    (event: ReactPointerEvent<HTMLElement>) => {
      if (disabled || event.button !== 0 || active.current) return;
      if ((event.target as HTMLElement).closest("[data-interactive]")) return;

      const startX = event.screenX;
      const startY = event.screenY;
      let dragging = false;
      active.current = true;

      const cleanup = () => {
        active.current = false;
        window.removeEventListener("pointermove", onMove);
        window.removeEventListener("pointerup", onUp);
        window.removeEventListener("pointercancel", cleanup);
      };

      const onMove = (moveEvent: PointerEvent) => {
        if (dragging) return;
        const distance = Math.hypot(moveEvent.screenX - startX, moveEvent.screenY - startY);
        if (distance < DRAG_THRESHOLD_PX) return;

        dragging = true;
        cleanup();
        void beginDrag().finally(() => {
          // The backend persists while the window moves; this captures the
          // final position and guards against display changes.
          window.setTimeout(() => {
            void persistPosition();
            void ensureOnScreen();
          }, 220);
        });
      };

      const onUp = () => {
        cleanup();
        if (!dragging) onTap?.();
      };

      window.addEventListener("pointermove", onMove);
      window.addEventListener("pointerup", onUp);
      window.addEventListener("pointercancel", cleanup);
    },
    [onTap, disabled],
  );

  return { onPointerDown };
}
