import { CloseIcon, MaximizeIcon, MinimizeIcon, RestoreIcon } from "@/components/icons";

interface WindowControlsProps {
  maximized: boolean;
  onClose: () => void;
  onMinimize: () => void;
  onMaximize: () => void;
}

/** Windows-style window controls: flush rectangular buttons, minimize/
 * maximize/close left to right, ending flush with the window's top-right
 * corner - this is a Windows-only product, so it should look like one
 * rather than borrowing macOS's traffic lights. */
export function WindowControls({ maximized, onClose, onMinimize, onMaximize }: WindowControlsProps) {
  return (
    <div className="titlebar__controls">
      <button type="button" className="titlebar__btn titlebar__btn--minimize" aria-label="Minimize" onClick={onMinimize}>
        <MinimizeIcon size={10} />
      </button>
      <button type="button" className="titlebar__btn titlebar__btn--maximize" aria-label={maximized ? "Restore" : "Maximize"} onClick={onMaximize}>
        {maximized ? <RestoreIcon size={10} /> : <MaximizeIcon size={10} />}
      </button>
      <button type="button" className="titlebar__btn titlebar__btn--close" aria-label="Close" onClick={onClose}>
        <CloseIcon size={10} />
      </button>
    </div>
  );
}
