use crate::types::{AppState, Post, Screen};
use crate::image;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
//use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

pub fn draw(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();

    let constraints = if state.search_active {
        vec![
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ]
    } else {
        vec![
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    draw_title_bar(frame, chunks[0], state);
    if state.search_active {
        draw_search_bar(frame, chunks[1], state);
        draw_main(frame, chunks[2], state);
    } else {
        draw_main(frame, chunks[1], state);
    }
    draw_status_bar(frame, chunks[if state.search_active { 3 } else { 2 }], state);
}

fn draw_title_bar(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let title = match state.screen {
        Screen::BoardList => " 5ちゃんねるびゅあ - 板一覧",
        Screen::ThreadList => {
            if let Some(ref board) = state.current_board {
                let brd = if board.len() > 40 {
                    format!("...{}", &board[board.len().saturating_sub(37)..])
                } else {
                    board.clone()
                };
                Box::leak(format!(" 5ちゃんねるびゅあ - {}", brd).into_boxed_str())
            } else {
                " 5ちゃんねるびゅあ - スレ一覧"
            }
        }
        Screen::Compose => " 5ちゃんねるびゅあ - レス投稿",
        Screen::ThreadView => {
            if let Some(ref info) = state.thread_info {
                let short = if info.len() > 50 {
                    //format!("{}...", &info[..47])
                    let truncated: String = info.chars().take(47).collect();
                    format!("{}...", truncated)
                } else {
                    info.clone()
                };
                Box::leak(format!(" 5ちゃんねるびゅあ - {}", short).into_boxed_str())
            } else {
                " 5ちゃんねるびゅあ - スレ"
            }
        }
    };

    let block = Paragraph::new(Line::from(Span::styled(
        title,
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )))
    .style(Style::default().bg(Color::Blue));

    frame.render_widget(block, area);
}

fn draw_search_bar(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let match_count = state.search_matches.len();
    let total = match state.screen {
        Screen::BoardList => state.boards.len(),
        Screen::ThreadList => state.threads.len(),
        Screen::ThreadView => state.posts.len(),
        Screen::Compose => 0,
    };
    let info = if match_count == total {
        format!("/{} ", state.search_query)
    } else {
        format!("/{} ({}件)", state.search_query, match_count)
    };

    let bar = Paragraph::new(Line::from(Span::styled(
        info,
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    )))
    .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(bar, area);
}

fn draw_main(frame: &mut Frame, area: Rect, state: &mut AppState) {
    if state.loading {
        let loading = Paragraph::new("読み込み中...")
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default()),
            );
        frame.render_widget(loading, area);
        return;
    }

    match state.screen {
        Screen::BoardList => draw_board_list(frame, area, state),
        Screen::ThreadList => draw_thread_list(frame, area, state),
        Screen::ThreadView => draw_thread_view(frame, area, state),
        Screen::Compose => draw_compose(frame, area, state),
    }
}

