mod api;
mod app;
mod types;
mod ui;

use crate::types::{AppState, Screen};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
fn main() -> Result<()> {
    color_eyre::install()?;

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    stdout.execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut state = AppState::new();
    state.load_boards();

    loop {
        terminal.draw(|f| ui::draw(f, &mut state))?;

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            if state.search_active {
                match key.code {
                    KeyCode::Esc => state.exit_search(),
                    KeyCode::Enter => {
                        state.exit_search();
                        match state.screen {
                            Screen::BoardList => state.select_board(),
                            Screen::ThreadList => state.select_thread(),
                            Screen::ThreadView => {}
                        }
                    }
                    KeyCode::Backspace => state.search_pop_char(),
                    KeyCode::Char('/') => state.exit_search(),
                    KeyCode::Up | KeyCode::Char('k') => state.prev_item(),
                    KeyCode::Down | KeyCode::Char('j') => state.next_item(),
                    KeyCode::Left | KeyCode::Char('h') => {
                        state.exit_search();
                    }
                    KeyCode::Char(c) if !c.is_control() => state.search_push_char(c),
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('/') => state.enter_search(),
                    KeyCode::Enter => match state.screen {
                        Screen::BoardList => state.select_board(),
                        Screen::ThreadList => state.select_thread(),
                        Screen::ThreadView => {}
                    },
                    KeyCode::Up | KeyCode::Char('k') => match state.screen {
                        Screen::BoardList | Screen::ThreadList => state.prev_item(),
                        Screen::ThreadView => state.scroll_up(),
                    },
                    KeyCode::Down | KeyCode::Char('j') => match state.screen {
                        Screen::BoardList | Screen::ThreadList => state.next_item(),
                        Screen::ThreadView => state.scroll_down(),
                    },
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
                        state.go_back();
                    }
                    KeyCode::Char('r') => match state.screen {
                        Screen::BoardList => state.load_boards(),
                        Screen::ThreadList => state.load_threads(),
                        Screen::ThreadView => {
                            let tid = state.current_thread_id.clone();
                            if let Some(id) = tid {
                                state.load_posts(&id);
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    let mut out = std::io::stdout();
    out.execute(LeaveAlternateScreen)?;

    Ok(())
}
