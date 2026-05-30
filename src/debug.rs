use std::fs::OpenOptions;
use std::io::Write;

const LOGFILE: &str = "/tmp/tano.log";

fn open_log() -> Option<std::fs::File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOGFILE)
        .ok()
}

pub fn log(msg: &str) {
    if let Some(mut f) = open_log() {
        let _ = writeln!(f, "{}", msg);
    }
}

pub fn response_headers(
    resp: &reqwest::blocking::Response,
    repost: bool,
) {
    if let Some(mut f) = open_log() {
        let _ = writeln!(
            f,
            "=== Response Headers (repost={}) ===",
            repost
        );

        let _ = writeln!(f, "status = {}", resp.status());

        for (key, value) in resp.headers() {
            let _ = writeln!(
                f,
                "{}: {}",
                key,
                value.to_str().unwrap_or("<non-utf8>")
            );
        }

        let _ = writeln!(f);
    }
}

pub fn cookies(cookie_header: Option<&str>) {
    if let Some(mut f) = open_log() {
        let _ = writeln!(f, "=== Cookies ===");

        match cookie_header {
            Some(v) => {
                let _ = writeln!(f, "{}", v);
            }
            None => {
                let _ = writeln!(f, "<none>");
            }
        }

        let _ = writeln!(f);
    }
}

pub fn body(text: &str) {
    if let Some(mut f) = open_log() {
        let _ = writeln!(f, "=== Body ===");
        let _ = writeln!(f, "{}", text);
        let _ = writeln!(f);
    }
}

pub fn separator() {
    if let Some(mut f) = open_log() {
        let _ = writeln!(
            f,
            "------------------------------------------------------------"
        );
    }
}
