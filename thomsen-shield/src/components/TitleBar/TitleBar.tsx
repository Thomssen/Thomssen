import { useEffect, useState } from "react";
import { APP } from "@/config/app";
import { ROUTE_LABELS } from "@/config/navigation";
import { useNavigation } from "@/state/NavigationProvider";
import { WindowControls } from "./WindowControls";
import { closeWindow, isWindowMaximized, minimizeWindow, onMaximizeChange, toggleMaximizeWindow } from "@/services/window";
import "./TitleBar.css";

interface TitleBarProps {
  /** Suppresses the "/ Route" segment - used during setup/license/loading
   * screens where there is no current route yet. */
  showRoute?: boolean;
}

/** Custom frameless title bar: the whole bar is a drag region; the window
 * controls opt out. Same mechanism as every other Thomsen app. */
export function TitleBar({ showRoute = true }: TitleBarProps) {
  const { route } = useNavigation();
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    void isWindowMaximized().then(setMaximized);
    const unsub = onMaximizeChange(setMaximized);
    return () => {
      void unsub.then((fn) => fn());
    };
  }, []);

  return (
    <div className="titlebar" data-tauri-drag-region onDoubleClick={toggleMaximizeWindow}>
      <div className="titlebar__label" data-tauri-drag-region>
        {APP.name}
        {showRoute && (
          <>
            <span className="titlebar__sep">/</span>
            <span className="titlebar__route">{ROUTE_LABELS[route]}</span>
          </>
        )}
      </div>

      <WindowControls maximized={maximized} onClose={closeWindow} onMinimize={minimizeWindow} onMaximize={toggleMaximizeWindow} />
    </div>
  );
}
