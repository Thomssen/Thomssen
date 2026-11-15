//! Rendering. Original layout: centered start screen, then a console view.
use crate::app::{App, AuthState};
use crate::commands;
use crate::config::VERSION;
use crate::output::{OutLine, Status};
use crate::theme;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

const MARK: [&str; 5] = [
    "     ▄▄▄██▄▄▄     ",
    "   ▄██▀▀  ▀▀██▄   ",
    "  ██▀   ▄▄   ▀██  ",
    "   ▀██▄▄▀▀▄▄██▀   ",
    "     ▀▀▀██▀▀▀     ",
];
const SPIN: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(Block::default().style(theme::base()), area);
    if area.width < 40 || area.height < 10 {
        let p = Paragraph::new("Terminal too small.\nResize to at least 40x10.").style(theme::dim()).alignment(Alignment::Center);
        f.render_widget(p, area);
        return;
    }
    if app.lines.is_empty() && app.busy.is_none() {
        draw_start(f, app, area);
    } else {
        draw_console(f, app, area);
    }
}

fn operator_status(app: &App) -> (String, Style) {
    match &app.auth {
        AuthState::Authorized(i) => (format!("authenticated  ·  {}", i.user.display_name.clone().unwrap_or(i.user.username.clone())), theme::ok()),
        AuthState::Checking => ("verifying session…".into(), theme::dim()),
        AuthState::Waiting(_) => ("waiting for Discord authorization".into(), theme::accent()),
        AuthState::Locked(m) => (format!("not authenticated  ·  {m}"), theme::err()),
    }
}

fn kv_line(k: &str, v: String, st: Style, w: usize) -> Line<'static> {
    let pad = " ".repeat(14usize.saturating_sub(k.width()));
    let mut l = Line::from(vec![Span::styled(format!("{k}{pad}"), theme::dim()), Span::styled(v, st)]);
    l = l.alignment(Alignment::Left);
    let _ = w;
    l
}

fn draw_start(f: &mut Frame, app: &App, area: Rect) {
    let col_w = area.width.min(64);
    let x = area.x + (area.width - col_w) / 2;
    let tall = area.height >= 26;
    let (op, op_style) = operator_status(app);
    let access = if app.authorized() { ("authorized".to_string(), theme::ok()) } else { ("restricted".to_string(), theme::dim()) };
    let case = app.active_case.as_ref().map(|c| format!("#{}  {}", c.0, c.1)).unwrap_or("none".into());

    let mut lines: Vec<Line> = vec![];
    if tall {
        for g in MARK {
            lines.push(Line::from(Span::styled(g, theme::accent())).alignment(Alignment::Center));
        }
        lines.push(Line::default());
    }
    lines.push(Line::from(Span::styled("P S Y C H O S E C", theme::bold())).alignment(Alignment::Center));
    lines.push(Line::default());
    lines.push(Line::from(Span::styled("PRIVATE OSINT & INVESTIGATION TERMINAL", theme::dim())).alignment(Alignment::Center));
    lines.push(Line::from(Span::styled(format!("v{VERSION}"), theme::dim())).alignment(Alignment::Center));
    lines.push(Line::default());
    let pad_l = (col_w as usize).saturating_sub(40) / 2;
    let ind = " ".repeat(pad_l.min(12));
    for (k, v, st) in [
        ("Operator", op, op_style),
        ("Access", access.0, access.1),
        ("Modules", format!("{} loaded", crate::osint::MODULES.len()), theme::base()),
        ("Active Case", case, theme::base()),
    ] {
        let mut l = kv_line(k, v, st, col_w as usize);
        l.spans.insert(0, Span::raw(ind.clone()));
        lines.push(l);
    }
    lines.push(Line::default());
    lines.push(
        Line::from(vec![
            Span::styled("/ ", theme::accent()), Span::styled("help   ", theme::dim()),
            Span::styled("/ ", theme::accent()), Span::styled("modules   ", theme::dim()),
            Span::styled("/ ", theme::accent()), Span::styled("case list   ", theme::dim()),
            Span::styled("/ ", theme::accent()), Span::styled("settings", theme::dim()),
        ])
        .alignment(Alignment::Center),
    );
    if let AuthState::Locked(_) = app.auth {
        lines.push(Line::from(Span::styled("type 'login' to sign in with Discord", theme::warn())).alignment(Alignment::Center));
    }

    let body_h = lines.len() as u16;
    let total = body_h + 4; // + gap + input box(3)
    let y = area.y + area.height.saturating_sub(total + 1) / 2;
    f.render_widget(Paragraph::new(lines).style(theme::base()), Rect { x, y, width: col_w, height: body_h.min(area.height) });
    let input_area = Rect { x, y: y + body_h + 1, width: col_w, height: 3.min(area.height.saturating_sub(y + body_h + 1)) };
    draw_input(f, app, input_area, true);
    if let Some(h) = commands::hint(&app.input) {
        let hr = Rect { x, y: input_area.y + 3, width: col_w, height: 1 };
        if hr.y < area.y + area.height {
            f.render_widget(Paragraph::new(Span::styled(h, theme::dim())), hr);
        }
    }
    draw_footer(f, app, Rect { x: area.x, y: area.y + area.height - 1, width: area.width, height: 1 });
}

