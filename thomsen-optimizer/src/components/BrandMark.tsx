/** The Thomsen Software crescent mark, inline as SVG so it stays crisp at
 * any size and inherits `color` from its container - the same spot a plain
 * "S" character used to sit in (Settings > About, the sign-in screen, the
 * onboarding welcome screen). Geometry mirrors src-tauri/icons/ (the actual
 * app/taskbar/installer icon and the Stripe checkout brand image), so this
 * is the same mark the customer sees everywhere else, not a different one. */
export function BrandMark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 1024 1024" width="100%" height="100%" className={className} aria-hidden="true">
      <mask id="brandmark-cutout">
        <rect width="1024" height="1024" fill="white" />
        <circle cx="653" cy="394" r="189" fill="black" />
        <circle cx="653" cy="394" r="63" fill="white" />
      </mask>
      <circle cx="540" cy="507" r="277" fill="currentColor" mask="url(#brandmark-cutout)" />
    </svg>
  );
}
