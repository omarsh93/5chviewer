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
            }
        }
    }

    fn compute_matches(&self) -> Vec<usize> {
        if self.search_query.is_empty() {
            let len = match self.screen {
                Screen::BoardList => self.boards.len(),
                Screen::ThreadList => self.threads.len(),
                Screen::ThreadView => self.posts.len(),
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
        }
    }

    pub fn enter_search(&mut self) {
        self.search_active = true;
        self.search_query.clear();
        self.search_matches = self.compute_matches();
        self.selected_index = 0;
    }

    pub fn exit_search(&mut self) {
        if self.search_active && !self.search_matches.is_empty() {
            self.selected_index =
                self.search_matches[self.selected_index.min(self.search_matches.len() - 1)];
        }
        self.search_active = false;
        self.search_query.clear();
        self.search_matches.clear();
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

    pub fn load_boards(&mut self) {
        self.loading = true;
        self.status_message = "板一覧を読み込み中...".to_string();
        match api::fetch_boards() {
            Ok(boards) => {
                self.boards = boards;
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

        if let Some(ref url) = self.current_board_url {
            match api::fetch_threads(url) {
                Ok(threads) => {
                    self.threads = threads;
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
        self.current_thread_title = Some(thread_title);
        self.current_thread_id = Some(thread_id.clone());
        self.load_posts(&thread_id);
    }

    pub fn load_posts(&mut self, thread_id: &str) {
        self.loading = true;
        self.status_message = "レスを読み込み中...".to_string();
        self.selected_index = 0;
        self.scroll_offset = 0;

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
                self.current_board = None;
                self.current_board_url = None;
            }
            Screen::ThreadView => {
                self.screen = Screen::ThreadList;
                self.selected_index = 0;
                self.scroll_offset = 0;
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
    }

    pub fn prev_item(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
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
}