fn draw_input(f: &mut Frame, app: &App, area: Rect, boxed: bool) {
    let prompt = if app.confirm_prompt() { "confirm > " } else { "psychosec > " };
    let mut spans = vec![Span::styled(prompt, theme::accent())];
    spans.extend(highlight(&app.input));
    let (rect, text_x) = if boxed {
        let blk = Block::default().borders(Borders::ALL).border_style(theme::border()).style(theme::base());
        let inner = blk.inner(area);
        f.render_widget(blk, area);
        (inner, inner.x)
    } else {
        (area, area.x)
    };
    f.render_widget(Paragraph::new(Line::from(spans)).style(theme::base()), rect);
    let before: String = app.input.chars().take(app.cursor).collect();
    let cx = text_x + prompt.width() as u16 + before.width() as u16;
    if cx < rect.x + rect.width {
        f.set_cursor_position(Position::new(cx, rect.y));
    }
}

pub fn highlight(input: &str) -> Vec<Span<'static>> {
    if input.is_empty() {
        return vec![];
    }
    let mut spans = vec![];
    let (cmd, rest) = match commands::command_prefix_len(input) {
        Some(n) => (Some(&input[..n]), &input[n..]),
        None => (None, input),
    };
    match cmd {
        Some(c) => spans.push(Span::styled(c.to_string(), theme::accent().add_modifier(Modifier::BOLD))),
        None => {
            let first = input.split_whitespace().next().unwrap_or("");
            let known = commands::COMPLETIONS.iter().any(|c| c.starts_with(&input.to_ascii_lowercase()));
            let _ = first;
            if known {
                return vec![Span::styled(input.to_string(), theme::base())];
            }
            return vec![Span::styled(input.to_string(), theme::err())];
        }
    }
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let flush = |cur: &mut String, spans: &mut Vec<Span<'static>>, quoted: bool| {
        if cur.is_empty() { return; }
        let st = if quoted { theme::ok() } else if cur.trim_start().starts_with('-') { theme::warn() } else { theme::base() };
        spans.push(Span::styled(std::mem::take(cur), st));
    };
    for c in rest.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => { cur.push(c); flush(&mut cur, &mut spans, true); quote = None; }
            (Some(_), c) => cur.push(c),
            (None, '"') | (None, '\'') => { flush(&mut cur, &mut spans, false); cur.push(c); quote = Some(c); }
            (None, c) if c.is_whitespace() => { flush(&mut cur, &mut spans, false); spans.push(Span::raw(c.to_string())); }
            (None, c) => cur.push(c),
        }
    }
    flush(&mut cur, &mut spans, quote.is_some());
    spans
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let left = format!("PSYCHOSEC v{VERSION}");
    let right = if app.busy.is_some() { "ctrl+c cancel" } else { "tab complete · ↑↓ history · pgup/pgdn scroll · ctrl+c quit" };
    let w = area.width as usize;
    let gap = w.saturating_sub(left.width() + right.width() + 2);
    let line = if gap > 0 {
        Line::from(vec![Span::styled(left, theme::dim()), Span::raw(" ".repeat(gap + 1)), Span::styled(right, theme::dim())])
    } else {
        Line::from(Span::styled(left, theme::dim()))
    };
    f.render_widget(Paragraph::new(line).style(theme::base()), area);
}

