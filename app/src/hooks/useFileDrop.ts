import { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

import { insideTauri } from "../api";

/**
 * Listens for files dragged from Explorer onto the window.
 *
 * Tauri intercepts OS drag & drop (HTML5 drop events do not expose real file
 * paths), so we subscribe to the webview's native drag-drop event.
 *
 * @param onDrop called with the absolute paths of the dropped files
 * @returns `true` while files are being dragged over the window
 */
export function useFileDrop(onDrop: (paths: string[]) => void): boolean {
  const [hovering, setHovering] = useState(false);

  // Keep the latest callback without re-subscribing on every render.
  const onDropRef = useRef(onDrop);
  onDropRef.current = onDrop;

  useEffect(() => {
    if (!insideTauri) return;

    let unlisten: (() => void) | undefined;
    let disposed = false;

    getCurrentWebview()
      .onDragDropEvent(({ payload }) => {
        switch (payload.type) {
          case "enter":
          case "over":
            setHovering(true);
            break;
          case "leave":
            setHovering(false);
            break;
          case "drop":
            setHovering(false);
            if (payload.paths.length > 0) onDropRef.current(payload.paths);
            break;
        }
      })
      .then((stop) => {
        // The component may have unmounted before the listener was ready.
        if (disposed) stop();
        else unlisten = stop;
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  return hovering;
}
