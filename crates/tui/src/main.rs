mod app;
mod ui;

use anyhow::Result;
use app::InputMode;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use std::io::stdout;

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut app = app::App::new()?;

    fetch_versions_async(&mut app).await;

    let result = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

async fn fetch_versions_async(app: &mut app::App) {
    app.loading = true;
    let http = reqwest::Client::new();
    if let Ok(versions) = miao_core::version::manifest::fetch_version_manifest(
        &http,
        &app.config.download_mirror,
    )
    .await
    {
        let releases: Vec<_> = versions
            .into_iter()
            .filter(|v| v.is_release())
            .take(50)
            .collect();
        app.set_versions(releases);
    } else {
        app.status_message = "Failed to fetch versions. Check network.".to_string();
        app.loading = false;
    }
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut app::App,
) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if app.input_mode == InputMode::Input {
                match key.code {
                    KeyCode::Enter => app.confirm_input(),
                    KeyCode::Esc => app.go_back(),
                    KeyCode::Backspace => { app.input_buffer.pop(); }
                    KeyCode::Char(c) => app.input_buffer.push(c),
                    _ => {}
                }
                continue;
            }

            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Tab => app.next_tab(),
                KeyCode::BackTab => app.prev_tab(),
                KeyCode::Up | KeyCode::Char('k') => app.prev_item(),
                KeyCode::Down | KeyCode::Char('j') => app.next_item(),
                KeyCode::Enter => app.select_item(),
                KeyCode::Char('h') | KeyCode::Left => app.go_back(),
                KeyCode::Char('a') => app.add_offline_account(),
                KeyCode::Char('r') => app.refresh_instances(),
                _ => {}
            }
        }
    }
}
