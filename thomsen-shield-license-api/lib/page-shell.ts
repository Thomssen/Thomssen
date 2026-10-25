/**
 * Shared HTML shell for server-rendered pages (currently just the
 * purchase success page). Mirrors the design tokens in
 * `thomsen-shield/src/styles/tokens.css` - same black/white/grey palette,
 * radii, and type - so a customer landing here right after paying sees
 * the same visual language as the site and the app they just bought.
 */
export function pageShell(title: string, bodyHtml: string): string {
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<meta name="robots" content="noindex" />
<title>${escapeHtml(title)}</title>
<style>
  :root {
    --bg: #050607;
    --surface: #0d0f11;
    --surface-2: #131619;
    --text: #f2f3f4;
    --text-secondary: #b4b9be;
    --text-muted: #7c838a;
    --text-faint: #565c62;
    --border: rgba(255,255,255,0.08);
    --border-strong: rgba(255,255,255,0.14);
    --btn-primary-bg: #f3f4f5;
    --btn-primary-bg-hover: #ffffff;
    --btn-primary-text: #0a0b0c;
    --good: #6f9e8f;
    --good-soft: rgba(111,158,143,0.14);
    --radius: 16px;
    --radius-sm: 10px;
    --radius-pill: 999px;
  }
  * { box-sizing: border-box; }
  html, body { margin: 0; height: 100%; }
  body {
    background: var(--bg);
    color: var(--text-secondary);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Inter, Roboto, Helvetica, Arial, sans-serif;
    font-size: 15px;
    line-height: 1.6;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 32px 20px;
  }
  .panel { width: 100%; max-width: 420px; display: flex; flex-direction: column; align-items: center; text-align: center; gap: 8px; }
  .logo { display: grid; place-items: center; width: 52px; height: 52px; border-radius: var(--radius); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); font-size: 18px; font-weight: 700; margin-bottom: 10px; }
  .eyebrow { font-size: 11px; font-weight: 600; letter-spacing: 0.14em; text-transform: uppercase; color: var(--text-muted); }
  h1 { margin: 4px 0 0; font-size: 24px; font-weight: 600; letter-spacing: -0.02em; color: var(--text); }
  p.lead { margin: 10px 0 0; max-width: 40ch; color: var(--text-muted); font-size: 14px; }
  .card { width: 100%; margin-top: 24px; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 22px; text-align: left; box-shadow: 0 1px 2px rgba(0,0,0,0.4); }
  .key-label { font-size: 11px; font-weight: 600; letter-spacing: 0.06em; text-transform: uppercase; color: var(--text-muted); }
  .key { display: block; margin-top: 8px; font-family: ui-monospace, "Cascadia Mono", Consolas, monospace; font-size: 20px; font-weight: 600; color: var(--text); letter-spacing: 0.02em; word-break: break-all; user-select: all; }
  .rule { height: 1px; background: var(--border); border: 0; margin: 18px 0; }
  .meta-row { display: flex; align-items: center; justify-content: space-between; font-size: 12.5px; color: var(--text-secondary); padding: 4px 0; }
  .actions { width: 100%; display: flex; flex-direction: column; gap: 10px; margin-top: 22px; }
  .btn { display: inline-flex; align-items: center; justify-content: center; height: 46px; padding-inline: 22px; border-radius: var(--radius-pill); font-size: 14px; font-weight: 600; text-decoration: none; border: 1px solid transparent; cursor: pointer; }
  .btn-primary { background: var(--btn-primary-bg); color: var(--btn-primary-text); }
  .btn-primary:hover { background: var(--btn-primary-bg-hover); }
  .btn-secondary { background: rgba(255,255,255,0.05); color: var(--text); border-color: var(--border); }
  .btn-secondary:hover { border-color: var(--border-strong); }
  .hint { margin-top: 18px; font-size: 12px; color: var(--text-faint); }
  .hint a { color: var(--text-faint); text-decoration: underline; }
  .hint a:hover { color: var(--text-muted); }
  .badge { display: inline-flex; align-items: center; gap: 6px; height: 22px; padding-inline: 9px; border-radius: var(--radius-pill); background: var(--good-soft); color: var(--good); font-size: 11px; font-weight: 600; }
  .badge::before { content: ""; width: 6px; height: 6px; border-radius: 50%; background: currentColor; }
  .spinner { width: 28px; height: 28px; border-radius: 50%; border: 2.5px solid var(--border-strong); border-top-color: var(--text-muted); animation: spin 0.8s linear infinite; margin: 8px 0; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
</head>
<body>
${bodyHtml}
</body>
</html>`;
}

export function escapeHtml(value: string): string {
  return value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#39;");
}
