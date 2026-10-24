import { useCallback, useRef } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { persistPosition, setWindowSize } from "./api";

const MIN_WIDTH = 320;
const MAX_WIDTH = 800;
const MIN_HEIGHT = 450;
const MAX_HEIGHT = 1200;

export function useWindowResize() {
  const resizing = useRef(false);

  const onPointerDown = useCallback((e: ReactPointerEvent<HTMLElement>) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();

    resizing.current = true;
    const startX = e.screenX;
    const startY = e.screenY;
    const startWidth = window.innerWidth;
    const startHeight = window.innerHeight;

    const onPointerMove = (moveEvent: PointerEvent) => {
      if (!resizing.current) return;
      const deltaX = moveEvent.screenX - startX;
      const deltaY = moveEvent.screenY - startY;

      const nextWidth = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, startWidth + deltaX));
      const nextHeight = Math.max(MIN_HEIGHT, Math.min(MAX_HEIGHT, startHeight + deltaY));

      void setWindowSize(nextWidth, nextHeight);
    };

    const onPointerUp = () => {
      resizing.current = false;
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
      window.removeEventListener("pointercancel", onPointerUp);
      void persistPosition();
    };

    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
    window.addEventListener("pointercancel", onPointerUp);
  }, []);

  return { onPointerDown };
}
