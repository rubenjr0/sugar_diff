use std::{io, time::Duration};

use color_eyre::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::*,
    style::{Color, Modifier, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, List, ListItem, Paragraph},
    Frame, Terminal,
};

mod meassurement;
use meassurement::Meassurement;

enum InputMode {
    Level,
    Time,
}

struct App {
    level_input: String,
    time_input: String,
    input_mode: InputMode,
    meassurements: Vec<Meassurement>,
}

impl App {
    fn new() -> App {
        App {
            level_input: String::new(),
            time_input: String::new(),
            input_mode: InputMode::Level,
            meassurements: vec![],
        }
    }

    fn add_meassure(&mut self) -> Result<()> {
        let m = Meassurement::new(self.level_input.to_owned(), self.time_input.to_owned())?;
        self.meassurements.insert(
            if let Some((idx, _)) = self
                .meassurements
                .iter()
                .enumerate()
                .rev()
                .find(|(_, m2)| m2.timestamp() < m.timestamp())
            {
                idx+1
            } else {
                0
            },
            m,
        );
        self.level_input.clear();
        self.time_input.clear();
        Ok(())
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|f| {
        let size = f.size();
        let block = Block::default().title("Sugar Diff").borders(Borders::ALL);
        f.render_widget(block, size);
    })?;

    let app = App::new();
    let res = run_app(&mut terminal, app);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    terminal.show_cursor()?;
    res
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => return Ok(()),
                KeyCode::Backspace => {
                    match app.input_mode {
                        InputMode::Level => app.level_input.pop(),
                        InputMode::Time => app.time_input.pop(),
                    };
                }
                KeyCode::Char(c) => {
                    match app.input_mode {
                        InputMode::Level => app.level_input.push(c),
                        InputMode::Time => app.time_input.push(c),
                    };
                }
                KeyCode::Enter => match app.input_mode {
                    InputMode::Level => app.input_mode = InputMode::Time,
                    InputMode::Time => {
                        app.add_meassure()?;
                        app.input_mode = InputMode::Level;
                    }
                },
                _ => continue,
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(8),
                Constraint::Min(1),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.size());

    let level_input = Paragraph::new(app.level_input.as_ref())
        .block(Block::default().borders(Borders::ALL).title("Level"));
    f.render_widget(level_input, chunks[0]);

    let time_input = Paragraph::new(app.time_input.as_ref())
        .block(Block::default().borders(Borders::ALL).title("Time"));
    f.render_widget(time_input, chunks[1]);

    let mut items = vec![];
    let start_idx = if app.meassurements.len() >= 6 {
        app.meassurements.len() - 6
    } else {
        0
    };
    for i in start_idx..app.meassurements.len() {
        let m = &app.meassurements[i];
        items.push(ListItem::new(format!(
            "{} {}",
            m.to_string(),
            if i > 0 {
                format!("({:+.3} / min)", m.diff(&app.meassurements[i - 1]))
            } else {
                String::new()
            }
        )));
    }

    f.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::TOP)
                .title("Meassurements"),
        ),
        chunks[2],
    );

    let data = app
        .meassurements
        .iter()
        .map(|m| (m.timestamp() as f64, m.y() as f64))
        .collect::<Vec<_>>();
    let dataset = Dataset::default()
        .marker(symbols::Marker::Braille)
        .graph_type(tui::widgets::GraphType::Line)
        .style(Style::default().fg(Color::Cyan))
        .data(&data);
    let chart = Chart::new(vec![dataset])
        .block(
            Block::default()
                .title(Span::styled(
                    "Meassurements chart",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default().title("Time").bounds([
                data.iter()
                    .map(|dp| dp.0)
                    .reduce(f64::min)
                    .unwrap_or_else(|| 0.0)
                    * 0.9,
                1440.0,
            ]),
        )
        .y_axis(Axis::default().title("Level").bounds([0.0, 400.0]));
    f.render_widget(chart, chunks[3]);

    let time_to_text = {
        let mut text = String::from("Waiting for meassurements...");
        if app.meassurements.len() >= 2 {
            let mf = app.meassurements.last();
            let mpf = app.meassurements.get(app.meassurements.len() - 2);
            if let Some((level, rate)) = mf.and_then(|mf| Some((mf.y(), mf.diff(mpf.unwrap())))) {
                let (variant, time) = if rate <= 0.0 {
                    ("low", (80 - level) as f32 / rate)
                } else {
                    ("high", (300 - level) as f32 / rate)
                };
                text = format!(
                    "Time to {}: {}",
                    variant,
                    human_duration::human_duration(&Duration::from_secs((time * 60.0) as u64))
                );
            }
        }
        text
    };
    let time_to = Paragraph::new(time_to_text).block(Block::default().borders(Borders::ALL));
    f.render_widget(time_to, chunks[4]);

    let (idx, len) = match app.input_mode {
        InputMode::Level => (0, app.level_input.len() as u16),
        InputMode::Time => (1, app.time_input.len() as u16),
    };
    f.set_cursor(chunks[idx].x + len + 1, chunks[idx].y + 1);
}
