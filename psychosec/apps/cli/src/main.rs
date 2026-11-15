mod api;
mod app;
mod auth;
mod commands;
mod config;
mod creds;
mod crypto;
mod osint;
mod output;
mod report;
mod store;
mod theme;
mod ui;
mod update;

use ratatui::crossterm::event::{self, Event, KeyEventKind};
use std::time::Duration;

fn main() {
    let rt = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("PSYCHOSEC could not start its runtime: {e}");
            std::process::exit(1);
        }
    };
    let _guard = rt.enter();
    let mut app = match app::App::new(rt.handle().clone()) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("PSYCHOSEC failed to start: {e}");
            std::process::exit(1);
        }
    };
    if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        eprintln!("PSYCHOSEC must be run in an interactive terminal (Windows Terminal, PowerShell or Command Prompt).");
        std::process::exit(1);
    }
    // `ratatui::init` enters raw mode + alternate screen and installs a panic hook
    // that restores the terminal before the panic message is printed.
    let mut terminal = ratatui::init();
    let _ = ratatui::crossterm::execute!(std::io::stdout(), ratatui::crossterm::terminal::SetTitle("PSYCHOSEC"));
    app.start();
    let res = run(&mut terminal, &mut app);
    ratatui::restore();
    if let Err(e) = res {
        eprintln!("PSYCHOSEC terminal error: {e}");
        std::process::exit(1);
    }
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut app::App) -> std::io::Result<()> {
    while !app.should_quit {
        app.pump();
        terminal.draw(|f| ui::draw(f, app))?;
        if event::poll(Duration::from_millis(80))? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press || k.kind == KeyEventKind::Repeat => app.on_key(k),
                Event::Resize(_, _) => {
                    terminal.autoresize()?;
                }
                _ => {}
            }
        }
    }
    Ok(())
}
