use eradicate_tui::{App, ErrorBox, AppMode};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Corner, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

fn main() -> Result<(), ErrorBox> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let tick_rate = Duration::from_millis(250);

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    tick_rate: Duration,
) -> Result<(), ErrorBox> {
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| draw_ui(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match app.app_mode {
                    AppMode::Normal => match key.code {
                        KeyCode::Enter => app.toggle_delete(),
                        KeyCode::Down | KeyCode::Char('j') => app.list.next(),
                        KeyCode::Up | KeyCode::Char('k') => app.list.previous(),
                        KeyCode::Char('g') => app.toggle_case_sensitive(),
                        KeyCode::Char('q') => break,
                        KeyCode::Char('i') => {
                            app.set_app_mode(AppMode::Insert);
                        }
                        KeyCode::Char('d') => app.delete_active_entries()?,
                        _ => {}
                    },
                    AppMode::Insert => match key.code {
                        KeyCode::Char(c) => {
                            app.curret_input.push(c);
                        }
                        KeyCode::Enter => {
                            let input = app.curret_input.clone();
                            app.set_pattern(&input)?;
                            app.set_app_mode(AppMode::Normal);
                        }
                        KeyCode::Backspace => {
                            app.curret_input.pop();
                        }
                        KeyCode::Esc => {
                            app.set_app_mode(AppMode::Normal);
                        }
                        _ => {}
                    },
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }

    Ok(())
}

fn draw_ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(f.size());

    let bg_box = Block::default().style(Style::default().bg(Color::Black));

    f.render_widget(bg_box, f.size());

    // build left side
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Percentage(5), Constraint::Percentage(95)].as_ref())
        .split(chunks[0]);

    let spans = Spans::from(vec![
        Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" toggle entry deletion, "),
        Span::styled("[d]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("elete active entries"),
    ]);

    let help_style = match app.app_mode {
        AppMode::Normal => Style::default(),
        AppMode::Insert => Style::default().fg(Color::Gray),
    };

    let mut text = Text::from(spans);
    text.patch_style(help_style);
    let help_text = Paragraph::new(text);
    f.render_widget(help_text, left_chunks[0]);

    let chunk_width = chunks[0].width as usize;

    let items: Vec<ListItem> = app
        .list
        .items
        .iter()
        .map(|entry| {
            let (turbo, turbo_color) = match entry.is_delete() {
                true => ("o <> o", Color::Red),
                false => ("- <> -", Color::Gray),
            };

            let file_type = if entry.is_file { "File" } else { "Dir" };

            let header = Spans::from(vec![
                Span::styled(file_type, Style::default().fg(Color::LightGreen)),
                Span::raw(" "),
                Span::styled(turbo, Style::default().fg(turbo_color)),
            ]);

            let path_display = entry.pathbuf.display().to_string();
            let path_desc = Spans::from(vec![Span::raw(path_display)]);

            ListItem::new(vec![
                header,
                path_desc,
                Spans::from("-".repeat(chunk_width)),
            ])
            .style(Style::default().fg(Color::LightCyan).bg(Color::Black))
        })
        .collect();

    let n = app.get_entries_by(|e| e.is_delete()).len();
    let spans = Spans::from(vec![
        Span::raw("Entries to eradicate: "),
        Span::styled(
            n.to_string(),
            Style::default().add_modifier(Modifier::BOLD).fg(Color::Red),
        ),
        Span::raw(" "),
    ]);

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(spans.0)
                .border_type(BorderType::Rounded),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )
        .start_corner(Corner::TopLeft);

    f.render_stateful_widget(list, left_chunks[1], &mut app.list.state);

    // build right side
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(chunks[1]);

    // build help message

    let (msg, style) = match app.app_mode {
        AppMode::Normal => (
            vec![
                Span::styled("[i]", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("nsert mode "),
                Span::styled("[g]", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" toogle case sensitive matches "),
                Span::styled("[q]", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("uit"),
            ],
            Style::default(),
        ),
        AppMode::Insert => (
            vec![
                Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" set the pattern, "),
                Span::styled("[Esc]", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" exit insert mode"),
            ],
            Style::default(),
        ),
    };

    let mut text = Text::from(Spans::from(msg));
    text.patch_style(style);
    let help_message = Paragraph::new(text);

    f.render_widget(help_message, right_chunks[0]);

    // display current pattern

    let case_text = if app.is_case_sensitive() { "ON" } else { "OFF" };

    let spans = match app.pattern.is_empty() {
        false => Spans::from(vec![
            Span::raw("Searching: "),
            Span::styled(
                app.pattern.as_str(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(", case sensitive: "),
            Span::styled(case_text, Style::default().add_modifier(Modifier::BOLD)),
        ]),
        true => Spans::from(vec![Span::styled(
            "Empty pattern, try inserting a new one",
            Style::default().add_modifier(Modifier::ITALIC),
        )]),
    };

    let mut text = Text::from(spans);
    text.patch_style(Style::default().fg(Color::Magenta));
    f.render_widget(Paragraph::new(text), right_chunks[1]);

    // display input

    let input = Paragraph::new(app.curret_input.as_ref())
        .style(match app.app_mode {
            AppMode::Normal => Style::default(),
            AppMode::Insert => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title("Pattern"));

    f.render_widget(input, right_chunks[2]);

    match app.app_mode {
        AppMode::Normal => {}
        AppMode::Insert => f.set_cursor(
            right_chunks[2].x + app.curret_input.width() as u16 + 1,
            right_chunks[2].y + 1,
        ),
    }
}
