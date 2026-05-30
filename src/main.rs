mod api;
mod app;
mod types;
mod ui;

use crate::types::{AppState, Screen};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
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
    state.load_favorites();
    state.load_read_threads();
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
                            Screen::Compose => {}
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
            } else if state.screen == Screen::Compose {
                match key.code {
                    KeyCode::Esc => state.exit_compose(),
                    KeyCode::Tab => state.compose_cycle_focus(),
                    KeyCode::Backspace => state.compose_delete_char(),
                    KeyCode::Enter => state.compose_newline(),
                    KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.send_post();
                    }
                    KeyCode::Char(c) if !c.is_control() => state.compose_insert_char(c),
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('/') => state.enter_search(),
                    KeyCode::Char('w') => {
                        if state.screen == Screen::ThreadView {
                            state.enter_compose();
                        }
                    }
                    KeyCode::Enter => match state.screen {
                        Screen::BoardList => state.select_board(),
                        Screen::ThreadList => state.select_thread(),
                        Screen::ThreadView => {}
                        Screen::Compose => {}
                    },
                    KeyCode::Up | KeyCode::Char('k') => match state.screen {
                        Screen::BoardList | Screen::ThreadList => state.prev_item(),
                        Screen::ThreadView => state.scroll_up(),
                        Screen::Compose => {}
                    },
                    KeyCode::Down | KeyCode::Char('j') => match state.screen {
                        Screen::BoardList | Screen::ThreadList => state.next_item(),
                        Screen::ThreadView => state.scroll_down(),
                        Screen::Compose => {}
                    },
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
                        state.go_back();
                    }
                    KeyCode::Char('f') => {
                        if state.screen == Screen::BoardList {
                            state.toggle_favorite();
                        }
                    }
                    KeyCode::Char('g') => {
                        if state.screen == Screen::ThreadView {
                            state.scroll_to_top();
                        }
                    }
                    KeyCode::Char('G') => {
                        if state.screen == Screen::ThreadView {
                            state.scroll_to_bottom();
                        }
                    }
                    KeyCode::PageDown | KeyCode::Char(' ') => match state.screen {
                        Screen::BoardList | Screen::ThreadList | Screen::ThreadView => {
                            state.scroll_page_down()
                        }
                        Screen::Compose => {}
                    },
                    KeyCode::PageUp | KeyCode::Char('b') => match state.screen {
                        Screen::BoardList | Screen::ThreadList | Screen::ThreadView => {
                            state.scroll_page_up()
                        }
                        Screen::Compose => {}
                    },
                    KeyCode::Char('r') => match state.screen {
                        Screen::BoardList => state.load_boards(),
                        Screen::ThreadList => state.load_threads(),
                        Screen::ThreadView => {
                            let tid = state.current_thread_id.clone();
                            if let Some(id) = tid {
                                state.load_posts(&id);
                            }
                        }
                        Screen::Compose => {}
                    },
                    KeyCode::Char('u') => {
                        if state.screen == Screen::ThreadView {
                            state.show_thread_url();
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    api::save_cookies();
    disable_raw_mode()?;
    let mut out = std::io::stdout();
    out.execute(LeaveAlternateScreen)?;

    Ok(())
}
