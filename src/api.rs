use crate::types::{Board, Post, ThreadItem};
use cookie_store::CookieStore as CookieStoreStruct;
use encoding_rs::SHIFT_JIS;
use regex_lite::Regex;
use reqwest::cookie::CookieStore;
use reqwest::header::HeaderValue;
use reqwest::Url;
use std::io::Write;
use std::sync::{Arc, OnceLock, RwLock};
use scraper::{Html, Selector};

fn urlencode_sjis(s: &str) -> Vec<u8> {
    let (encoded, _, _) = SHIFT_JIS.encode(s);
    let encoded = encoded.into_owned();
    let mut result = Vec::with_capacity(encoded.len());
    for &byte in &encoded {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte);
            }
            b' ' => result.push(b'+'),
            _ => {
                result.push(b'%');
                result.push(hex_char(byte >> 4));
                result.push(hex_char(byte & 0x0F));
            }
        }
    }
    result
}

fn hex_char(v: u8) -> u8 {
    match v {
        0..=9 => b'0' + v,
        _ => b'A' + v - 10,
    }
}

const BBSMENU_URL: &str = "https://menu.5ch.io/bbsmenu.html";

fn cookie_jar_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let mut path = std::path::PathBuf::from(home);
    path.push(".config");
    path.push("tano");
    path.push("cookies.json");
    path
}

struct PersistentJar {
    store: RwLock<CookieStoreStruct>,
    path: std::path::PathBuf,
}

impl PersistentJar {
    fn load(path: std::path::PathBuf) -> Self {
        let store = if path.exists() {
            std::fs::File::open(&path)
                .ok()
                .and_then(|f| {
                    let reader = std::io::BufReader::new(f);
                    cookie_store::serde::json::load(reader).ok()
                })
                .unwrap_or_default()
        } else {
            CookieStoreStruct::default()
        };
        PersistentJar { store: RwLock::new(store), path }
    }

    fn save(&self) {
        let snapshot = self.store.read().ok().map(|s| s.clone());
        if let Some(store) = snapshot {
            if let Some(dir) = self.path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(mut file) = std::fs::File::create(&self.path) {
                let _ = cookie_store::serde::json::save(&store, &mut file);
            }
        }
    }
}

impl CookieStore for PersistentJar {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        let iter = cookie_headers.filter_map(|val| {
            val.to_str()
                .ok()
                .and_then(|s| cookie_store::RawCookie::parse(s).ok())
                .map(|c: cookie_store::RawCookie<'_>| c.into_owned())
        });
        if let Ok(mut store) = self.store.write() {
            store.store_response_cookies(iter, url);
        }
        self.save();
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        let store = self.store.read().ok()?;
        let s: String = store
            .get_request_values(url)
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; ");
        if s.is_empty() {
            return None;
        }
        HeaderValue::from_str(&s).ok()
    }
}

fn cookie_jar() -> &'static Arc<PersistentJar> {
    static JAR: OnceLock<Arc<PersistentJar>> = OnceLock::new();
    JAR.get_or_init(|| Arc::new(PersistentJar::load(cookie_jar_path())))
}

pub fn save_cookies() {
    cookie_jar().save();
}

fn fetch_client() -> &'static reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64)")
            .cookie_provider(cookie_jar().clone())
            .build()
            .expect("Failed to create HTTP client")
    })
}

fn post_client() -> &'static reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    //.redirect(reqwest::redirect::Policy::none())
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:151.0) Gecko/20100101 Firefox/151.0")
            .cookie_provider(cookie_jar().clone())
            .build()
            .expect("Failed to create HTTP client")
    })
}

fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    let resp = fetch_client()
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

