//! The Companion app as a web app: its web build, embedded gzipped (`build.rs`), served on the
//! same address as the API, so that a phone or tablet without the app opens
//! `http://<computer>:47811` in its browser and has all of it - signing on, the duty, the
//! navigator, the messages, the IBIS and the ticket printer. The page pairs with the code as
//! the app does; nothing it serves needs a key, and nothing but the build is reachable.

use crate::http::Response;
use std::io::Read;

include!(concat!(env!("OUT_DIR"), "/web_files.rs"));

/// What a browser asking for `path` gets: the file, the page for the app's own routes
/// (`/duty`, `/pair?code=…`: a path without an extension), else none.
pub fn answer(path: &str, gzip_ok: bool) -> Option<Response> {
    let wanted = if path == "/" { "/index.html" } else { path };
    let found = FILES.iter().find(|(p, _)| *p == wanted);
    let last = wanted.rsplit('/').next().unwrap_or("");
    let (path, gz) = match found {
        Some((p, b)) => (*p, *b),
        None if !last.contains('.') => FILES.iter().find(|(p, _)| *p == "/index.html").map(|(p, b)| (*p, *b)).unwrap_or(("/index.html", &[])),
        None => return None,
    };
    if gz.is_empty() {
        return Some(missing());
    }
    let mut r = if gzip_ok {
        let mut r = Response::new(200, ctype(path), gz.to_vec());
        r.encoding = Some("gzip");
        r
    } else {
        // (a client that takes no gzip - no browser - gets it unpacked)
        let mut body = Vec::new();
        flate2::read::GzDecoder::new(gz).read_to_end(&mut body).ok()?;
        Response::new(200, ctype(path), body)
    };
    // the build's own files carry their content's hash in their name and never change; the
    // page and the rest are asked again when the add-on was updated
    r.cache = if path.starts_with("/_expo/static/") || path.starts_with("/assets/") {
        "public, max-age=31536000, immutable"
    } else if path == "/index.html" {
        "no-cache"
    } else {
        "public, max-age=3600"
    };
    Some(r)
}

/// Whether this build has the web app.
pub fn present() -> bool {
    FILES.iter().any(|(p, _)| *p == "/index.html")
}

/// The page of a build without the web app (a developer's, before `tools/companion-web.mjs`).
fn missing() -> Response {
    Response::new(503, "text/html; charset=utf-8", "<!doctype html><meta charset=utf-8><meta name=viewport content=\"width=device-width\"><title>Omsi-Hub</title><body style=\"font:16px system-ui;background:#0d1424;color:#e8eefc;padding:24px\"><h1>Omsi-Hub</h1><p>This build of Omsi-Hub has no web app. Use the Omsi-Hub Companion app.</p>")
}

fn ctype(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "wasm" => "application/wasm",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_app_s_routes_are_its_page_and_only_its_files_are_served() {
        if !present() {
            // (a build without the web app: every page says so)
            assert_eq!(answer("/", true).unwrap().status, 503);
            return;
        }
        let page = answer("/", true).unwrap();
        assert_eq!((page.status, page.ctype, page.encoding, page.cache), (200, "text/html; charset=utf-8", Some("gzip"), "no-cache"));
        // the app's own routes are the page, which routes them itself
        assert_eq!(answer("/pair", true).unwrap().body, page.body);
        assert_eq!(answer("/duty", true).unwrap().body, page.body);
        // unpacked for a client without gzip
        let plain = answer("/index.html", false).unwrap();
        assert!(plain.encoding.is_none() && String::from_utf8(plain.body).unwrap().contains("<div id=\"root\">"));
        let wasm = answer("/canvaskit.wasm", true).unwrap();
        assert_eq!(wasm.ctype, "application/wasm");
        assert!(answer("/settings.json", true).is_none());
        assert!(answer("/nothing.js", true).is_none());
    }
}