fn draw_board_list(frame: &mut Frame, area: Rect, state: &mut AppState) {
    if state.boards.is_empty() {
        let empty = Paragraph::new("板一覧が空です。'r' で再読み込み")
            .style(Style::default().fg(Color::Gray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default()),
            );
        frame.render_widget(empty, area);
        return;
    }

    let fav = &state.favorites;
    let items: Vec<ListItem> = if state.search_active && !state.search_matches.is_empty() {
        state
            .search_matches
            .iter()
            .map(|&i| {
                let board = &state.boards[i];
                let star = if fav.contains(&board.url) { "★ " } else { "  " };
                ListItem::new(format!("{}{}", star, board.name))
            })
            .collect()
    } else if !state.search_active {
        state
            .boards
            .iter()
            .map(|board| {
                let star = if fav.contains(&board.url) { "★ " } else { "  " };
                ListItem::new(format!("{}{}", star, board.name))
            })
            .collect()
    } else {
        Vec::new()
    };

    let title = if state.search_active {
        format!("板一覧 ({}件)", state.search_matches.len())
    } else {
        "板一覧".to_string()
    };

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" > ")
        .block(Block::default().borders(Borders::ALL).title(title));

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_index));
    *list_state.offset_mut() = state.list_offset;
    state.visible_items = (area.height.saturating_sub(2)) as usize;

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_thread_list(frame: &mut Frame, area: Rect, state: &mut AppState) {
    if state.threads.is_empty() {
        let empty = Paragraph::new("スレ一覧が空です。'r' で再読み込み")
            .style(Style::default().fg(Color::Gray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default()),
            );
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = if state.search_active && !state.search_matches.is_empty() {
        state
            .search_matches
            .iter()
            .map(|&i| {
                let thread = &state.threads[i];
                let max_chars = area.width as usize - 18;
                let title = if thread.title.chars().count() > max_chars {
                    format!(
                        "{}...",
                        thread.title.chars().take(max_chars).collect::<String>()
                    )
                } else {
                    thread.title.clone()
                };
                ListItem::new(format!("{} ({}レス)", title, thread.post_count))
            })
            .collect()
    } else if !state.search_active {
        state
            .threads
            .iter()
            .map(|thread| {
                let max_chars = area.width as usize - 18;
                let title = if thread.title.chars().count() > max_chars {
                    format!(
                        "{}...",
                        thread.title.chars().take(max_chars).collect::<String>()
                    )
                } else {
                    thread.title.clone()
                };
                ListItem::new(format!("{} ({}レス)", title, thread.post_count))
            })
            .collect()
    } else {
        Vec::new()
    };

    let title = if state.search_active {
        format!("スレ一覧 ({}件)", state.search_matches.len())
    } else {
        "スレ一覧".to_string()
    };

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" > ")
        .block(Block::default().borders(Borders::ALL).title(title));

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_index));
    *list_state.offset_mut() = state.list_offset;
    state.visible_items = (area.height.saturating_sub(2)) as usize;

    frame.render_stateful_widget(list, area, &mut list_state);
}


fn draw_thread_view(frame: &mut Frame, area: Rect, state: &mut AppState) {
    if state.posts.is_empty() {
        let empty = Paragraph::new("レスがありません。'r' で再読み込み")
            .style(Style::default().fg(Color::Gray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default()),
            );
        frame.render_widget(empty, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    if state.search_active {
        let total = state.posts.len();
        let info = format!(
            " 検索結果: {}件 / {}レス | ↑↓:移動 | Enter:決定",
            state.search_matches.len(),
            total,
        );
        let info_bar = Paragraph::new(Line::from(Span::styled(
            info,
            Style::default().fg(Color::Cyan),
        )))
        .style(Style::default().bg(Color::DarkGray));
        frame.render_widget(info_bar, chunks[0]);

        let items: Vec<ListItem> = state
            .search_matches
            .iter()
            .map(|&i| {
                let post = &state.posts[i];
                let first_line = post.body.lines().next().unwrap_or("");
                let preview = if first_line.chars().count() > area.width as usize - 10 {
                    format!(
                        "{}...",
                        first_line.chars().take(area.width as usize - 13).collect::<String>()
                    )
                } else {
                    first_line.to_string()
                };
                ListItem::new(format!("#{} {}", i + 1, preview))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(" > ")
            .block(Block::default().borders(Borders::ALL));
        let mut list_state = ListState::default();
        list_state.select(Some(state.selected_index));
        frame.render_stateful_widget(list, chunks[1], &mut list_state);
    } else {
        let total = state.posts.len();
        let info = format!(
            " 1-{}/{} | ↑↓:スクロール | ←:戻る | /:検索 | w:書き込み | q:終了",
            total, total
        );
        let info_bar = Paragraph::new(Line::from(Span::styled(
            info,
            Style::default().fg(Color::Cyan),
        )))
        .style(Style::default().bg(Color::DarkGray));
        frame.render_widget(info_bar, chunks[0]);

        let content_width = chunks[1].width.saturating_sub(2);
        let mut all_lines: Vec<Line<'static>> = Vec::new();
        for (i, post) in state.posts.iter().enumerate() {
            all_lines.extend(format_post(i + 1, post, content_width));
        }
        state.visible_items = (chunks[1].height.saturating_sub(2)) as usize;
        let text = Text::from(all_lines);

        let paragraph = Paragraph::new(text)
            .scroll((state.scroll_offset as u16, 0))
            .style(Style::default())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default()),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, chunks[1]);
    }
}

fn draw_compose(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(3),
        ])
        .split(area);

    let field_style = |focus: bool| -> Style {
        if focus {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            Style::default().fg(Color::White)
        }
    };

    let name_text = format!(" 名前: {} ", state.compose_name);
    let name_bar = Paragraph::new(Line::from(Span::styled(
        name_text,
        field_style(state.compose_focus == 0),
    )))
    .block(Block::default().borders(Borders::ALL).title("名前"));
    frame.render_widget(name_bar, chunks[0]);

    let email_text = format!(" メール: {} ", state.compose_email);
    let email_bar = Paragraph::new(Line::from(Span::styled(
        email_text,
        field_style(state.compose_focus == 1),
    )))
    .block(Block::default().borders(Borders::ALL).title("メール"));
    frame.render_widget(email_bar, chunks[1]);

    let message_text = if state.compose_message.is_empty() {
        Text::styled(" ", field_style(state.compose_focus == 2))
    } else {
        Text::styled(
            state.compose_message.clone(),
            field_style(state.compose_focus == 2),
        )
    };
    let message_block = Paragraph::new(message_text)
        .block(Block::default().borders(Borders::ALL).title("メッセージ"))
        .wrap(Wrap { trim: false });
    frame.render_widget(message_block, chunks[2]);
}