pub fn post_message(board_url: &str, thread_id: &str, name: &str, email: &str, message: &str) -> Result<(), String> {
    let host = board_url
        .trim_end_matches('/')
        .split('/')
        .nth(2)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Invalid board URL".to_string())?;

    let board_name = board_url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Invalid board URL".to_string())?;

    let url = format!("https://{}/test/bbs.cgi?guid=ON", host);

    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let pairs = [
        ("FROM", name),
        ("mail", email),
        ("MESSAGE", message),
        ("bbs", board_name),
        ("key", thread_id),
        ("time", &time.to_string()),
        ("submit", "書き込む"),
        ("oekaki_thread1", ""),
    ];

    let body = encode_body(&pairs);

    let referer = format!("https://{}/test/bbs.cgi", host);

    let resp = post_with_headers(&url, &host, &referer, body, false)?;

    let status = resp.status();
    /*
    if status.is_redirection() || status.is_success() {
        return Ok(());
    }
    */
    if ! status.is_success() {
        let bytes = resp.bytes().map_err(|e| format!("レスポンス読み取りエラー: {}", e))?;
        let text = decode_sjis(&bytes);
        let error_msg = extract_error(&text, status);
        return Err(error_msg);
    }

    let bytes = resp.bytes().map_err(|e| format!("レスポンス読み取りエラー: {}", e))?;
    let text = decode_sjis(&bytes);
    let feature = extract_feature(&text);

    if let Some(fv) = feature {
        let mut pairs_with_feature: Vec<(&str, &str)> = pairs.to_vec();
        pairs_with_feature.push(("feature", &fv));
        let body = encode_body(&pairs_with_feature);
        let resp = post_with_headers(&url, &host, &referer, body, true)?;
        let status = resp.status();
        if status.is_redirection() || status.is_success() {
            return Ok(());
        }
        let bytes = resp.bytes().map_err(|e| format!("レスポンス読み取りエラー: {}", e))?;
        let text = decode_sjis(&bytes);
        let error_msg = extract_error(&text, status);
        return Err(error_msg);
    }

    let error_msg = extract_error(&text, status);
    Err(error_msg)
}

fn encode_body(pairs: &[(&str, &str)]) -> Vec<u8> {
    let mut body = Vec::new();
    for (i, (key, value)) in pairs.iter().enumerate() {
        if i > 0 {
            body.push(b'&');
        }
        body.extend_from_slice(key.as_bytes());
        body.push(b'=');
        body.extend_from_slice(&urlencode_sjis(value));
    }
    body
}

fn post_with_headers(url: &str, host: &str, referer: &str, body: Vec<u8>, repost: bool) -> Result<reqwest::blocking::Response, String> {
    let resp = post_client()
        .post(url)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "ja,en-US;q=0.9,en;q=0.8")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Origin", &format!("https://{}", host))
        .header("Priority", "u=0, i")
        .header("Referer", referer)
        .header("Sec-Fetch-Dest", "document")
        .header("Sec-Fetch-Mode", "navigate")
        .header("Sec-Fetch-Site", "same-origin")
        .header("Sec-Fetch-User", "?1")
        .header("TE", "trailers")
        .header("Upgrade-Insecure-Requests", "1")
        .body(body)
        .send()
        .map_err(|e| format!("送信エラー: {}", e))?;

    // debug
    {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/tano.log")
        {
            let status = resp.status();
            let _ = writeln!(f, "=== Response Headers (repost={}) ===", repost);
            let _ = writeln!(f, "status = {}", status);
            for (key, value) in resp.headers() {
                let _ = writeln!(f, "{}: {}", key, value.to_str().unwrap_or("<non-utf8>"));
            }
        }
    }

    // debug
    {
        let url4cookie = Url::parse(url).unwrap();
        if let Some(c) = cookie_jar().cookies(&url4cookie) {
            println!("{:?}", c);
        }
    }

    Ok(resp)
}

/*
fn extract_feature(html: &str) -> Option<String> {
    let re = Regex::new(r#"<input\s+type="hidden"\s+name="feature"\s+value="([^"]*)"#).ok()?;
    re.captures(html)?.get(1).map(|m| m.as_str().to_string())
}
*/

fn extract_feature(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(r#"input[name="feature"]"#).ok()?;

    document
        .select(&selector)
        .next()?
        .value()
        .attr("value")
        .map(str::to_string)
}

fn extract_error(text: &str, status: reqwest::StatusCode) -> String {
    text.lines()
        .find(|l| l.contains("ERROR") || l.contains("エラー"))
        .map(|l| l.chars().take(100).collect())
        .unwrap_or_else(|| format!("HTTP error: {}", status))
}
