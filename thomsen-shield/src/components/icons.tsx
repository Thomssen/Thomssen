/**
 * The app's icon set: window controls, sidebar/nav-adjacent glyphs, and
 * network/security icons. All 1em, currentColor, 1.6 stroke, 24x24 - same
 * visual language across every Thomsen app. Never emoji.
 */

type P = { className?: string; size?: number };

const svg = (size: number, children: React.ReactNode, extra?: string) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth={1.6}
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
    className={extra}
  >
    {children}
  </svg>
);

// -- Window controls --------------------------------------------------------
export const MinimizeIcon = ({ size = 12 }: P) => svg(size, <path d="M5 12h14" />);
export const MaximizeIcon = ({ size = 12 }: P) => svg(size, <rect x="5" y="5" width="14" height="14" rx="1.5" />);
export const RestoreIcon = ({ size = 12 }: P) =>
  svg(
    size,
    <>
      <rect x="7.5" y="7.5" width="11" height="11" rx="1.5" />
      <path d="M5.5 14.5V6a1.5 1.5 0 0 1 1.5-1.5h8.5" />
    </>,
  );
export const CloseIcon = ({ size = 12 }: P) => svg(size, <path d="M6 6l12 12M18 6L6 18" />);

// -- General -----------------------------------------------------------------
export const ChevronDown = ({ className, size = 16 }: P) => svg(size, <path d="M6 9l6 6 6-6" />, className);
export const ChevronRight = ({ className, size = 16 }: P) => svg(size, <path d="M9 6l6 6-6 6" />, className);
export const PlusIcon = ({ className, size = 16 }: P) => svg(size, <path d="M12 5v14M5 12h14" />, className);
export const CheckIcon = ({ className, size = 16 }: P) => svg(size, <path d="M20 6L9 17l-5-5" />, className);
export const XIcon = ({ className, size = 16 }: P) => svg(size, <path d="M6 6l12 12M18 6L6 18" />, className);
export const CopyIcon = ({ className, size = 15 }: P) =>
  svg(
    size,
    <>
      <rect x="9" y="9" width="12" height="12" rx="2" />
      <path d="M6 15H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v1" />
    </>,
    className,
  );
export const TrashIcon = ({ className, size = 15 }: P) =>
  svg(
    size,
    <>
      <path d="M4 7h16" />
      <path d="M10 11v6M14 11v6" />
      <path d="M6 7l1 12a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1l1-12" />
      <path d="M9 7V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v3" />
    </>,
    className,
  );
export const SearchIcon = ({ className, size = 15 }: P) =>
  svg(
    size,
    <>
      <circle cx="11" cy="11" r="7" />
      <path d="M20 20l-3.5-3.5" />
    </>,
    className,
  );
export const DotsIcon = ({ className, size = 16 }: P) =>
  svg(
    size,
    <>
      <circle cx="5" cy="12" r="1" />
      <circle cx="12" cy="12" r="1" />
      <circle cx="19" cy="12" r="1" />
    </>,
    className,
  );
export const AlertTriangleIcon = ({ className, size = 16 }: P) =>
  svg(
    size,
    <>
      <path d="M12 3.5 21.5 20h-19L12 3.5Z" />
      <path d="M12 10v4" />
      <circle cx="12" cy="17.2" r="0.9" fill="currentColor" stroke="none" />
    </>,
    className,
  );
export const InfoIcon = ({ className, size = 16 }: P) =>
  svg(
    size,
    <>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 11v5.5" />
      <circle cx="12" cy="7.8" r="0.9" fill="currentColor" stroke="none" />
    </>,
    className,
  );
export const RefreshIcon = ({ className, size = 15 }: P) =>
  svg(
    size,
    <>
      <path d="M20 11a8 8 0 1 0-2.6 6.9" />
      <path d="M20 5v6h-6" />
    </>,
    className,
  );
export const DownloadIcon = ({ className, size = 15 }: P) =>
  svg(
    size,
    <>
      <path d="M12 4v11" />
      <path d="M7.5 11.5 12 16l4.5-4.5" />
      <path d="M5 17v2a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-2" />
    </>,
    className,
  );

