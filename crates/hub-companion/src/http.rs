//! The little HTTP the companion's server speaks (ported from the old game's companion): one
//! request per connection (`Connection: close`), a request head of 8 KiB at most and a body of
//! 4 KiB at most - what the app sends fits in a few hundred bytes, and an unbounded stream is a
//! way to fill the launcher's memory. Parsing, routing and the commands a device may send are
//! here, apart from the sockets, so that they can be tested as they are.

use serde_json::{json, Value};

/// Longest request head (request line and headers).
pub const MAX_HEAD: usize = 8 * 1024;
/// Longest request body.
pub const MAX_BODY: usize = 4 * 1024;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Request {
    pub method: String,
    /// The path, percent-decoded, without the query.
    pub path: String,
    pub query: Vec<(String, String)>,
    /// Header names in lower case.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    pub fn query(&self, name: &str) -> Option<&str> {
        self.query.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// The device key the request carries (`Authorization: Bearer <key>`).
    pub fn key(&self) -> &str {
        self.header("authorization").and_then(|v| v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer "))).map(str::trim).unwrap_or("")
    }

    /// The body as JSON (`null` when it is none).
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpError {
    /// The head is not complete yet: read more.
    Incomplete,
    TooLarge,
    Bad,
}

/// Parse a request head from the bytes read so far: the request and where its body begins.
pub fn parse_head(bytes: &[u8]) -> Result<(Request, usize), HttpError> {
    let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") else {
        return Err(if bytes.len() > MAX_HEAD { HttpError::TooLarge } else { HttpError::Incomplete });
    };
    if end > MAX_HEAD {
        return Err(HttpError::TooLarge);
    }
    let head = std::str::from_utf8(&bytes[..end]).map_err(|_| HttpError::Bad)?;
    let mut lines = head.split("\r\n");
    let mut first = lines.next().ok_or(HttpError::Bad)?.split(' ');
    let method = first.next().filter(|m| !m.is_empty() && m.bytes().all(|b| b.is_ascii_uppercase())).ok_or(HttpError::Bad)?.to_string();
    let target = first.next().filter(|t| t.starts_with('/')).ok_or(HttpError::Bad)?;
    if !first.next().is_some_and(|v| v.starts_with("HTTP/1.")) {
        return Err(HttpError::Bad);
    }
    let (raw_path, raw_query) = target.split_once('?').unwrap_or((target, ""));
    let path = percent_decode(raw_path).ok_or(HttpError::Bad)?;
    if path.contains("..") || path.contains('\\') || path.contains('\0') {
        return Err(HttpError::Bad);
    }
    let query = raw_query
        .split('&')
        .filter(|kv| !kv.is_empty())
        .filter_map(|kv| {
            let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
            Some((percent_decode(&k.replace('+', " "))?, percent_decode(&v.replace('+', " "))?))
        })
        .collect();
    let headers = lines.filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string())).collect();
    Ok((Request { method, path, query, headers, body: Vec::new() }, end + 4))
}

/// How long the body is, by `Content-Length` (none: 0). Too long a body is an error.
pub fn body_length(r: &Request) -> Result<usize, HttpError> {
    match r.header("content-length") {
        None => Ok(0),
        Some(v) => match v.parse::<usize>() {
            Ok(n) if n <= MAX_BODY => Ok(n),
            Ok(_) => Err(HttpError::TooLarge),
            Err(_) => Err(HttpError::Bad),
        },
    }
}

fn percent_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Where a request goes.
#[derive(Debug, Clone, PartialEq)]
pub enum Route {
    /// `GET /api/hello`: what the server is. No key needed.
    Hello,
    /// `POST /api/pair`: a pairing code for a device key. No key needed.
    Pair,
    /// `GET /api/state?after=<version>`: the state, once it is newer than `after` (or after a
    /// while without news).
    State(Option<u64>),
    /// `POST /api/do`: a [`Command`] as JSON.
    Do,
    /// `GET /api/route`: the trip's route as a line and its stops.
    Route,
    /// `GET /api/pos?after=<n>`: where the bus is, the places after the `n`-th (or the latest),
    /// once there is one (or after a short while without).
    Pos(Option<u64>),
    /// `GET /api/roads?x=&y=&r=`: the map's roads within `r` metres of a place.
    Roads {
        x: f64,
        y: f64,
        r: f64,
    },
    /// `GET /api/lines`: the map's lines and tours, for taking a duty from the phone.
    Lines,
    /// `GET /api/device/<id>`: a device's screen, its form (`crate::devices`).
    Device(String),
    /// `GET /api/texture/<id>`: a picture of a form, as a PNG.
    Texture(String),
    /// `GET /api/screen/<id>?after=<v>`: a screen's stand, once it is newer than `after` (or
    /// after a short while without news).
    Screen(String, Option<u64>),
    /// A browser's question before a request of another origin (the app's web build).
    Preflight,
    /// `GET /…` but `/api/`: the web app's page or one of its files (`crate::web`). No key
    /// needed - the page pairs with the code first.
    Web(String),
    NotFound,
    /// A known path with the wrong method.
    Method,
}