fn draw_console(f: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(3), Constraint::Length(1), Constraint::Length(2), Constraint::Length(1)])
        .split(area);
    // header
    let (op, st) = operator_status(app);
    let case = app.active_case.as_ref().map(|c| format!("case #{} {}", c.0, c.1)).unwrap_or("no case".into());
    let hdr = Line::from(vec![
        Span::styled(" PSYCHOSEC", theme::accent().add_modifier(Modifier::BOLD)),
        Span::styled("  ·  ", theme::border()),
        Span::styled(op, st),
        Span::styled("  ·  ", theme::border()),
        Span::styled(case, theme::dim()),
    ]);
    f.render_widget(Paragraph::new(hdr).style(theme::base()), rows[0]);

    // output
    let out = rows[1];
    let width = out.width.saturating_sub(2) as usize;
    let all = render_lines(&app.lines, width.max(20));
    let h = out.height as usize;
    let max_scroll = all.len().saturating_sub(h);
    app.max_scroll.set(max_scroll);
    let scroll = app.scroll.min(max_scroll);
    let end = all.len() - scroll;
    let start = end.saturating_sub(h);
    let view: Vec<Line> = all[start..end].to_vec();
    f.render_widget(Paragraph::new(view).style(theme::base()), Rect { x: out.x + 1, y: out.y, width: out.width.saturating_sub(1), height: out.height });
    if scroll > 0 {
        let tag = format!(" ↑ {} more lines (pgdn) ", scroll);
        let w = tag.width() as u16;
        f.render_widget(Paragraph::new(Span::styled(tag, theme::accent())), Rect { x: out.x + out.width.saturating_sub(w + 1), y: out.y, width: w.min(out.width), height: 1 });
    }

    // status / hint line
    let status = if let Some(b) = &app.busy {
        Line::from(vec![Span::styled(format!(" {} ", SPIN[(app.tick as usize / 2) % SPIN.len()]), theme::accent()), Span::styled(format!("{}…", b.label), theme::dim())])
    } else if let Some(h) = commands::hint(&app.input) {
        Line::from(Span::styled(format!(" {h}"), theme::dim()))
    } else {
        Line::default()
    };
    f.render_widget(Paragraph::new(status).style(theme::base()), rows[2]);

    // input
    let blk = Block::default().borders(Borders::TOP).border_style(theme::border()).style(theme::base());
    let inner = blk.inner(rows[3]);
    f.render_widget(blk, rows[3]);
    draw_input(f, app, Rect { x: inner.x + 1, width: inner.width.saturating_sub(1), ..inner }, false);
    draw_footer(f, app, rows[4]);
}

fn glyph(s: Status) -> Span<'static> {
    match s {
        Status::Ok => Span::styled("● ", theme::ok()),
        Status::Warn => Span::styled("● ", theme::warn()),
        Status::Err => Span::styled("● ", theme::err()),
        Status::Info => Span::styled("· ", theme::accent()),
        Status::Dim => Span::styled("· ", theme::dim()),
    }
}

fn wrap(s: &str, w: usize) -> Vec<String> {
    textwrap::wrap(s, textwrap::Options::new(w.max(8)).break_words(true)).into_iter().map(|c| c.into_owned()).collect::<Vec<_>>()
}

const KEYW: usize = 20;

