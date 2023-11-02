mod download_large_file;

use std::{
    io::{self, Stdout},
    time::Duration,
};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use downloader_rs::download_operation::DownloadOperation;
use downloader_rs::download_service::DownloadService;

/// presses 'q'.
pub fn main() -> Result<()> {
    let mut terminal = setup_terminal().context("setup failed")?;
    run(&mut terminal).context("app loop failed")?;
    restore_terminal(&mut terminal).context("restore terminal failed")?;
    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    let mut stdout = io::stdout();
    enable_raw_mode().context("failed to enable raw mode")?;
    execute!(stdout, EnterAlternateScreen).context("unable to enter alternate screen")?;
    Terminal::new(CrosstermBackend::new(stdout)).context("creating terminal failed")
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("unable to switch to main screen")?;
    terminal.show_cursor().context("unable to show cursor")
}

fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    let mut operation: Option<DownloadOperation> = None;
    let mut download_service = DownloadService::new();
    loop {
        terminal.draw(|f| {
            render_app(f, &operation);
        })?;
        if start_download()? {
            download_service.start_service();
            operation = Some(download_large_file::start_download(&mut download_service));
        }
        if should_quit()? {
            download_service.stop();
            break;
        }
    }
    Ok(())
}

fn render_app<B>(frame: &mut Frame<B>, operation: &Option<DownloadOperation>) where B: Backend {
    let mut constraints: Vec<Constraint> = Vec::new();
    constraints.push(Constraint::Max(2));
    if let Some(operation) = operation {
        constraints.push(Constraint::Max(2));
        for index in 0..operation.chunk_count() {
            constraints.push(Constraint::Max(2));
        }
    }
    constraints.push(Constraint::Max(2));

    let chunks = Layout::default()
        .constraints(constraints)
        .split(frame.size());

    let greeting = Paragraph::new("Hello World! (press 'd' to download)");
    frame.render_widget(greeting, chunks[0]);

    if let Some(operation) = operation {
        let progress = operation.progress();
        draw_progress(frame, "progress", progress, chunks[1]);
        for index in 0..operation.chunk_count() {
            draw_progress(frame, &format!("chunk{}", index), operation.chunk_progress(index), chunks[2 + index]);
        }
    }
}

fn draw_progress<B>(frame: &mut Frame<B>, title: &str, progress: f64, area: Rect) where B: Backend {
    let label = format!("{:.2}%", progress * 100.0);
    let gauge = Gauge::default()
        .block(Block::default().title(title))
        .gauge_style(
            Style::default()
                .fg(Color::Magenta)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC | Modifier::BOLD),
        )
        .label(label)
        .use_unicode(true)
        .ratio(progress);
    frame.render_widget(gauge, area);
}

fn start_download() -> Result<bool> {
    if event::poll(Duration::from_millis(250)).context("event poll failed")? {
        if let Event::Key(key) = event::read().context("event read failed")? {
            return Ok(KeyCode::Char('d') == key.code);
        }
    }
    Ok(false)
}

fn should_quit() -> Result<bool> {
    if event::poll(Duration::from_millis(250)).context("event poll failed")? {
        if let Event::Key(key) = event::read().context("event read failed")? {
            return Ok(KeyCode::Char('q') == key.code);
        }
    }
    Ok(false)
}