impl Route {
    /// Whether the route wants a paired device's key.
    pub fn needs_key(&self) -> bool {
        matches!(self, Route::State(_) | Route::Pos(_) | Route::Do | Route::Route | Route::Roads { .. } | Route::Lines | Route::Device(_) | Route::Texture(_) | Route::Screen(..))
    }
}

pub fn route(r: &Request) -> Route {
    if r.method == "OPTIONS" {
        return Route::Preflight;
    }
    let get = r.method == "GET" || r.method == "HEAD";
    let post = r.method == "POST";
    let wants = |ok: bool, to: Route| if ok { to } else { Route::Method };
    match r.path.as_str() {
        "/api/hello" => wants(get, Route::Hello),
        "/api/pair" => wants(post, Route::Pair),
        "/api/state" => wants(get, Route::State(r.query("after").and_then(|v| v.parse::<u64>().ok()))),
        "/api/do" => wants(post, Route::Do),
        "/api/route" => wants(get, Route::Route),
        "/api/pos" => wants(get, Route::Pos(r.query("after").and_then(|v| v.parse::<u64>().ok()))),
        "/api/lines" => wants(get, Route::Lines),
        "/api/roads" => {
            let f = |name: &str| r.query(name).and_then(|v| v.parse::<f64>().ok()).filter(|v| v.is_finite());
            // (a place on a map: within a few thousand kilometres of its origin)
            let (Some(x), Some(y)) = (f("x").filter(|v| v.abs() < 1e7), f("y").filter(|v| v.abs() < 1e7)) else { return Route::NotFound };
            let radius = f("r").unwrap_or(1000.0).clamp(100.0, 2000.0);
            wants(get, Route::Roads { x, y, r: radius })
        }
        p => {
            // (an id of a form or a picture: 20 hex characters, never a path)
            let id = |prefix: &str| p.strip_prefix(prefix).filter(|id| crate::devices::id_ok(id)).map(str::to_string);
            if let Some(id) = id("/api/device/") {
                wants(get, Route::Device(id))
            } else if let Some(id) = id("/api/texture/") {
                wants(get, Route::Texture(id))
            } else if let Some(id) = id("/api/screen/") {
                wants(get, Route::Screen(id, r.query("after").and_then(|v| v.parse::<u64>().ok())))
            } else if !p.starts_with("/api/") && p != "/api" {
                wants(get, Route::Web(p.to_string()))
            } else {
                Route::NotFound
            }
        }
    }
}

/// A key of the bus's (a trigger's name): letters, digits and `_`, as the scripts write them.
pub fn key_ok(k: &str) -> bool {
    (1..=64).contains(&k.len()) && k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// A line's or a tour's name of the timetable: printable, short.
fn name_ok(s: &str) -> bool {
    (1..=32).contains(&s.chars().count()) && !s.chars().any(char::is_control)
}

/// What a device may ask for. A fixed list, and nothing else gets past the parser: no file,
/// no profile, no setting is reachable from the network.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    SignOn {
        number: String,
        code: String,
    },
    AcceptOrder,
    DriveFree,
    TakeDuty {
        line: String,
        tour: String,
        trip: Option<u32>,
    },
    TypeIbis,
    IbisKey {
        key: String,
    },
    IbisLine {
        line: String,
    },
    PrinterKey {
        key: String,
    },
    Ticket {
        i: u32,
    },
    MsgRead {
        id: u64,
    },
    MsgAnswer {
        id: u64,
        key: String,
    },
    MsgQuick {
        key: String,
    },
    PauseEnd,
    /// A tap or key of one of the bus's devices: pressed and let go, or held (`down: true`)
    /// and let go (`down: false`).
    Press {
        trigger: String,
        down: Option<bool>,
    },
}

