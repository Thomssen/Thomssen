import { CloseIcon, MaximizeIcon, MinimizeIcon, RestoreIcon } from "@/components/icons";

interface WindowControlsProps {
  maximized: boolean;
  onClose: () => void;
  onMinimize: () => void;
  onMaximize: () => void;
}

/** macOS-style traffic-light window controls, kept on the right (Windows
 * convention) rather than macOS's native left placement - same mechanism as
 * every other Thomsen app's frameless title bar. */
export function WindowControls({ maximized, onClose, onMinimize, onMaximize }: WindowControlsProps) {
  return (
    <div className="titlebar__controls">
      <button type="button" className="titlebar__btn titlebar__btn--minimize" aria-label="Minimize" onClick={onMinimize}>
        <MinimizeIcon size={7} />
      </button>
      <button type="button" className="titlebar__btn titlebar__btn--maximize" aria-label={maximized ? "Restore" : "Maximize"} onClick={onMaximize}>
        {maximized ? <RestoreIcon size={7} /> : <MaximizeIcon size={7} />}
      </button>
      <button type="button" className="titlebar__btn titlebar__btn--close" aria-label="Close" onClick={onClose}>
        <CloseIcon size={7} />
      </button>
    </div>
  );
}