fn format_post(num: usize, post: &Post, content_width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    let id_str = match &post.id {
        Some(id) => format!(" ID:{}", id),
        None => String::new(),
    };
    let email_str = if post.email.is_empty() || post.email == "sage" {
        String::new()
    } else {
        format!(" <{}>", post.email)
    };
    let header = format!("{}{} 名前:{} {} {}", num, email_str, post.name, post.date, id_str);
    let sep = "-".repeat(header.len().min(60));

    lines.push(Line::from(sep));
    lines.push(Line::from(header));

    for body_line in post.body.lines() {
        /*
        if let Some(img_url) = image::find_image_url(body_line)
            && let Some(path) = image::download_image(&img_url)
            && let Some(img_lines) = image::render_image(&path, content_width)
        {
            lines.extend(img_lines);
        } else {
            lines.push(Line::from(body_line.to_string()));
        }
        */
        lines.push(Line::from(body_line.to_string()));
    }

    lines.push(Line::from(""));
    lines
}

fn draw_status_bar(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let help = if state.search_active {
        " 文字入力:検索 | Esc:終了 | Enter:決定 | ↑↓:移動 "
    } else {
        match state.screen {
            Screen::BoardList => {
                " ↑↓:移動 | Enter:板を開く | /:検索 | f:お気に入り | r:再読み込み | q:終了 "
            }
            Screen::ThreadList => {
                " ↑↓:移動 | Enter:スレを開く | /:検索 | r:再読み込み | ←:戻る | q:終了 "
            }
            Screen::ThreadView => {
                " ↑↓:スクロール | /:検索 | ←:戻る | r:再読み込み | w:書き込み | q:終了 "
            }
            Screen::Compose => {
                " Tab:項目移動 | Ctrl+S:送信 | Esc:キャンセル "
            }
        }
    };

    let status = if state.loading {
        "読み込み中..."
    } else {
        &state.status_message
    };

    let text = Line::from(vec![
        Span::styled(status, Style::default().fg(Color::Yellow)),
        Span::raw(" | "),
        Span::styled(help, Style::default().fg(Color::DarkGray)),
    ]);

    let block = Paragraph::new(text).style(Style::default().bg(Color::Black));
    frame.render_widget(block, area);
}
