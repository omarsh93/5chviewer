#[derive(Debug, Clone)]
pub struct Board {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct ThreadItem {
    pub id: String,
    pub title: String,
    pub post_count: u32,
}

#[derive(Debug, Clone)]
pub struct Post {
    pub name: String,
    pub email: String,
    pub date: String,
    pub body: String,
    pub id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    BoardList,
    ThreadList,
    ThreadView,
    Compose,
}

use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct AppState {
    pub screen: Screen,
    pub boards: Vec<Board>,
    pub threads: Vec<ThreadItem>,
    pub posts: Vec<Post>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub current_board: Option<String>,
    pub current_board_url: Option<String>,
    pub current_thread_title: Option<String>,
    pub current_thread_id: Option<String>,
    pub loading: bool,
    pub status_message: String,
    pub thread_info: Option<String>,
    pub search_query: String,
    pub search_active: bool,
    pub search_matches: Vec<usize>,
    pub list_offset: usize,
    pub visible_items: usize,
    pub compose_name: String,
    pub compose_email: String,
    pub compose_message: String,
    pub compose_focus: usize,
    pub favorites: HashSet<String>,
    pub read_threads: HashSet<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            screen: Screen::BoardList,
            boards: Vec::new(),
            threads: Vec::new(),
            posts: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            list_offset: 0,
            visible_items: 0,
            current_board: None,
            current_board_url: None,
            current_thread_title: None,
            current_thread_id: None,
            loading: false,
            status_message: String::new(),
            thread_info: None,
            search_query: String::new(),
            search_active: false,
            search_matches: Vec::new(),
            compose_name: String::new(),
            compose_email: "sage".to_string(),
            compose_message: String::new(),
            compose_focus: 0,
            favorites: HashSet::new(),
            read_threads: HashSet::new(),
        }
    }
}
