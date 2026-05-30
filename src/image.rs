use std::fs;
use std::path::PathBuf;

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

pub fn download_image(url: &str) -> Option<Vec<u8>> {
    let dir = cache_dir();
    let path = dir.join(url_to_filename(url));
    if path.exists() {
        return fs::read(&path).ok();
    }
    let response = reqwest::blocking::get(url).ok()?;
    let bytes = response.bytes().ok()?;
    fs::write(&path, &bytes).ok()?;
    Some(bytes.to_vec())
}
