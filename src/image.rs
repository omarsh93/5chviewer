use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp"];

fn cache_dir() -> PathBuf {
    let base = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".cache")
        });
    let dir = base.join("tano").join("images");
    fs::create_dir_all(&dir).ok();
    dir
}

fn url_to_filename(url: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:016x}.img", hasher.finish())
}

pub fn find_image_url(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return None;
    }
    let lower = trimmed.to_lowercase();
    for ext in IMAGE_EXTENSIONS {
        if lower.contains(&format!(".{}", ext)) {
            let cleaned = trimmed
                .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '-' && c != '_' && c != '.');
            return Some(cleaned.to_string());
        }
    }
    None
}

pub fn download_image(url: &str) -> Option<PathBuf> {
    let dir = cache_dir();
    let path = dir.join(url_to_filename(url));
    if path.exists() {
        return Some(path);
    }
    let response = reqwest::blocking::get(url).ok()?;
    let bytes = response.bytes().ok()?;
    fs::write(&path, &bytes).ok()?;
    Some(path)
}

#[allow(dead_code)]
pub fn is_available() -> bool {
    Command::new("chafa").arg("--version").output().is_ok()
}

pub fn render_image(path: &Path, max_width: u16) -> Option<Vec<Line<'static>>> {
    let width = max_width.clamp(10, 80);
    let height = (width as f32 * 0.5) as u16;
    let output = Command::new("chafa")
        .args([
            "--symbols",
            "block",
            "--colors",
            "240",
            "--size",
            &format!("{}x{}", width, height),
            "--optimize",
            "0",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(parse_ansi(&stdout))
}

fn parse_ansi(output: &str) -> Vec<Line<'static>> {
    let mut result = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let mut fg: Option<Color> = None;
    let mut bg: Option<Color> = None;
    let mut rev = false;

    let mut chars = output.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.next() == Some('[') {
            let mut param = String::new();
            let final_byte = loop {
                match chars.next() {
                    Some(b @ ('m' | 'h' | 'l')) => break b,
                    Some(ch) => param.push(ch),
                    None => break 'm',
                }
            };
            flush_span(&mut spans, &mut buf, make_style(fg, bg, rev));
            if final_byte == 'm' {
                apply_sgr(&mut fg, &mut bg, &mut rev, &param);
            }
        } else if c == '\n' {
            flush_span(&mut spans, &mut buf, make_style(fg, bg, rev));
            if spans.is_empty() {
                result.push(Line::from(""));
            } else {
                result.push(Line::from(std::mem::take(&mut spans)));
            }
        } else {
            buf.push(c);
        }
    }
    flush_span(&mut spans, &mut buf, make_style(fg, bg, rev));
    if !spans.is_empty() {
        result.push(Line::from(spans));
    }
    result
}

fn flush_span(spans: &mut Vec<Span<'static>>, buf: &mut String, style: Style) {
    if !buf.is_empty() {
        spans.push(Span::styled(std::mem::take(buf), style));
    }
}

fn make_style(fg: Option<Color>, bg: Option<Color>, rev: bool) -> Style {
    let mut s = Style::default();
    if let Some(c) = fg {
        s = s.fg(c);
    }
    if let Some(c) = bg {
        s = s.bg(c);
    }
    if rev {
        s = s.reversed();
    }
    s
}

fn apply_sgr(fg: &mut Option<Color>, bg: &mut Option<Color>, rev: &mut bool, params: &str) {
    if params.is_empty() || params == "0" {
        *fg = None;
        *bg = None;
        *rev = false;
        return;
    }
    let parts: Vec<&str> = params.split(';').collect();
    let mut i = 0;
    while i < parts.len() {
        match parts[i] {
            "0" => {
                *fg = None;
                *bg = None;
                *rev = false;
            }
            "7" => {
                *rev = true;
            }
            "27" => {
                *rev = false;
            }
            "38" => {
                i += 1;
                if i < parts.len() {
                    match parts[i] {
                        "5" => {
                            i += 1;
                            if i < parts.len() {
                                *fg = Some(Color::Indexed(parts[i].parse().unwrap_or(0)));
                            }
                        }
                        "2" => {
                            i += 1;
                            let r = parts.get(i).and_then(|v| v.parse().ok()).unwrap_or(0);
                            i += 1;
                            let g = parts.get(i).and_then(|v| v.parse().ok()).unwrap_or(0);
                            i += 1;
                            let b = parts.get(i).and_then(|v| v.parse().ok()).unwrap_or(0);
                            *fg = Some(Color::Rgb(r, g, b));
                        }
                        _ => {}
                    }
                }
            }
            "48" => {
                i += 1;
                if i < parts.len() {
                    match parts[i] {
                        "5" => {
                            i += 1;
                            if i < parts.len() {
                                *bg = Some(Color::Indexed(parts[i].parse().unwrap_or(0)));
                            }
                        }
                        "2" => {
                            i += 1;
                            let r = parts.get(i).and_then(|v| v.parse().ok()).unwrap_or(0);
                            i += 1;
                            let g = parts.get(i).and_then(|v| v.parse().ok()).unwrap_or(0);
                            i += 1;
                            let b = parts.get(i).and_then(|v| v.parse().ok()).unwrap_or(0);
                            *bg = Some(Color::Rgb(r, g, b));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
}
