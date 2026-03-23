mod app;
mod bench;
mod drives;
mod headless;
mod history;
mod ui;

use app::App;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::io;
use std::time::{Duration, Instant};

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--headless") {
        let path = args
            .windows(2)
            .find(|w| w[0] == "--path")
            .map(|w| w[1].clone());
        let size = args
            .windows(2)
            .find(|w| w[0] == "--size")
            .and_then(|w| w[1].parse::<u64>().ok());
        let json = args.iter().any(|a| a == "--json");
        return headless::run(path, size, json);
    }

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        original_hook(info);
    }));

    let mut terminal = ratatui::init();
    let result = run(App::new(), &mut terminal);
    ratatui::restore();
    result
}

fn run(mut app: App, terminal: &mut ratatui::DefaultTerminal) -> io::Result<()> {
    let tick_rate = Duration::from_millis(80);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        app.should_quit = true;
                    } else {
                        app.handle_key(key.code);
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.tick();
            last_tick = Instant::now();
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