impl Command {
    pub fn from_json(v: &Value) -> Option<Command> {
        let s = |k: &str| v.get(k).and_then(Value::as_str);
        let digits = |x: &str| (1..=12).contains(&x.len()) && x.bytes().all(|b| b.is_ascii_digit());
        let id = || v.get("id").and_then(Value::as_u64);
        Some(match s("do")? {
            "sign_on" => Command::SignOn { number: s("number").filter(|x| digits(x))?.to_string(), code: s("code").filter(|x| digits(x))?.to_string() },
            "accept_order" => Command::AcceptOrder,
            "drive_free" => Command::DriveFree,
            "take_duty" => {
                let trip = match v.get("trip") {
                    None | Some(Value::Null) => None,
                    Some(t) => Some(t.as_u64().filter(|t| (1..1000).contains(t))? as u32),
                };
                Command::TakeDuty { line: s("line").filter(|x| name_ok(x))?.to_string(), tour: s("tour").filter(|x| name_ok(x))?.to_string(), trip }
            }
            "type_ibis" => Command::TypeIbis,
            "ibis_key" => Command::IbisKey { key: s("key").filter(|k| key_ok(k))?.to_string() },
            "ibis_line" => Command::IbisLine { line: s("line").filter(|x| (1..=8).contains(&x.len()) && x.bytes().all(|b| b.is_ascii_alphanumeric()))?.to_string() },
            "printer_key" => Command::PrinterKey { key: s("key").filter(|k| key_ok(k))?.to_string() },
            "ticket" => Command::Ticket { i: v.get("i").and_then(Value::as_u64).filter(|i| (1..=99).contains(i))? as u32 },
            "msg_read" => Command::MsgRead { id: id()? },
            "msg_answer" => Command::MsgAnswer { id: id()?, key: s("key").filter(|k| key_ok(k))?.to_string() },
            "msg_quick" => Command::MsgQuick { key: s("key").filter(|k| key_ok(k))?.to_string() },
            "pause_end" => Command::PauseEnd,
            "press" => Command::Press {
                trigger: s("trigger").filter(|k| key_ok(k))?.to_string(),
                down: match v.get("down") {
                    None | Some(Value::Null) => None,
                    Some(d) => Some(d.as_bool()?),
                },
            },
            _ => return None,
        })
    }

    /// The command as the plugin reads it from `data/companion.json` (None: the add-on's own,
    /// nothing for the game).
    pub fn for_plugin(&self) -> Option<Value> {
        Some(match self {
            Command::SignOn { number, code } => json!({ "do": "sign_on", "number": number, "code": code }),
            Command::AcceptOrder => json!({ "do": "accept_order" }),
            Command::DriveFree => json!({ "do": "drive_free" }),
            Command::TakeDuty { line, tour, trip } => json!({ "do": "take_duty", "line": line, "tour": tour, "trip": trip }),
            Command::TypeIbis => json!({ "do": "type_ibis" }),
            Command::IbisKey { key } => json!({ "do": "key", "key": key }),
            Command::IbisLine { line } => json!({ "do": "ibis_line", "line": line }),
            Command::PrinterKey { key } => json!({ "do": "key", "key": key }),
            Command::Ticket { i } => json!({ "do": "ticket", "i": i }),
            Command::MsgRead { id } => json!({ "do": "msg", "kind": "read", "id": id }),
            Command::MsgAnswer { id, key } => match key.as_str() {
                "read" => json!({ "do": "msg", "kind": "read", "id": id }),
                "accept" | "decline" => json!({ "do": "msg", "kind": key, "id": id }),
                _ => json!({ "do": "msg", "kind": "reply", "id": id, "key": key }),
            },
            Command::MsgQuick { key } => json!({ "do": "msg", "kind": "quick", "key": key }),
            Command::PauseEnd => return None,
            Command::Press { trigger, down } => json!({ "do": "press", "key": trigger, "down": down }),
        })
    }
}

/// An answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub status: u16,
    pub ctype: &'static str,
    pub body: Vec<u8>,
    /// `Cache-Control`: nothing kept, but for the web app's files (`web`).
    pub cache: &'static str,
    /// `Content-Encoding` of the body (the web app's files are kept gzipped).
    pub encoding: Option<&'static str>,
}

impl Response {
    pub fn new(status: u16, ctype: &'static str, body: impl Into<Vec<u8>>) -> Response {
        Response { status, ctype, body: body.into(), cache: "no-store", encoding: None }
    }