pub fn render_lines(lines: &[OutLine], width: usize) -> Vec<Line<'static>> {
    let mut v: Vec<Line<'static>> = vec![];
    for l in lines {
        match l {
            OutLine::Blank => v.push(Line::default()),
            OutLine::Cmd(c) => {
                let mut spans = vec![Span::styled("psychosec > ", theme::dim())];
                spans.extend(highlight(c));
                v.push(Line::from(spans));
            }
            OutLine::Title(t) => v.push(Line::from(Span::styled(t.clone(), theme::accent().add_modifier(Modifier::BOLD)))),
            OutLine::Heading(t) => v.push(Line::from(vec![Span::styled("── ", theme::border()), Span::styled(t.clone(), theme::dim().add_modifier(Modifier::BOLD))])),
            OutLine::Text(t) => {
                for w in wrap(t, width.saturating_sub(2)) {
                    v.push(Line::from(vec![Span::raw("  "), Span::styled(w, theme::base())]));
                }
            }
            OutLine::Kv(k, val) => {
                let kw = k.width().min(KEYW);
                let ks = if k.width() > KEYW { format!("{}…", k.chars().take(KEYW - 1).collect::<String>()) } else { k.clone() };
                let pad = " ".repeat(KEYW + 2 - ks.width().min(KEYW));
                let vw = width.saturating_sub(KEYW + 4).max(10);
                let _ = kw;
                for (i, w) in wrap(val, vw).into_iter().enumerate() {
                    if i == 0 {
                        v.push(Line::from(vec![Span::raw("  "), Span::styled(format!("{ks}{pad}"), theme::dim()), Span::styled(w, theme::base())]));
                    } else {
                        v.push(Line::from(vec![Span::raw(" ".repeat(KEYW + 4)), Span::styled(w, theme::base())]));
                    }
                }
                if val.is_empty() {
                    v.push(Line::from(vec![Span::raw("  "), Span::styled(ks, theme::dim())]));
                }
            }
            OutLine::Item(s, label, detail) => {
                let lw = if label.is_empty() { 0 } else { KEYW };
                let ls = if label.width() > lw && lw > 0 { format!("{}…", label.chars().take(lw - 1).collect::<String>()) } else { label.clone() };
                let pad = " ".repeat((lw + 1).saturating_sub(ls.width()));
                let dw = width.saturating_sub(lw + 5).max(10);
                let dstyle = if *s == Status::Dim { theme::dim() } else { theme::base() };
                for (i, w) in wrap(detail, dw).into_iter().enumerate() {
                    if i == 0 {
                        v.push(Line::from(vec![Span::raw(" "), glyph(*s), Span::styled(format!("{ls}{pad}"), theme::dim()), Span::styled(w, dstyle)]));
                    } else {
                        v.push(Line::from(vec![Span::raw(" ".repeat(lw + 4)), Span::styled(w, dstyle)]));
                    }
                }
            }
            OutLine::Notice(s, t) => {
                let st = match s { Status::Err => theme::err(), Status::Ok => theme::ok(), Status::Warn => theme::warn(), Status::Dim => theme::dim(), Status::Info => theme::base() };
                for (i, w) in wrap(t, width.saturating_sub(4)).into_iter().enumerate() {
                    if i == 0 {
                        v.push(Line::from(vec![Span::raw(" "), glyph(*s), Span::styled(w, st)]));
                    } else {
                        v.push(Line::from(vec![Span::raw("   "), Span::styled(w, st)]));
                    }
                }
            }
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn dump(term: &Terminal<TestBackend>) -> String {
        let b = term.backend().buffer();
        (0..b.area.height)
            .map(|y| (0..b.area.width).map(|x| b[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn renders_start_and_console() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _g = rt.enter();
        let mut app = App::new(rt.handle().clone()).unwrap();
        let mut term = Terminal::new(TestBackend::new(90, 30)).unwrap();
        term.draw(|f| draw(f, &app)).unwrap();
        let s = dump(&term);
        println!("{s}");
        assert!(s.contains("P S Y C H O S E C") && s.contains("PRIVATE OSINT & INVESTIGATION TERMINAL") && s.contains("Active Case") && s.contains("psychosec >"));
        app.push(crate::output::OutLine::Notice(Status::Ok, "hello world".into()));
        for _ in 0..3 { app.push(crate::output::OutLine::Kv("Key".into(), "value ".repeat(40))); }
        term.draw(|f| draw(f, &app)).unwrap();
        let s = dump(&term);
        println!("{s}");
        assert!(s.contains("hello world"));
        let mut small = Terminal::new(TestBackend::new(30, 8)).unwrap();
        small.draw(|f| draw(f, &app)).unwrap();
        assert!(dump(&small).contains("too small"));
    }

    #[test]
    fn highlighting_flags_unknown_commands() {
        let h = highlight("zzz foo");
        assert_eq!(h[0].style, theme::err());
        let h = highlight("report export --format html");
        assert!(h.iter().any(|s| s.content == "--format" && s.style == theme::warn()));
    }
}
