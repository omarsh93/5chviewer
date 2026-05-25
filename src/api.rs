use crate::types::{Board, Post, ThreadItem};
use encoding_rs::SHIFT_JIS;
use regex_lite::Regex;

const BBSMENU_URL: &str = "https://menu.5ch.io/bbsmenu.html";

fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64)")
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client
        .get(url)
        .send()
        .map_err(|e| format!("HTTP request error: {}", e))?;

    let bytes = resp
        .bytes()
        .map_err(|e| format!("Body read error: {}", e))?
        .to_vec();

    Ok(bytes)
}

fn decode_sjis(bytes: &[u8]) -> String {
    let (decoded, _, _had_errors) = SHIFT_JIS.decode(bytes);
    decoded.to_string()
}

pub fn fetch_boards() -> Result<Vec<Board>, String> {
    let bytes = fetch_bytes(BBSMENU_URL)?;
    let html_str = decode_sjis(&bytes);

    // Parse categories and boards from bbsmenu.html
    // Format: <br><b>カテゴリ名</b><br>
    // <a href=URL>板名</a><br>
    let cat_re = Regex::new(r"(?i)<br>\s*<b>(.*?)</b>\s*<br>").unwrap();
    //let board_re = Regex::new(r#"(?i)<a\s+href="(https?://[^"]+\.5ch\.io/[^"]*)"[^>]*>(.*?)</a>"#).unwrap();
    let board_re = Regex::new(r#"(?i)<a\s+href=(https?://[0-9a-z]+\.5ch\.io/[0-9a-z]+/)>([^<]*?)</a>"#).unwrap();

    // Find all category positions
    let mut cat_positions: Vec<(usize, String)> = cat_re
        .captures_iter(&html_str)
        .map(|c| {
            let pos = c.get(0).unwrap().end();
            let name = c[1].trim().to_string();
            (pos, name)
        })
        .collect();

    if cat_positions.is_empty() {
        // Fallback: just scrape all board links
        let mut boards: Vec<Board> = board_re
            .captures_iter(&html_str)
            .filter(|c| {
                let name = c[2].trim();
                !name.is_empty() && !name.starts_with('[')
            })
            .map(|c| Board {
                name: c[2].trim().to_string(),
                url: c[1].to_string(),
            })
            .collect();

        boards.sort_by(|a, b| a.name.cmp(&b.name));
        boards.dedup_by(|a, b| a.url == b.url);
        return Ok(boards);
    }

    // Sentinel for end of string
    cat_positions.push((html_str.len(), String::new()));

    let mut boards = Vec::new();
    for i in 0..cat_positions.len() - 1 {
        let category = &cat_positions[i].1;
        let start = cat_positions[i].0;
        let end = cat_positions[i + 1].0;
        let section = &html_str[start..end];

        for cap in board_re.captures_iter(section) {
            let name = cap[2].trim().to_string();
            let url = cap[1].to_string();
            if !name.is_empty() && !name.starts_with('[') {
                boards.push(Board {
                    name: format!("[{}] {}", category, name),
                    url,
                });
            }
        }
    }

    if boards.is_empty() {
        // Ultimate fallback
        for cap in board_re.captures_iter(&html_str) {
            let name = cap[2].trim().to_string();
            let url = cap[1].to_string();
            if !name.is_empty() && !name.starts_with('[') {
                boards.push(Board { name, url });
            }
        }
    }

    boards.sort_by(|a, b| a.name.cmp(&b.name));
    boards.dedup_by(|a, b| a.url == b.url);

    Ok(boards)
}

pub fn fetch_threads(board_url: &str) -> Result<Vec<ThreadItem>, String> {
    let base = board_url.trim_end_matches('/');
    let url = format!("{}/subject.txt", base);

    let bytes = fetch_bytes(&url)?;
    let text = decode_sjis(&bytes);

    let mut threads = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Format: "1234567890.dat.l50 タイトル (123)"
        // Format: "1234567890.dat<> タイトル (123)"
        if let Some((key, rest)) = line.split_once(".dat<>") {
            let rest = rest.trim();
            if let Some((title, count_str)) = rest.rsplit_once(" (") {
                let count_str = count_str.trim_end_matches(')');
                let count: u32 = count_str.parse().unwrap_or(0);
                threads.push(ThreadItem {
                    id: key.to_string(),
                    title: title.trim().to_string(),
                    post_count: count,
                });
            }
        }
    }

    // Newest first
    threads.reverse();

    Ok(threads)
}

pub fn fetch_posts(board_url: &str, thread_id: &str) -> Result<Vec<Post>, String> {
    let base = board_url.trim_end_matches('/');
    let url = format!("{}/dat/{}.dat", base, thread_id);

    let bytes = fetch_bytes(&url)?;
    let text = decode_sjis(&bytes);

    let mut posts = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // DAT format: name<>email<>date ID:xxx<>body<>...
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

            posts.push(Post {
                name,
                email,
                date,
                body,
                id,
            });
        }
    }

    Ok(posts)
}
