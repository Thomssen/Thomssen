import { CloseIcon, MacMaximizeIcon, MacRestoreIcon, MaximizeIcon, MinimizeIcon, RestoreIcon } from "@/components/icons";
import { useAppearance } from "@/state/AppearanceProvider";

interface WindowControlsProps {
  maximized: boolean;
  onClose: () => void;
  onMinimize: () => void;
  onMaximize: () => void;
}

/** Window controls, in either of two styles picked in Settings > Appearance:
 * Windows' flush rectangular buttons (the default - this is a Windows-only
 * product) or macOS's circular traffic lights, for anyone who prefers the
 * look. Minimize and close reuse the same glyph either way (a flat dash and
 * an X read the same on both platforms), just resized and restyled via CSS
 * - see `.titlebar__controls--mac` in TitleBar.css. Maximize/restore does
 * NOT reuse Windows' square-outline glyph under mac styling: a Windows icon
 * revealed on hover inside an otherwise mac-styled circle is exactly what
 * read as "the normal Windows buttons showing through". */
export function WindowControls({ maximized, onClose, onMinimize, onMaximize }: WindowControlsProps) {
  const { windowControlsStyle } = useAppearance();
  const isMac = windowControlsStyle === "mac";
  const iconSize = isMac ? 7 : 10;

  const minimizeBtn = (
    <button key="minimize" type="button" className="titlebar__btn titlebar__btn--minimize" aria-label="Minimize" onClick={onMinimize}>
      <MinimizeIcon size={iconSize} />
    </button>
  );
  const maximizeBtn = (
    <button key="maximize" type="button" className="titlebar__btn titlebar__btn--maximize" aria-label={maximized ? "Restore" : "Maximize"} onClick={onMaximize}>
      {isMac ? (
        maximized ? (
          <MacRestoreIcon size={iconSize} />
        ) : (
          <MacMaximizeIcon size={iconSize} />
        )
      ) : maximized ? (
        <RestoreIcon size={iconSize} />
      ) : (
        <MaximizeIcon size={iconSize} />
      )}
    </button>
  );
  const closeBtn = (
    <button key="close" type="button" className="titlebar__btn titlebar__btn--close" aria-label="Close" onClick={onClose}>
      <CloseIcon size={iconSize} />
    </button>
  );

  // Windows convention reads minimize/maximize/close left to right. Mac's
  // traffic lights are always close/minimize/maximize (red/yellow/green) in
  // that reading order regardless of which corner they sit in - keeping
  // Windows' button order under Mac's colors is what made the last version
  // read as "wrong", not just "a different style".
  return (
    <div className={["titlebar__controls", isMac ? "titlebar__controls--mac" : ""].filter(Boolean).join(" ")}>
      {isMac ? [closeBtn, minimizeBtn, maximizeBtn] : [minimizeBtn, maximizeBtn, closeBtn]}
    </div>
  );
}