    pub fn json(v: &Value) -> Response {
        Response::new(200, "application/json; charset=utf-8", v.to_string())
    }

    /// An error as JSON: `{error}` with its status.
    pub fn error(status: u16, error: &str) -> Response {
        Response { status, ..Response::json(&json!({ "error": error })) }
    }

    /// The whole answer as it goes on the wire; `head_only` for HEAD. Every answer lets the
    /// app's web build read it (CORS): the key, not the origin, is what admits a device.
    pub fn to_bytes(&self, head_only: bool) -> Vec<u8> {
        let encoding = self.encoding.map(|e| format!("Content-Encoding: {e}\r\nVary: Accept-Encoding\r\n")).unwrap_or_default();
        let out = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n{encoding}Cache-Control: {}\r\nX-Content-Type-Options: nosniff\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Authorization, Content-Type\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Max-Age: 600\r\nConnection: close\r\n\r\n",
            self.status,
            reason(self.status),
            self.ctype,
            self.body.len(),
            self.cache
        );
        let mut bytes = out.into_bytes();
        if !head_only {
            bytes.extend_from_slice(&self.body);
        }
        bytes
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        503 => "Service Unavailable",
        _ => "Error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(text: &str) -> Request {
        parse_head(text.as_bytes()).unwrap().0
    }

    #[test]
    fn a_request_head_is_read_with_its_query_and_headers() {
        let r = req("GET /api/state?after=12 HTTP/1.1\r\nHost: 192.168.1.20\r\nAuthorization: Bearer abc\r\n\r\n");
        assert_eq!((r.method.as_str(), r.path.as_str()), ("GET", "/api/state"));
        assert_eq!(r.key(), "abc");
        assert_eq!(route(&r), Route::State(Some(12)));
        assert_eq!(route(&req("GET /api/state HTTP/1.1\r\n\r\n")), Route::State(None));
        assert_eq!(req("GET / HTTP/1.1\r\n\r\n").key(), "");
        assert_eq!(parse_head(b"GET / HTTP/1.1\r\nHost: x\r\n"), Err(HttpError::Incomplete));
        assert_eq!(parse_head(&vec![b'a'; MAX_HEAD + 1]), Err(HttpError::TooLarge));
        assert_eq!(parse_head(b"get / HTTP/1.1\r\n\r\n"), Err(HttpError::Bad));
        assert_eq!(parse_head(b"GET /../x HTTP/1.1\r\n\r\n"), Err(HttpError::Bad));
        assert_eq!(parse_head(b"GET /%2e%2e/x HTTP/1.1\r\n\r\n"), Err(HttpError::Bad));
    }

    #[test]
    fn routes_answer_only_their_own_method_and_want_a_key_where_it_counts() {
        assert_eq!(route(&req("GET /api/hello HTTP/1.1\r\n\r\n")), Route::Hello);
        assert_eq!(route(&req("GET /api/pair HTTP/1.1\r\n\r\n")), Route::Method);
        assert_eq!(route(&req("OPTIONS /api/do HTTP/1.1\r\n\r\n")), Route::Preflight);
        assert_eq!(route(&req("GET /api/settings.json HTTP/1.1\r\n\r\n")), Route::NotFound);
        assert_eq!(route(&req("GET /pair?code=123456 HTTP/1.1\r\n\r\n")), Route::Web("/pair".into()));
        assert_eq!(route(&req("POST / HTTP/1.1\r\n\r\n")), Route::Method);
        assert!(!route(&req("GET / HTTP/1.1\r\n\r\n")).needs_key());
        assert_eq!(route(&req("GET /api/roads?x=-120.5&y=3000 HTTP/1.1\r\n\r\n")), Route::Roads { x: -120.5, y: 3000.0, r: 1000.0 });
        assert_eq!(route(&req("GET /api/roads?x=1&y=2&r=99999 HTTP/1.1\r\n\r\n")), Route::Roads { x: 1.0, y: 2.0, r: 2000.0 });
        assert_eq!(route(&req("GET /api/roads?x=NaN&y=1 HTTP/1.1\r\n\r\n")), Route::NotFound);
        assert_eq!(route(&req("GET /api/device/0123456789abcdef0123 HTTP/1.1\r\n\r\n")), Route::Device("0123456789abcdef0123".into()));
        assert_eq!(route(&req("GET /api/screen/0123456789abcdef0123?after=4 HTTP/1.1\r\n\r\n")), Route::Screen("0123456789abcdef0123".into(), Some(4)));
        assert_eq!(route(&req("GET /api/texture/zzzzzzzzzzzzzzzzzzzz HTTP/1.1\r\n\r\n")), Route::NotFound);
        assert_eq!(route(&req("GET /api/texture/0123 HTTP/1.1\r\n\r\n")), Route::NotFound);
        assert_eq!(route(&req("POST /api/texture/0123456789abcdef0123 HTTP/1.1\r\n\r\n")), Route::Method);
        for p in [
            "GET /api/state",
            "POST /api/do",
            "GET /api/route",
            "GET /api/roads?x=0&y=0",
            "GET /api/lines",
            "GET /api/device/0123456789abcdef0123",
            "GET /api/texture/0123456789abcdef0123",
            "GET /api/screen/0123456789abcdef0123",
        ] {
            assert!(route(&req(&format!("{p} HTTP/1.1\r\n\r\n"))).needs_key(), "{p}");
        }
        assert!(!route(&req("POST /api/pair HTTP/1.1\r\n\r\n")).needs_key());
        assert!(!route(&req("GET /api/hello HTTP/1.1\r\n\r\n")).needs_key());
    }

    #[test]
    fn the_body_has_a_limit() {
        assert_eq!(body_length(&req("POST /api/do HTTP/1.1\r\nContent-Length: 5000\r\n\r\n")), Err(HttpError::TooLarge));
        assert_eq!(body_length(&req("POST /api/do HTTP/1.1\r\nContent-Length: 12\r\n\r\n")), Ok(12));
    }

    #[test]
    fn only_the_listed_commands_get_through() {
        let c = |t: &str| Command::from_json(&serde_json::from_str(t).unwrap());
        assert_eq!(c(r#"{"do":"sign_on","number":"482913","code":"5821"}"#), Some(Command::SignOn { number: "482913".into(), code: "5821".into() }));
        assert_eq!(c(r#"{"do":"sign_on","number":"48a913","code":"1"}"#), None);
        assert_eq!(c(r#"{"do":"ibis_key","key":"IBIS_7"}"#), Some(Command::IbisKey { key: "IBIS_7".into() }));
        assert_eq!(c(r#"{"do":"ibis_key","key":"../x"}"#), None);
        assert_eq!(c(r#"{"do":"ticket","i":0}"#), None);
        assert_eq!(c(r#"{"do":"take_duty","line":"M29","tour":"3"}"#), Some(Command::TakeDuty { line: "M29".into(), tour: "3".into(), trip: None }));
        assert_eq!(c(r#"{"do":"take_duty","line":"M29","tour":"3","trip":-1}"#), None);
        assert_eq!(c(r#"{"do":"msg_answer","id":77,"key":"accept"}"#).unwrap().for_plugin().unwrap(), json!({ "do": "msg", "kind": "accept", "id": 77 }));
        assert_eq!(c(r#"{"do":"msg_answer","id":77,"key":"late"}"#).unwrap().for_plugin().unwrap()["kind"], "reply");
        assert_eq!(c(r#"{"do":"pause_end"}"#).unwrap().for_plugin(), None);
        assert_eq!(c(r#"{"do":"press","trigger":"IBIS_7"}"#).unwrap().for_plugin().unwrap(), json!({ "do": "press", "key": "IBIS_7", "down": null }));
        assert_eq!(c(r#"{"do":"press","trigger":"almex_riegel","down":true}"#), Some(Command::Press { trigger: "almex_riegel".into(), down: Some(true) }));
        assert_eq!(c(r#"{"do":"press","trigger":"x y"}"#), None);
        assert_eq!(c(r#"{"do":"press","trigger":"IBIS_7","down":"yes"}"#), None);
        assert_eq!(c(r#"{"do":"format_disk"}"#), None);
        assert_eq!(c(r#"{"number":"1"}"#), None);
    }

    #[test]
    fn an_answer_lets_the_web_build_read_it() {
        let text = String::from_utf8(Response::error(401, "not_paired").to_bytes(false)).unwrap();
        assert!(text.starts_with("HTTP/1.1 401 Unauthorized\r\n"));
        assert!(text.contains("Access-Control-Allow-Origin: *\r\n"));
        assert!(text.ends_with("\r\n\r\n{\"error\":\"not_paired\"}"));
        assert!(String::from_utf8(Response::json(&json!({})).to_bytes(true)).unwrap().ends_with("\r\n\r\n"));
    }
}
