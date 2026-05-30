use crate::api;
use crate::types::{AppState, Board, BoardListItem, Post, Screen, ThreadItem};
use std::collections::HashMap;

impl AppState {
    fn map_selected_index(&self) -> usize {
        if self.screen == Screen::BoardList {
            self.board_list_mapped_index()
                .unwrap_or(0)
        } else if self.search_active && !self.search_matches.is_empty() {
            self.search_matches[self.selected_index.min(self.search_matches.len() - 1)]
        } else {
            self.selected_index
        }
    }

    fn item_count(&self) -> usize {
        if self.screen == Screen::BoardList {
            self.board_list_items().len()
        } else if self.search_active && !self.search_matches.is_empty() {
            self.search_matches.len()
        } else {
            match self.screen {
                Screen::BoardList => unreachable!(),
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
                .filter(|(_, b)| b.name.to_lowercase().contains(&q) || b.category.to_lowercase().contains(&q))
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
            let idx = self.search_matches[self.selected_index.min(self.search_matches.len() - 1)];
            if self.screen == Screen::BoardList {
                let items = self.board_list_items();
                self.selected_index = items.iter().position(|item| {
                    matches!(item, BoardListItem::Board(i) if *i == idx)
                }).unwrap_or(0);
            } else {
                self.selected_index = idx;
            }
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

    fn config_path() -> std::path::PathBuf {
        Self::data_path().join("config")
    }

    pub fn load_config(&mut self) {
        let path = Self::config_path();
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return,
        };
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(eq) = line.find('=') {
                let key = line[..eq].trim();
                let val = line[eq + 1..].trim();
                match key {
                    "show_images" => self.show_images = val != "false",
                    _ => {}
                }
            }
        }
    }

    pub fn save_config(&self) {
        let dir = Self::data_path();
        let _ = std::fs::create_dir_all(&dir);
        let path = Self::config_path();
        let content = format!("show_images={}\n", if self.show_images { "true" } else { "false" });
        let _ = std::fs::write(&path, content);
    }

    fn data_path() -> std::path::PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let mut path = std::path::PathBuf::from(home);
        path.push(".config");
        path.push("tano");
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
        path.push("tano");
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
        if self.boards.is_empty() || self.selected_is_category_header() {
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
            b_fav.cmp(&a_fav)
                .then(a.category.cmp(&b.category))
                .then(a.name.cmp(&b.name))
        });
        self.search_matches = self.compute_matches();
    }

    pub fn board_list_items(&self) -> Vec<BoardListItem> {
        let mut items: Vec<BoardListItem> = Vec::new();
        let indices: Vec<usize> = if self.search_active && !self.search_matches.is_empty() {
            self.search_matches.clone()
        } else {
            (0..self.boards.len()).collect()
        };

        let mut cat_indices: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut cat_order: Vec<&str> = Vec::new();
        for &i in &indices {
            let cat = self.boards[i].category.as_str();
            if !cat_indices.contains_key(cat) {
                cat_order.push(cat);
            }
            cat_indices.entry(cat).or_default().push(i);
        }

        for cat in cat_order {
            let cat_indices = &cat_indices[cat];
            items.push(BoardListItem::CategoryHeader {
                name: cat.to_string(),
                board_count: cat_indices.len(),
            });
            if !self.collapsed_categories.contains(cat) {
                for &i in cat_indices {
                    items.push(BoardListItem::Board(i));
                }
            }
        }
        items
    }

    fn board_list_mapped_index(&self) -> Option<usize> {
        let items = self.board_list_items();
        if self.selected_index >= items.len() {
            return None;
        }
        match items[self.selected_index] {
            BoardListItem::Board(i) => Some(i),
            BoardListItem::CategoryHeader { .. } => None,
        }
    }

    pub fn selected_is_category_header(&self) -> bool {
        if self.search_active && !self.search_matches.is_empty() {
            return false;
        }
        let items = self.board_list_items();
        if self.selected_index >= items.len() {
            return false;
        }
        matches!(items[self.selected_index], BoardListItem::CategoryHeader { .. })
    }

    pub fn toggle_category(&mut self) {
        let items = self.board_list_items();
        if self.selected_index >= items.len() {
            return;
        }
        if let BoardListItem::CategoryHeader { ref name, .. } = items[self.selected_index] {
            if !self.collapsed_categories.insert(name.clone()) {
                self.collapsed_categories.remove(name);
            }
        }
    }

    fn boards_cache_path() -> std::path::PathBuf {
        Self::favorites_path().join("boards")
    }

    fn collapse_all_categories(&mut self) {
        self.collapsed_categories = self.boards.iter().map(|b| b.category.clone()).collect();
    }

    fn load_boards_from_cache(&mut self) -> bool {
        let path = Self::boards_cache_path();
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let mut boards = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() >= 3 {
                let url = parts[0].to_string();
                let name = parts[1].to_string();
                let category = parts[2].to_string();
                boards.push(Board { name, url, category });
            } else if let Some(sep) = line.find('|') {
                // backward compat: url|name (with [cat] prefix in name)
                let url = line[..sep].to_string();
                let name = line[sep + 1..].to_string();
                let (cat_name, pure_name) = parse_category_from_name(&name);
                boards.push(Board { name: pure_name, url, category: cat_name });
            }
        }
        if boards.is_empty() {
            return false;
        }
        self.boards = boards;
        self.collapse_all_categories();
        self.sort_boards_with_favorites();
        self.status_message = format!("{} 個の板を読み込みました", self.boards.len());
        true
    }

    fn save_boards_cache(&self) {
        let path = Self::boards_cache_path();
        let _ = std::fs::create_dir_all(Self::favorites_path());
        let content: String = self
            .boards
            .iter()
            .map(|b| format!("{}|{}|{}", b.url, b.name, b.category))
            .collect::<Vec<_>>()
            .join("\n");
        let _ = std::fs::write(&path, content);
    }

    pub fn load_boards(&mut self) {
        if self.load_boards_from_cache() {
            return;
        }
        self.fetch_boards_from_server();
    }

    pub fn reload_boards(&mut self) {
        self.fetch_boards_from_server();
    }

    fn fetch_boards_from_server(&mut self) {
        self.loading = true;
        self.status_message = "板一覧を読み込み中...".to_string();
        match api::fetch_boards() {
            Ok(boards) => {
                self.boards = boards;
                self.collapse_all_categories();
                self.sort_boards_with_favorites();
                self.save_boards_cache();
                self.status_message = format!("{} 個の板を読み込みました", self.boards.len());
            }
            Err(e) => {
                self.status_message = format!("エラー: {}", e);
            }
        }
        self.loading = false;
    }

    pub fn select_board(&mut self) {
        if self.boards.is_empty() || self.selected_is_category_header() {
            return;
        }
        let idx = self.map_selected_index();
        let board = &self.boards[idx];
        self.current_board = Some(board.name.clone());
        self.current_board_url = Some(board.url.clone());
        self.load_threads();
    }

    fn threads_cache_path(board_url: &str) -> std::path::PathBuf {
        let host = board_url.trim_end_matches('/').split('/').nth(2).unwrap_or("unknown_host");
        let board = board_url.trim_end_matches('/').rsplit('/').next().unwrap_or("unknown_board");
        Self::favorites_path().join("threads").join(host).join(board)
    }

    fn load_threads_from_cache(&mut self) -> bool {
        let url = match self.current_board_url {
            Some(ref url) => url.clone(),
            None => return false,
        };
        let path = Self::threads_cache_path(&url);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let mut threads = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(sep) = line.find('|') {
                let id = line[..sep].to_string();
                let rest = &line[sep + 1..];
                if let Some(sep2) = rest.rfind('|') {
                    let title = rest[..sep2].to_string();
                    let count: u32 = rest[sep2 + 1..].parse().unwrap_or(0);
                    threads.push(ThreadItem { id, title, post_count: count });
                }
            }
        }
        if threads.is_empty() {
            return false;
        }
        self.threads = threads;
        self.sort_threads_with_read();
        self.status_message = format!("{} 個のスレを読み込みました", self.threads.len());
        true
    }

    fn save_threads_cache(&self) {
        let url = match self.current_board_url {
            Some(ref url) => url,
            None => return,
        };
        let path = Self::threads_cache_path(url);
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let content: String = self
            .threads
            .iter()
            .map(|t| format!("{}|{}|{}", t.id, t.title, t.post_count))
            .collect::<Vec<_>>()
            .join("\n");
        let _ = std::fs::write(&path, content);
    }

    pub fn load_threads(&mut self) {
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.list_offset = 0;
        if self.load_threads_from_cache() {
            self.screen = Screen::ThreadList;
            return;
        }
        self.fetch_threads_from_server();
    }

    pub fn reload_threads(&mut self) {
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.list_offset = 0;
        self.fetch_threads_from_server();
    }

    fn fetch_threads_from_server(&mut self) {
        self.loading = true;
        self.status_message = "スレ一覧を読み込み中...".to_string();

        if let Some(ref url) = self.current_board_url {
            match api::fetch_threads(url) {
                Ok(threads) => {
                    self.threads = threads;
                    self.sort_threads_with_read();
                    self.save_threads_cache();
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

    fn posts_cache_path(board_url: &str, thread_id: &str) -> std::path::PathBuf {
        let host = board_url.trim_end_matches('/').split('/').nth(2).unwrap_or("unknown_host");
        let board = board_url.trim_end_matches('/').rsplit('/').next().unwrap_or("unknown_board");
        Self::favorites_path().join("posts").join(host).join(board).join(thread_id)
    }

    fn load_posts_from_cache(&mut self) -> bool {
        let url = match self.current_board_url {
            Some(ref url) => url.clone(),
            None => return false,
        };
        let tid = match self.current_thread_id {
            Some(ref id) => id.clone(),
            None => return false,
        };
        let path = Self::posts_cache_path(&url, &tid);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let mut posts = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split("<>").collect();
            if parts.len() >= 4 {
                let name = parts[0].trim().to_string();
                let email = parts[1].trim().to_string();
                let date_id = parts[2].trim().to_string();
                let body = parts[3..].join("<>");
                let body = body.replace("<br>", "\n");
                let (date, id) = if let Some(idx) = date_id.find(" ID:") {
                    let d = date_id[..idx].trim().to_string();
                    let i = date_id[idx + 4..].trim().to_string();
                    (d, Some(i))
                } else {
                    (date_id, None)
                };
                posts.push(Post { name, email, date, body, id });
            }
        }
        if posts.is_empty() {
            return false;
        }
        let count = posts.len();
        self.posts = posts;
        self.thread_info = self
            .current_thread_title
            .as_ref()
            .map(|t| format!("{} ({}レス)", t, count));
        self.status_message = format!("{} レスを読み込みました", count);
        true
    }

    fn save_posts_cache(&self) {
        let url = match self.current_board_url {
            Some(ref url) => url,
            None => return,
        };
        let tid = match self.current_thread_id {
            Some(ref id) => id,
            None => return,
        };
        let path = Self::posts_cache_path(url, tid);
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let content: String = self
            .posts
            .iter()
            .map(|p| {
                let id_str = match &p.id {
                    Some(id) => format!(" ID:{}", id),
                    None => String::new(),
                };
                let body = p.body.replace('\n', "<br>");
                format!("{}<>{}<>{}<>{}{}", p.name, p.email, p.date, id_str, body)
            })
            .collect::<Vec<_>>()
            .join("\n");
        let _ = std::fs::write(&path, content);
    }

    pub fn load_posts(&mut self, thread_id: &str) {
        self.current_thread_id = Some(thread_id.to_string());
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.list_offset = 0;
        if self.load_posts_from_cache() {
            self.screen = Screen::ThreadView;
            return;
        }
        self.fetch_posts_from_server();
    }

    pub fn reload_posts(&mut self) {
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.list_offset = 0;
        self.fetch_posts_from_server();
    }

    fn fetch_posts_from_server(&mut self) {
        let tid = match self.current_thread_id {
            Some(ref id) => id.clone(),
            None => return,
        };
        self.loading = true;
        self.status_message = "レスを読み込み中...".to_string();
        if let Some(ref url) = self.current_board_url {
            match api::fetch_posts(url, &tid) {
                Ok(posts) => {
                    let count = posts.len();
                    self.posts = posts;
                    self.save_posts_cache();
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

    fn calc_scroll_offset(&self, post_index: usize) -> usize {
        let mut offset = 0;
        for i in 0..post_index.min(self.posts.len()) {
            offset += 3 + self.posts[i].body.lines().count();
        }
        offset
    }

    pub fn follow_reference(&mut self) {
        if self.screen != Screen::ThreadView || self.posts.is_empty() {
            return;
        }
        use regex_lite::Regex;
        use std::sync::OnceLock;
        static REF_RE: OnceLock<Regex> = OnceLock::new();
        //let re = REF_RE.get_or_init(|| Regex::new(r">>(\d+)").unwrap());
        let re = REF_RE.get_or_init(|| Regex::new(r"&gt;&gt;(\d+)").unwrap());

        let mut line_count = 0;
        for (_, post) in self.posts.iter().enumerate() {
            let post_lines = 3 + post.body.lines().count();
            if line_count + post_lines > self.scroll_offset {
                for body_line in post.body.lines() {
                    if let Some(cap) = re.captures(body_line) {
                        let target: usize = cap[1].parse().unwrap_or(0);
                        if target > 0 && target <= self.posts.len() {
                            self.scroll_offset = self.calc_scroll_offset(target - 1);
                            self.status_message = format!("#{} にジャンプしました", target);
                            return;
                        } else {
                            self.status_message =
                                format!("#{} は存在しません (全{}レス)", target, self.posts.len());
                            return;
                        }
                    }
                }
                self.status_message = "このレスに参照はありません".to_string();
                return;
            }
            line_count += post_lines;
        }
        self.status_message = "このレスに参照はありません".to_string();
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
        match self.screen {
            Screen::BoardList | Screen::ThreadList => {
                self.selected_index = 0;
                self.list_offset = 0;
            }
            Screen::ThreadView => {
                self.scroll_offset = 0;
            }
            Screen::Compose => {}
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        match self.screen {
            Screen::BoardList | Screen::ThreadList => {
                let len = self.item_count();
                if len > 0 {
                    self.selected_index = len - 1;
                    if self.visible_items > 0 {
                        self.list_offset = self.selected_index.saturating_sub(self.visible_items - 1);
                    }
                }
            }
            Screen::ThreadView => {
                let total_lines: usize = self.posts.iter().map(|p| p.body.lines().count() + 3).sum();
                self.scroll_offset = total_lines.saturating_sub(1);
            }
            Screen::Compose => {}
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

fn parse_category_from_name(name: &str) -> (String, String) {
    if let Some(end) = name.find(']') {
        if name.starts_with('[') {
            let cat = name[1..end].trim();
            let rest = name[end + 1..].trim();
            return (cat.to_string(), rest.to_string());
        }
    }
    (String::new(), name.to_string())
}
