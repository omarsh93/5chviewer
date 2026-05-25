use crate::api;
use crate::types::{AppState, Screen};

impl AppState {
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
        let board = &self.boards[self.selected_index];
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
        let thread_id = self.threads[self.selected_index].id.clone();
        let thread_title = self.threads[self.selected_index].title.clone();
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
        let len = match self.screen {
            Screen::BoardList => self.boards.len(),
            Screen::ThreadList => self.threads.len(),
            Screen::ThreadView => self.posts.len(),
        };
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
