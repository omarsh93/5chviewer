use crate::api;
use crate::types::{AppState, Screen};

impl AppState {
    fn map_selected_index(&self) -> usize {
        if self.search_active && !self.search_matches.is_empty() {
            self.search_matches[self.selected_index.min(self.search_matches.len() - 1)]
        } else {
            self.selected_index
        }
    }

    fn item_count(&self) -> usize {
        if self.search_active && !self.search_matches.is_empty() {
            self.search_matches.len()
        } else {
            match self.screen {
                Screen::BoardList => self.boards.len(),
                Screen::ThreadList => self.threads.len(),
                Screen::ThreadView => self.posts.len(),
                Screen::Compose => 0,
            }
        }
    }

    fn compute_matches(&self) -> Vec<usize> {
        if self.search_query.is_empty() {
            let len = match self.screen {
                Screen::BoardList => self.boards.len(),
                Screen::ThreadList => self.threads.len(),
                Screen::ThreadView => self.posts.len(),
                Screen::Compose => 0,
            };
            return (0..len).collect();
        }
        let q = self.search_query.to_lowercase();
        match self.screen {
            Screen::BoardList => self
                .boards
                .iter()
                .enumerate()
                .filter(|(_, b)| b.name.to_lowercase().contains(&q))
                .map(|(i, _)| i)
                .collect(),
            Screen::ThreadList => self
                .threads
                .iter()
                .enumerate()
                .filter(|(_, t)| t.title.to_lowercase().contains(&q))
                .map(|(i, _)| i)
                .collect(),
            Screen::ThreadView => self
                .posts
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    p.body.to_lowercase().contains(&q) || p.name.to_lowercase().contains(&q)
                })
                .map(|(i, _)| i)
                .collect(),
            Screen::Compose => Vec::new(),
        }
    }

    pub fn enter_search(&mut self) {
        self.search_active = true;
        self.search_query.clear();
        self.search_matches = self.compute_matches();
        self.selected_index = 0;
        self.list_offset = 0;
    }

    pub fn exit_search(&mut self) {
        if self.search_active && !self.search_matches.is_empty() {
            self.selected_index =
                self.search_matches[self.selected_index.min(self.search_matches.len() - 1)];
        }
        self.search_active = false;
        self.search_query.clear();
        self.search_matches.clear();
        self.list_offset = 0;
    }

    pub fn search_push_char(&mut self, c: char) {
        self.search_query.push(c);
        self.search_matches = self.compute_matches();
        self.selected_index = 0;
    }

    pub fn search_pop_char(&mut self) {
        self.search_query.pop();
        self.search_matches = self.compute_matches();
        self.selected_index = 0;
    }

    fn data_path() -> std::path::PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let mut path = std::path::PathBuf::from(home);
        path.push(".config");
        path.push("5chviewer");
        path
    }

    pub fn load_read_threads(&mut self) {
        let file = Self::data_path().join("read_threads");
        let content = match std::fs::read_to_string(&file) {
            Ok(c) => c,
            Err(_) => return,
        };
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() {
                self.read_threads.insert(line.to_string());
            }
        }
    }

    pub fn save_read_threads(&self) {
        let dir = Self::data_path();
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let file = dir.join("read_threads");
        let content = self.read_threads.iter().cloned().collect::<Vec<_>>().join("\n");
        let _ = std::fs::write(&file, content);
    }

    fn favorites_path() -> std::path::PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let mut path = std::path::PathBuf::from(home);
        path.push(".config");
        path.push("5chviewer");
        path
    }

    pub fn load_favorites(&mut self) {
        let dir = Self::favorites_path();
        let file = dir.join("favorites");
        let content = match std::fs::read_to_string(&file) {
            Ok(c) => c,
            Err(_) => return,
        };
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() {
                self.favorites.insert(line.to_string());
            }
        }
    }

    pub fn save_favorites(&self) {
        let dir = Self::favorites_path();
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let file = dir.join("favorites");
        let content = self.favorites.iter().cloned().collect::<Vec<_>>().join("\n");
        let _ = std::fs::write(&file, content);
    }

    pub fn toggle_favorite(&mut self) {
        if self.boards.is_empty() {
            return;
        }
        let idx = self.map_selected_index();
        let board_url = self.boards[idx].url.clone();
        let board_name = self.boards[idx].name.clone();
        if !self.favorites.insert(board_url.clone()) {
            self.favorites.remove(&board_url);
        }
        self.save_favorites();
        self.sort_boards_with_favorites();
        self.selected_index = 0;
        self.list_offset = 0;
        let status = if self.favorites.contains(&board_url) { "追加" } else { "解除" };
        self.status_message = format!("お気に入り{}: {}", status, board_name);
    }

    fn sort_threads_with_read(&mut self) {
        let read = &self.read_threads;
        let board_url = self.current_board_url.as_deref().unwrap_or("");
        self.threads.sort_by(|a, b| {
            let a_key = format!("{}|{}", board_url, a.id);
            let b_key = format!("{}|{}", board_url, b.id);
            let a_read = read.contains(&a_key);
            let b_read = read.contains(&b_key);
            b_read.cmp(&a_read)
        });
    }

    fn sort_boards_with_favorites(&mut self) {
        let fav = &self.favorites;
        self.boards.sort_by(|a, b| {
            let a_fav = fav.contains(&a.url);
            let b_fav = fav.contains(&b.url);
            b_fav.cmp(&a_fav).then(a.name.cmp(&b.name))
        });
        self.search_matches = self.compute_matches();
    }

    pub fn load_boards(&mut self) {
        self.loading = true;
        self.status_message = "板一覧を読み込み中...".to_string();
        match api::fetch_boards() {
            Ok(boards) => {
                self.boards = boards;
                self.sort_boards_with_favorites();
                self.status_message = format!("{} 個の板を読み込みました", self.boards.len());
            }
            Err(e) => {
                self.status_message = format!("エラー: {}", e);
            }
        }
        self.loading = false;
    }

    pub fn select_board(&mut self) {
        if self.boards.is_empty() {
            return;
        }
        let idx = self.map_selected_index();
        let board = &self.boards[idx];
        self.current_board = Some(board.name.clone());
        self.current_board_url = Some(board.url.clone());
        self.load_threads();
    }

    pub fn load_threads(&mut self) {
        self.loading = true;
        self.status_message = "スレ一覧を読み込み中...".to_string();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.list_offset = 0;

        if let Some(ref url) = self.current_board_url {
            match api::fetch_threads(url) {
                Ok(threads) => {
                    self.threads = threads;
                    self.sort_threads_with_read();
                    self.status_message =
                        format!("{} 個のスレを読み込みました", self.threads.len());
                }
                Err(e) => {
                    self.status_message = format!("エラー: {}", e);
                }
            }
        }
        self.loading = false;
        self.screen = Screen::ThreadList;
    }

    pub fn select_thread(&mut self) {
        if self.threads.is_empty() {
            return;
        }
        let idx = self.map_selected_index();
        let thread_id = self.threads[idx].id.clone();
        let thread_title = self.threads[idx].title.clone();
        if let Some(ref board_url) = self.current_board_url {
            self.read_threads.insert(format!("{}|{}", board_url, thread_id));
            self.save_read_threads();
        }
        self.current_thread_title = Some(thread_title);
        self.current_thread_id = Some(thread_id.clone());
        self.load_posts(&thread_id);
    }

    pub fn load_posts(&mut self, thread_id: &str) {
        self.loading = true;
        self.status_message = "レスを読み込み中...".to_string();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.list_offset = 0;

        if let Some(ref url) = self.current_board_url {
            match api::fetch_posts(url, thread_id) {
                Ok(posts) => {
                    let count = posts.len();
                    self.posts = posts;
                    self.thread_info = self
                        .current_thread_title
                        .as_ref()
                        .map(|t| format!("{} ({}レス)", t, count));
                    self.status_message = format!("{} レスを読み込みました", count);
                }
                Err(e) => {
                    self.status_message = format!("エラー: {}", e);
                }
            }
        }
        self.loading = false;
        self.screen = Screen::ThreadView;
    }

    pub fn go_back(&mut self) {
        if self.search_active {
            self.exit_search();
            return;
        }
        match self.screen {
            Screen::ThreadList => {
                self.screen = Screen::BoardList;
                self.selected_index = 0;
                self.scroll_offset = 0;
                self.list_offset = 0;
                self.current_board = None;
                self.current_board_url = None;
            }
            Screen::ThreadView => {
                self.screen = Screen::ThreadList;
                self.selected_index = 0;
                self.scroll_offset = 0;
                self.list_offset = 0;
                self.posts.clear();
                self.current_thread_title = None;
                self.current_thread_id = None;
                self.thread_info = None;
            }
            _ => {}
        }
    }

    pub fn next_item(&mut self) {
        let len = self.item_count();
        if len == 0 {
            return;
        }
        if self.selected_index + 1 < len {
            self.selected_index += 1;
        }
        if self.visible_items > 0 {
            let max_offset = self.selected_index.saturating_sub(self.visible_items - 1);
            if self.list_offset < max_offset {
                self.list_offset = max_offset;
            }
        }
    }

    pub fn prev_item(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
        if self.selected_index < self.list_offset {
            self.list_offset = self.selected_index;
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset += 1;
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        if self.screen == crate::types::Screen::ThreadView {
            let total_lines: usize = self.posts.iter().map(|p| p.body.lines().count() + 3).sum();
            self.scroll_offset = total_lines.saturating_sub(1);
        }
    }

    pub fn scroll_page_down(&mut self) {
        let page = self.visible_items.max(1);
        match self.screen {
            Screen::BoardList | Screen::ThreadList => {
                let len = self.item_count();
                if len == 0 {
                    return;
                }
                let new_index = (self.selected_index + page).min(len.saturating_sub(1));
                self.selected_index = new_index;
                let max_offset = self.selected_index.saturating_sub(page.saturating_sub(1));
                if self.list_offset < max_offset {
                    self.list_offset = max_offset;
                }
            }
            Screen::ThreadView => {
                self.scroll_offset = self.scroll_offset.saturating_add(page);
            }
            Screen::Compose => {}
        }
    }

    pub fn scroll_page_up(&mut self) {
        let page = self.visible_items.max(1);
        match self.screen {
            Screen::BoardList | Screen::ThreadList => {
                if self.selected_index > 0 {
                    self.selected_index = self.selected_index.saturating_sub(page);
                }
                if self.selected_index < self.list_offset {
                    self.list_offset = self.selected_index;
                }
            }
            Screen::ThreadView => {
                self.scroll_offset = self.scroll_offset.saturating_sub(page);
            }
            Screen::Compose => {}
        }
    }

    pub fn show_thread_url(&mut self) {
        if let (Some(board_url), Some(thread_id)) = (&self.current_board_url, &self.current_thread_id) {
            let host = board_url.trim_end_matches('/').split('/').nth(2).unwrap_or("");
            let board_name = board_url.trim_end_matches('/').rsplit('/').next().unwrap_or("");
            self.status_message = format!("https://{}/test/read.cgi/{}/{}", host, board_name, thread_id);
        }
    }

    pub fn enter_compose(&mut self) {
        if self.current_board_url.is_none() || self.current_thread_id.is_none() {
            self.status_message = "スレが選択されていません".to_string();
            return;
        }
        self.compose_name.clear();
        self.compose_email = "sage".to_string();
        self.compose_message.clear();
        self.compose_focus = 0;
        self.screen = Screen::Compose;
        self.status_message = "Ctrl+S:送信 Esc:キャンセル".to_string();
    }

    pub fn exit_compose(&mut self) {
        self.screen = Screen::ThreadView;
        self.status_message.clear();
    }

    pub fn compose_cycle_focus(&mut self) {
        self.compose_focus = (self.compose_focus + 1) % 3;
    }

    pub fn compose_insert_char(&mut self, c: char) {
        match self.compose_focus {
            0 => self.compose_name.push(c),
            1 => self.compose_email.push(c),
            2 => self.compose_message.push(c),
            _ => {}
        }
    }

    pub fn compose_delete_char(&mut self) {
        match self.compose_focus {
            0 => { self.compose_name.pop(); }
            1 => { self.compose_email.pop(); }
            2 => { self.compose_message.pop(); }
            _ => {}
        }
    }

    pub fn compose_newline(&mut self) {
        if self.compose_focus == 2 {
            self.compose_message.push('\n');
        } else {
            self.compose_cycle_focus();
        }
    }

    pub fn send_post(&mut self) {
        if self.compose_message.trim().is_empty() {
            self.status_message = "メッセージが空です".to_string();
            return;
        }
        let board_url = match self.current_board_url.clone() {
            Some(u) => u,
            None => {
                self.status_message = "板URLが不明です".to_string();
                return;
            }
        };
        let thread_id = match self.current_thread_id.clone() {
            Some(id) => id,
            None => {
                self.status_message = "スレIDが不明です".to_string();
                return;
            }
        };
        let name = self.compose_name.clone();
        let email = self.compose_email.clone();
        let message = self.compose_message.clone();

        self.loading = true;
        self.status_message = "送信中...".to_string();

        match api::post_message(&board_url, &thread_id, &name, &email, &message) {
            Ok(()) => {
                self.status_message = "レスを送信しました".to_string();
                self.loading = false;
                self.exit_compose();
                self.load_posts(&thread_id);
            }
            Err(e) => {
                self.status_message = format!("送信エラー: {}", e);
                self.loading = false;
            }
        }
    }
}