// -- Network / security ---------------------------------------------------------
export const ShieldIcon = ({ className, size = 16 }: P) =>
  svg(size, <path d="M12 3.5 19 6.3v5.4c0 4.4-2.9 7.7-7 8.8-4.1-1.1-7-4.4-7-8.8V6.3L12 3.5Z" />, className);
export const ShieldCheckIcon = ({ className, size = 16 }: P) =>
  svg(
    size,
    <>
      <path d="M12 3.5 19 6.3v5.4c0 4.4-2.9 7.7-7 8.8-4.1-1.1-7-4.4-7-8.8V6.3L12 3.5Z" />
      <path d="M9 12.2l2 2 4-4.2" />
    </>,
    className,
  );
export const NetworkIcon = ({ className, size = 16 }: P) =>
  svg(
    size,
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M3.5 12h17" />
      <path d="M12 3.5c2.4 2.3 3.7 5.2 3.7 8.5s-1.3 6.2-3.7 8.5c-2.4-2.3-3.7-5.2-3.7-8.5S9.6 5.8 12 3.5Z" />
    </>,
    className,
  );
export const BanIcon = ({ className, size = 15 }: P) =>
  svg(
    size,
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M6.2 6.2l11.6 11.6" />
    </>,
    className,
  );
export const LockIcon = ({ className, size = 16 }: P) =>
  svg(
    size,
    <>
      <rect x="5" y="11" width="14" height="9" rx="2" />
      <path d="M8 11V7a4 4 0 0 1 8 0v4" />
    </>,
    className,
  );
export const UnlockIcon = ({ className, size = 16 }: P) =>
  svg(
    size,
    <>
      <rect x="5" y="11" width="14" height="9" rx="2" />
      <path d="M8 11V7a4 4 0 0 1 7.5-2" />
    </>,
    className,
  );
export const AppWindowIcon = ({ className, size = 16 }: P) =>
  svg(
    size,
    <>
      <rect x="3.5" y="5" width="17" height="14" rx="2" />
      <path d="M3.5 9h17" />
      <circle cx="6.3" cy="7" r="0.6" fill="currentColor" stroke="none" />
      <circle cx="8.3" cy="7" r="0.6" fill="currentColor" stroke="none" />
    </>,
    className,
  );
export const GlobeIcon = ({ className, size = 15 }: P) =>
  svg(
    size,
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M3.5 12h17M12 3.5c2.4 2.3 3.7 5.2 3.7 8.5s-1.3 6.2-3.7 8.5c-2.4-2.3-3.7-5.2-3.7-8.5S9.6 5.8 12 3.5Z" />
    </>,
    className,
  );
export const ActivityIcon = ({ className, size = 16 }: P) => svg(size, <path d="M3.5 12h4l2.2-7 4.6 14 2.2-7H20.5" />, className);
export const KeyIcon = ({ className, size = 16 }: P) =>
  svg(
    size,
    <>
      <circle cx="8" cy="15.5" r="3.6" />
      <path d="M10.3 13.2 18 5.5" />
      <path d="M15 8.5l2 2M17.5 6l2 2" />
    </>,
    className,
  );
export const SettingsIcon = ({ className, size = 16 }: P) =>
  svg(
    size,
    <>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 13.5a1.7 1.7 0 0 0 .3 1.9l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.5V19.5a2 2 0 1 1-4 0v-.2a1.7 1.7 0 0 0-1-1.5 1.7 1.7 0 0 0-1.9.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.9 1.7 1.7 0 0 0-1.5-1H4.5a2 2 0 1 1 0-4h.2a1.7 1.7 0 0 0 1.5-1 1.7 1.7 0 0 0-.3-1.9l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.9.3H10.5a1.7 1.7 0 0 0 1-1.5V4.5a2 2 0 1 1 4 0v.2a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.9-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.9V10.5a1.7 1.7 0 0 0 1.5 1h.2a2 2 0 1 1 0 4h-.2a1.7 1.7 0 0 0-1.5 1Z" />
    </>,
    className,
  );

export type IconComponent = (props: P) => React.ReactElement;
