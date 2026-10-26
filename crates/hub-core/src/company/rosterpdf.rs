//! RosterFlow's prints: the bus graph and the duty graph as a PDF file (Optibus's "Bus Graph":
//! a row per bus or duty on an hour axis, its trips as bars in the line's colour with the
//! stops' first letters and the minutes over their ends, the depot runs black, the duties
//! under the bus as a grey band with their numbers), and the duty cards (a British depot's:
//! two cards to an A4 sheet across, every stop with its time, the timing points marked, in the
//! company's colours and with its logo). A small PDF writer of its own: pages of rectangles,
//! polygons, lines, pictures and words in the Helvetica every PDF reader has.

/// A picture placed on the pages: `w` by `h` pixels, `components` samples a pixel (1 grey, 3
/// RGB) stored as `filter` says ("FlateDecode", "DCTDecode"), and its transparency (one 8-bit
/// sample a pixel, Flate-compressed) where it has one.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub w: u32,
    pub h: u32,
    pub components: u8,
    pub filter: &'static str,
    pub data: Vec<u8>,
    pub alpha: Option<Vec<u8>>,
}

/// A PDF being written: its pages' drawing, in points from the top left.
pub struct Pdf {
    w: f32,
    h: f32,
    pages: Vec<String>,
    images: Vec<Image>,
}

fn rgb(c: [u8; 3]) -> String {
    format!("{:.3} {:.3} {:.3}", c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0)
}

/// A text as a PDF string in WinAnsi: Latin-1's letters as they are, others a "?".
fn pdf_string(s: &str) -> Vec<u8> {
    let mut out = vec![b'('];
    for ch in s.chars() {
        let b = match ch {
            '\u{20AC}' => 0x80,
            '\u{2013}' => 0x96,
            '\u{2014}' => 0x97,
            '\u{2192}' => b'>',
            c if (c as u32) < 0x20 => b' ',
            c if (c as u32) < 0x100 => c as u32 as u8,
            _ => b'?',
        };
        if matches!(b, b'(' | b')' | b'\\') {
            out.push(b'\\');
        }
        out.push(b);
    }
    out.push(b')');
    out
}

impl Pdf {
    /// A PDF of pages `w` by `h` points (A4 across: 842 by 595), its first page begun.
    pub fn new(w: f32, h: f32) -> Pdf {
        Pdf { w, h, pages: vec![String::new()], images: Vec::new() }
    }

    /// A picture the pages may show (`picture`): its number.
    pub fn add_image(&mut self, img: Image) -> usize {
        self.images.push(img);
        self.images.len() - 1
    }

    /// Picture `k` filling `w` by `h` points from `x`, `y`.
    pub fn picture(&mut self, k: usize, x: f32, y: f32, w: f32, h: f32) {
        let yy = self.y(y + h);
        let s = format!("q {w:.2} 0 0 {h:.2} {x:.2} {yy:.2} cm /Im{k} Do Q\n");
        self.out().push_str(&s);
    }

    pub fn new_page(&mut self) {
        self.pages.push(String::new());
    }

    pub fn pages(&self) -> usize {
        self.pages.len()
    }

    fn out(&mut self) -> &mut String {
        self.pages.last_mut().unwrap()
    }

    fn y(&self, y: f32) -> f32 {
        self.h - y
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, fill: [u8; 3]) {
        let yy = self.y(y + h);
        let s = format!("{} rg {x:.2} {yy:.2} {w:.2} {h:.2} re f\n", rgb(fill));
        self.out().push_str(&s);
    }

    pub fn frame(&mut self, x: f32, y: f32, w: f32, h: f32, width: f32, c: [u8; 3]) {
        let yy = self.y(y + h);
        let s = format!("{} RG {width:.2} w {x:.2} {yy:.2} {w:.2} {h:.2} re S\n", rgb(c));
        self.out().push_str(&s);
    }

    pub fn line(&mut self, a: (f32, f32), b: (f32, f32), width: f32, c: [u8; 3]) {
        let (ay, by) = (self.y(a.1), self.y(b.1));
        let s = format!("{} RG {width:.2} w {:.2} {ay:.2} m {:.2} {by:.2} l S\n", rgb(c), a.0, b.0);
        self.out().push_str(&s);
    }

    /// A filled polygon, outlined in `edge` if given.
    pub fn poly(&mut self, pts: &[(f32, f32)], fill: [u8; 3], edge: Option<[u8; 3]>) {
        if pts.len() < 3 {
            return;
        }
        let mut s = format!("{} rg ", rgb(fill));
        if let Some(e) = edge {
            s.push_str(&format!("{} RG 0.4 w ", rgb(e)));
        }
        for (k, (x, y)) in pts.iter().enumerate() {
            s.push_str(&format!("{x:.2} {:.2} {} ", self.y(*y), if k == 0 { "m" } else { "l" }));
        }
        s.push_str(if edge.is_some() { "h B\n" } else { "h f\n" });
        self.out().push_str(&s);
    }

    /// How wide a text is set (Helvetica's average widths: near enough to centre it).
    pub fn width(s: &str, size: f32, bold: bool) -> f32 {
        let per: f32 = s
            .chars()
            .map(|c| match c {
                'i' | 'l' | 'j' | '.' | ',' | ':' | ';' | '!' | '|' | '\'' | ' ' => 0.28,
                'f' | 't' | 'r' | 'I' | '(' | ')' | '-' => 0.36,
                'm' | 'w' | 'M' | 'W' => 0.83,
                c if c.is_ascii_uppercase() => 0.68,
                c if c.is_ascii_digit() => 0.556,
                _ => 0.54,
            })
            .sum();
        per * size * if bold { 1.06 } else { 1.0 }
    }

    /// Words with their baseline at `y`; `align` 0 left, 1 centred, 2 right of `x`.
    pub fn text(&mut self, x: f32, y: f32, size: f32, bold: bool, s: &str, c: [u8; 3], align: u8) {
        let w = Self::width(s, size, bold);
        let x = match align {
            1 => x - w * 0.5,
            2 => x - w,
            _ => x,
        };
        let yy = self.y(y);
        let mut b = format!("BT /{} {size:.1} Tf {} rg {x:.2} {yy:.2} Td ", if bold { "F2" } else { "F1" }, rgb(c)).into_bytes();
        b.extend(pdf_string(s));
        b.extend(b" Tj ET\n");
        // (the page's drawing is kept as a string: Latin-1 bytes stored as their chars)
        let s: String = b.iter().map(|&x| x as char).collect();
        self.out().push_str(&s);
    }

    /// The file.
    pub fn finish(&self) -> Vec<u8> {
        let mut out: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
        let mut offsets: Vec<usize> = Vec::new();
        let n = self.pages.len();
        // objects: 1 catalog, 2 pages, 3 F1, 4 F2, the pictures (and their transparency), then
        // a page and its contents each
        let obj = |out: &mut Vec<u8>, offsets: &mut Vec<usize>, body: Vec<u8>| {
            offsets.push(out.len());
            out.extend(format!("{} 0 obj\n", offsets.len()).into_bytes());
            out.extend(body);
            out.extend(b"\nendobj\n");
        };
        let stream = |dict: String, data: &[u8]| {
            let mut body = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
            body.extend(data);
            body.extend(b"\nendstream");
            body
        };
        let mut image_ids = Vec::new();
        let mut next = 5;
        for img in &self.images {
            image_ids.push(next);
            next += 1 + img.alpha.is_some() as usize;
        }
        let first_page = next;
        obj(&mut out, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
        let kids: Vec<String> = (0..n).map(|k| format!("{} 0 R", first_page + 2 * k)).collect();
        obj(&mut out, &mut offsets, format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).into_bytes());
        obj(&mut out, &mut offsets, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec());
        obj(&mut out, &mut offsets, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_vec());
        for (img, id) in self.images.iter().zip(&image_ids) {
            let space = if img.components == 1 { "/DeviceGray" } else { "/DeviceRGB" };
            let filter = if img.filter.is_empty() { String::new() } else { format!(" /Filter /{}", img.filter) };
            let mask = if img.alpha.is_some() { format!(" /SMask {} 0 R", id + 1) } else { String::new() };
            let dict = format!("/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace {space} /BitsPerComponent 8{filter}{mask}", img.w, img.h);
            obj(&mut out, &mut offsets, stream(dict, &img.data));
            if let Some(a) = &img.alpha {
                let dict = format!("/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode", img.w, img.h);
                obj(&mut out, &mut offsets, stream(dict, a));
            }
        }
        let xobjects: String = image_ids.iter().enumerate().map(|(k, id)| format!("/Im{k} {id} 0 R ")).collect();
        let xobjects = if xobjects.is_empty() { String::new() } else { format!(" /XObject << {xobjects}>>") };
        for (k, page) in self.pages.iter().enumerate() {
            obj(
                &mut out,
                &mut offsets,
                format!(
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.0} {:.0}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >>{xobjects} >> /Contents {} 0 R >>",
                    self.w,
                    self.h,
                    first_page + 1 + 2 * k
                )
                .into_bytes(),
            );
            let bytes: Vec<u8> = page.chars().map(|c| c as u32 as u8).collect();
            let mut body = format!("<< /Length {} >>\nstream\n", bytes.len()).into_bytes();
            body.extend(bytes);
            body.extend(b"\nendstream");
            obj(&mut out, &mut offsets, body);
        }
        let xref = out.len();
        out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).into_bytes());
        for o in &offsets {
            out.extend(format!("{o:010} 00000 n \n").into_bytes());
        }
        out.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", offsets.len() + 1).into_bytes());
        out
    }
}

// --- the graphs ----------------------------------------------------------------------------------

/// What a graph's row shows on its axis.
#[derive(Clone, Debug, PartialEq)]
pub enum ItemKind {
    /// A trip with passengers, pointed the way it runs (`inbound`: to the left).
    Trip { inbound: bool },
    /// A depot run (black, "P" over it).
    Pull,
    /// An empty run between places.
    Deadhead,
    /// Signing on or off.
    Sign,
    /// Travel to a relief point (its letter).
    Travel,
    /// Custom work.
    Custom,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphItem {
    pub from: i32,
    pub to: i32,
    pub kind: ItemKind,
    pub label: String,
    pub colour: [u8; 3],
    /// Over its ends: the stop it leaves (two letters) and the minute, the stop it reaches.
    pub start_stop: String,
    pub end_stop: String,
}

/// A row: a bus or a duty, what it runs, and the band under it (a bus's duties, a duty's buses).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GraphRow {
    pub name: String,
    pub sub: String,
    pub sub2: String,
    pub items: Vec<GraphItem>,
    pub under: Vec<(i32, i32, String)>,
}

const INK: [u8; 3] = [20, 20, 20];
const GREY: [u8; 3] = [120, 120, 120];
const RULE: [u8; 3] = [205, 205, 205];

/// The hour's label on the axis.
fn hour(h: i32) -> String {
    format!("{:02}", h.rem_euclid(24))
}

/// A graph (Optibus's bus graph or duty graph) as a PDF: `title` and the words after it
/// (`info`), when it was printed, and its rows, as many pages as they need (A4 across).
pub fn graph(title: &str, info: &[String], printed: &str, page_word: &str, rows: &[GraphRow]) -> Vec<u8> {
    let (pw, ph) = (842.0f32, 595.0f32);
    let mut pdf = Pdf::new(pw, ph);
    let t0 = rows.iter().flat_map(|r| r.items.iter().map(|i| i.from)).min().unwrap_or(300).div_euclid(60) * 60;
    let t1 = ((rows.iter().flat_map(|r| r.items.iter().map(|i| i.to)).max().unwrap_or(1380) + 59).div_euclid(60) * 60).max(t0 + 120);
    let (left, right) = (78.0f32, pw - 22.0);
    let x_of = |m: i32| left + (m - t0) as f32 / (t1 - t0) as f32 * (right - left);
    let row_h = 46.0f32;
    let top = 74.0f32;
    let per_page = (((ph - top - 40.0) / row_h).floor() as usize).max(1);
    let pages = rows.len().div_ceil(per_page).max(1);
    for page in 0..pages {
        if page > 0 {
            pdf.new_page();
        }
        pdf.text(22.0, 34.0, 20.0, true, title, INK, 0);
        let mut x = 22.0 + Pdf::width(title, 20.0, true) + 18.0;
        for w in info {
            pdf.text(x, 34.0, 12.0, false, w, INK, 0);
            x += Pdf::width(w, 12.0, false) + 16.0;
        }
        pdf.text(right, 22.0, 9.0, false, &format!("{page_word}: {}/{pages}", page + 1), INK, 2);
        pdf.text(right, 34.0, 9.0, false, printed, INK, 2);
        let shown = &rows[page * per_page..((page + 1) * per_page).min(rows.len())];
        let bottom = top + shown.len() as f32 * row_h;
        // the hours, on top and under the rows, and their lines down
        for h in (t0 / 60)..=(t1 / 60) {
            let xh = x_of(h * 60);
            pdf.text(xh, top - 6.0, 10.0, false, &hour(h), INK, 1);
            pdf.text(xh, bottom + 14.0, 10.0, false, &hour(h), INK, 1);
            pdf.line((xh, top - 2.0), (xh, bottom), 0.3, RULE);
            if h * 60 + 30 < t1 {
                let xm = x_of(h * 60 + 30);
                pdf.line((xm, top), (xm, bottom), 0.2, [232, 232, 232]);
            }
        }
        for (k, r) in shown.iter().enumerate() {
            let y = top + k as f32 * row_h;
            pdf.line((22.0, y + row_h - 6.0), (right, y + row_h - 6.0), 0.3, RULE);
            pdf.text(48.0, y + 18.0, 11.0, false, &r.name, INK, 1);
            pdf.text(48.0, y + 27.0, 6.0, false, &r.sub, INK, 1);
            pdf.text(48.0, y + 34.0, 6.0, false, &r.sub2, INK, 1);
            let bar_y = y + 22.0;
            let bar_h = 6.5;
            // (the last trip's end: where, and at which stop)
            let mut last_end: Option<(f32, String)> = None;
            for it in &r.items {
                let (a, b) = (x_of(it.from), x_of(it.to).max(x_of(it.from) + 1.0));
                match it.kind {
                    ItemKind::Trip { inbound } => {
                        let tip = 3.0f32.min((b - a) * 0.4);
                        let pts = if inbound {
                            vec![(a, bar_y + bar_h * 0.5), (a + tip, bar_y), (b, bar_y), (b, bar_y + bar_h), (a + tip, bar_y + bar_h)]
                        } else {
                            vec![(a, bar_y), (b - tip, bar_y), (b, bar_y + bar_h * 0.5), (b - tip, bar_y + bar_h), (a, bar_y + bar_h)]
                        };
                        pdf.poly(&pts, it.colour, Some([60, 60, 60]));
                        if Pdf::width(&it.label, 5.5, false) < b - a + 8.0 {
                            pdf.text((a + b) * 0.5, bar_y - 1.2, 5.5, false, &it.label, INK, 1);
                        }
                    }
                    ItemKind::Pull => {
                        pdf.rect(a, bar_y, b - a, bar_h, [0, 0, 0]);
                        pdf.text((a + b) * 0.5, bar_y - 1.2, 5.5, false, "P", INK, 1);
                    }
                    ItemKind::Deadhead => pdf.rect(a, bar_y + bar_h * 0.35, b - a, bar_h * 0.3, [90, 90, 90]),
                    ItemKind::Sign => pdf.line((a, bar_y + bar_h * 0.5), (b, bar_y + bar_h * 0.5), 0.8, GREY),
                    ItemKind::Travel | ItemKind::Custom => {
                        pdf.rect(a, bar_y, b - a, bar_h, [250, 250, 250]);
                        pdf.frame(a, bar_y, b - a, bar_h, 0.5, if it.kind == ItemKind::Travel { [214, 150, 20] } else { GREY });
                        pdf.text((a + b) * 0.5, bar_y + bar_h - 1.4, 5.0, true, &it.label, INK, 1);
                    }
                }
                // the stops' letters and the minutes over the trip's ends
                // (as Optibus prints them: "Re 55 | 00" - the stop once where one trip ends and
                // the next begins, the minutes either side of the line)
                if matches!(it.kind, ItemKind::Trip { .. }) {
                    pdf.line((a, bar_y - 7.0), (a, bar_y), 0.3, INK);
                    let joined = last_end.as_ref().is_some_and(|(x, s)| *s == it.start_stop && a - x < 14.0);
                    if !joined {
                        pdf.text(a + 0.8, bar_y - 13.0, 4.8, false, &it.start_stop, INK, 0);
                    }
                    pdf.text(a + 0.8, bar_y - 8.0, 4.8, false, &format!("{:02}", it.from.rem_euclid(60)), INK, 0);
                    pdf.line((b, bar_y - 7.0), (b, bar_y), 0.3, INK);
                    pdf.text(b - 0.8, bar_y - 13.0, 4.8, false, &it.end_stop, INK, 2);
                    pdf.text(b - 0.8, bar_y - 8.0, 4.8, false, &format!("{:02}", it.to.rem_euclid(60)), INK, 2);
                    last_end = Some((b, it.end_stop.clone()));
                }
            }
            // the band under it
            for (k2, (a, b, label)) in r.under.iter().enumerate() {
                let (xa, xb) = (x_of(*a), x_of(*b));
                pdf.rect(xa, bar_y + bar_h + 1.5, (xb - xa).max(1.0), 3.0, if k2 % 2 == 0 { [118, 118, 118] } else { [228, 228, 228] });
                pdf.frame(xa, bar_y + bar_h + 1.5, (xb - xa).max(1.0), 3.0, 0.3, GREY);
                pdf.text(xa + 1.0, bar_y + bar_h + 10.5, 5.0, false, label, GREY, 0);
            }
        }
    }
    pdf.finish()
}

// --- the duty cards ------------------------------------------------------------------------------

/// The company's look on a print: its name, its two colours (the monogram's; the first the
/// cards' marks), its logo picture where it has one (none: the monogram and the name).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Brand {
    pub name: String,
    pub short: String,
    pub colours: [[u8; 3]; 2],
    pub logo: Option<Image>,
}

/// A line of a duty card's column, in the order the duty is driven.
#[derive(Clone, Debug, PartialEq)]
pub enum CardLine {
    /// Bold and centred, ruled under ("Duty start", "BREAK 0902 - 0958").
    Centre(String),
    /// A heading with its value on the right, ruled over and under ("Running no.  130/01").
    Head(String, String),
    /// A trip's heading, bold and set in, ruled over ("Line 110 - Trip 2", "DEADHEAD").
    Sub(String),
    /// A stop and its time (or anything else with one); a timing point on the company's colour.
    Stop { name: String, time: String, timing: bool },
    /// Bold and set in ("Handover to duty 1003").
    Note(String),
}

/// A duty card: the depot over it, its day and date and its number (each a label and its
/// value), two rows of figures, and its lines.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct DutyCard {
    pub depot: String,
    pub day: (String, String),
    pub date: (String, String),
    pub number: (String, String),
    pub figures: Vec<(String, String)>,
    pub more: Vec<(String, String)>,
    pub lines: Vec<CardLine>,
}

/// The cards' own words: what a timing point means, "Page", and what a half says when the duty
/// goes on after it.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CardWords {
    pub legend: String,
    pub page: String,
    pub more: String,
}

/// Half an A4 sheet across (a card: the sheet is cut in two), its margin, its body's top and
/// bottom, a stop's row.
const HALF: f32 = 421.0;
const MARGIN: f32 = 12.0;
const BODY_TOP: f32 = 96.0;
const BODY_BOTTOM: f32 = 541.0;
const ROW: f32 = 11.6;

fn luma(c: [u8; 3]) -> f32 {
    (0.2126 * c[0] as f32 + 0.7152 * c[1] as f32 + 0.0722 * c[2] as f32) / 255.0
}

/// `a` a share `t` of the way to `b`.
fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let m = |k: usize| (a[k] as f32 + (b[k] as f32 - a[k] as f32) * t).round() as u8;
    [m(0), m(1), m(2)]
}

impl Brand {
    /// The colour the cards are marked in: the first, else the second where the first is near
    /// white, else the timetable's yellow.
    fn main(&self) -> [u8; 3] {
        self.colours.iter().copied().find(|c| luma(*c) < 0.92).unwrap_or([255, 214, 0])
    }

    /// The colour as ink on white paper: darkened where it is too light to read.
    fn ink(&self) -> [u8; 3] {
        let c = self.main();
        if luma(c) > 0.55 {
            mix(c, [0, 0, 0], 0.55)
        } else {
            c
        }
    }

    /// A timing point's ground: the colour, light enough to read black on.
    fn ground(&self) -> [u8; 3] {
        let c = self.main();
        mix(c, [255, 255, 255], if luma(c) > 0.7 { 0.25 } else { 0.72 })
    }
}

fn line_h(l: &CardLine) -> f32 {
    match l {
        CardLine::Stop { .. } | CardLine::Note(_) => ROW,
        CardLine::Sub(_) => ROW + 2.0,
        CardLine::Head(..) | CardLine::Centre(_) => ROW + 3.0,
    }
}

/// A card's lines in columns of the body's height, a heading kept with the line after it.
fn columns(lines: &[CardLine]) -> Vec<Vec<&CardLine>> {
    let room = BODY_BOTTOM - BODY_TOP - 4.0;
    let mut cols: Vec<Vec<&CardLine>> = vec![Vec::new()];
    let mut used = 0.0;
    for (i, l) in lines.iter().enumerate() {
        let keep = if matches!(l, CardLine::Head(..) | CardLine::Sub(_)) { lines.get(i + 1).map(line_h).unwrap_or(0.0) } else { 0.0 };
        if used + line_h(l) + keep > room && cols.last().is_some_and(|c| !c.is_empty()) {
            cols.push(Vec::new());
            used = 0.0;
        }
        cols.last_mut().unwrap().push(l);
        used += line_h(l);
    }
    cols
}

/// Words cut to `room` points wide.
fn fit(s: &str, size: f32, bold: bool, room: f32) -> String {
    if Pdf::width(s, size, bold) <= room {
        return s.to_string();
    }
    let mut t = s.to_string();
    while !t.is_empty() && Pdf::width(&format!("{t}..."), size, bold) > room {
        t.pop();
    }
    format!("{}...", t.trim_end())
}

/// A timing point's mark: a diamond around `x`, `y`.
fn diamond(pdf: &mut Pdf, x: f32, y: f32, c: [u8; 3]) {
    pdf.poly(&[(x, y - 2.8), (x + 2.8, y), (x, y + 2.8), (x - 2.8, y)], c, None);
}

/// The company's mark in the top right corner, its right edge at `right`: its logo picture,
/// else its monogram on its colours with its name beside it.
fn mark(pdf: &mut Pdf, right: f32, brand: &Brand, logo: Option<usize>) {
    let (top, high, wide) = (10.0, 30.0, 130.0);
    if let (Some(k), Some(img)) = (logo, brand.logo.as_ref()) {
        let scale = (wide / img.w.max(1) as f32).min(high / img.h.max(1) as f32);
        let (w, h) = (img.w as f32 * scale, img.h as f32 * scale);
        pdf.picture(k, right - w, top + (high - h) * 0.5, w, h);
        return;
    }
    let [main, second] = brand.colours;
    pdf.rect(right - high, top, high, high, main);
    pdf.rect(right - high, top + high * 0.78, high, high * 0.22, second);
    let ink = if luma(main) > 0.6 { [26, 20, 0] } else { [255, 255, 255] };
    pdf.text(right - high * 0.5, top + high * 0.55, 11.0, true, &brand.short, ink, 1);
    let name = fit(&brand.name, 12.0, true, wide);
    pdf.text(right - high - 7.0, top + high * 0.5 + 4.0, 12.0, true, &name, brand.ink(), 2);
}

/// Half a sheet from `x0`: the card's head, its columns `cols` (two at most), the timing points'
/// legend and which page of the duty it is.
#[allow(clippy::too_many_arguments)]
fn card_half(pdf: &mut Pdf, x0: f32, card: &DutyCard, cols: &[Vec<&CardLine>], brand: &Brand, logo: Option<usize>, words: &CardWords, page: usize, pages: usize) {
    let w = HALF - 2.0 * MARGIN;
    let right = x0 + w;
    // the head: the depot, the day and the date on the left; the mark and the number on the right
    pdf.text(x0, 26.0, 13.0, true, &card.depot.to_uppercase(), INK, 0);
    pdf.text(x0, 43.0, 9.5, true, &card.day.0, INK, 0);
    pdf.text(x0 + Pdf::width(&card.day.0, 9.5, true) + 8.0, 43.0, 13.0, true, &card.day.1, INK, 0);
    pdf.text(x0, 59.0, 10.0, true, &card.date.0, INK, 0);
    pdf.text(x0 + Pdf::width(&card.date.0, 10.0, true) + 6.0, 59.0, 10.0, true, &card.date.1, INK, 0);
    mark(pdf, right, brand, logo);
    pdf.text(right, 60.0, 15.0, true, &card.number.1, brand.ink(), 2);
    pdf.text(right - Pdf::width(&card.number.1, 15.0, true) - 8.0, 60.0, 10.5, true, &card.number.0, INK, 2);
    pdf.line((x0, 66.0), (right, 66.0), 2.2, brand.main());
    // the figures, in two rows
    for (row, y) in [(&card.figures, 77.0), (&card.more, 91.0)] {
        let slot = w / row.len().max(1) as f32;
        for (k, (label, value)) in row.iter().enumerate() {
            let x = x0 + k as f32 * slot;
            pdf.text(x, y, 6.8, false, label, INK, 0);
            let room = slot - Pdf::width(label, 6.8, false) - 8.0;
            pdf.text(x + Pdf::width(label, 6.8, false) + 4.0, y, 6.8, true, &fit(value, 6.8, true, room), INK, 0);
        }
    }
    pdf.line((x0, 82.0), (right, 82.0), 0.3, RULE);
    // the body: two columns
    pdf.frame(x0, BODY_TOP, w, BODY_BOTTOM - BODY_TOP, 0.6, INK);
    pdf.line((x0 + w * 0.5, BODY_TOP), (x0 + w * 0.5, BODY_BOTTOM), 0.6, INK);
    let cw = w * 0.5;
    let (ground, ink) = (brand.ground(), brand.ink());
    for (k, col) in cols.iter().enumerate() {
        let cx = x0 + k as f32 * cw;
        let time_x = cx + cw - 20.0;
        let mut y = BODY_TOP + 2.0;
        for (i, l) in col.iter().enumerate() {
            let h = line_h(l);
            let base = y + h - 3.4;
            let rule = |pdf: &mut Pdf, at: f32| pdf.line((cx, at), (cx + cw, at), 0.4, GREY);
            match l {
                CardLine::Stop { name, time, timing } => {
                    if *timing {
                        pdf.rect(cx + 3.0, y + 1.0, time_x - cx - 1.0, h - 1.4, ground);
                        diamond(pdf, time_x + 9.0, y + h * 0.5 + 0.2, ink);
                    }
                    let room = time_x - cx - 10.0 - Pdf::width(time, 8.0, false);
                    pdf.text(cx + 6.0, base, 8.0, false, &fit(name, 8.0, false, room), INK, 0);
                    pdf.text(time_x, base, 8.0, false, time, INK, 2);
                }
                CardLine::Note(t) => pdf.text(cx + 10.0, base, 8.0, true, &fit(t, 8.0, true, cw - 14.0), INK, 0),
                CardLine::Sub(t) => {
                    if i > 0 {
                        rule(pdf, y + 0.5);
                    }
                    pdf.text(cx + 18.0, base, 8.2, true, &fit(t, 8.2, true, cw - 22.0), INK, 0);
                }
                CardLine::Head(label, value) => {
                    if i > 0 {
                        rule(pdf, y + 0.5);
                    }
                    rule(pdf, y + h - 0.5);
                    pdf.text(cx + 6.0, base - 1.0, 8.5, true, label, INK, 0);
                    pdf.text(time_x, base - 1.0, 8.5, true, value, INK, 2);
                }
                CardLine::Centre(t) => {
                    if i > 0 {
                        rule(pdf, y + 0.5);
                    }
                    rule(pdf, y + h - 0.5);
                    pdf.text(cx + cw * 0.5, base - 1.0, 8.5, true, &fit(t, 8.5, true, cw - 8.0), INK, 1);
                }
            }
            y += h;
        }
    }
    // the legend: what the timing point's mark says
    pdf.frame(x0, BODY_BOTTOM + 4.0, w, 24.0, 1.4, INK);
    let room = Pdf::width(&words.legend, 7.0, true) + 9.0;
    let at = x0 + w * 0.5 - room * 0.5;
    diamond(pdf, at + 3.0, BODY_BOTTOM + 16.0, ink);
    pdf.text(at + 9.0, BODY_BOTTOM + 18.5, 7.0, true, &words.legend, INK, 0);
    pdf.text(x0 + w * 0.5, 584.0, 8.5, false, &format!("{} {page}/{pages}", words.page), INK, 1);
    if page < pages {
        pdf.text(right, 584.0, 6.0, false, &words.more, GREY, 2);
    }
}

/// The duty cards as a PDF: two cards to an A4 sheet across, as a depot prints them to cut
/// in two - every stop of a trip with its time, the timing points on the company's colour and
/// marked, in the company's look (`brand`). A duty of more than two columns goes on on the
/// half after; a duty of more than a half begins on a sheet of its own.
pub fn duty_cards(cards: &[DutyCard], brand: &Brand, words: &CardWords) -> Vec<u8> {
    let mut pdf = Pdf::new(842.0, 595.0);
    let logo = brand.logo.clone().map(|img| pdf.add_image(img));
    // (the halves used so far: even ones on the left)
    let mut slot = 0usize;
    for card in cards {
        let cols = columns(&card.lines);
        let halves = cols.len().div_ceil(2).max(1);
        if slot % 2 == 1 && halves > 1 {
            slot += 1;
        }
        for h in 0..halves {
            let at = slot + h;
            while pdf.pages() < at / 2 + 1 {
                pdf.new_page();
            }
            let right = !at.is_multiple_of(2);
            let x0 = if right { HALF + MARGIN } else { MARGIN };
            if right {
                // (where the sheet is cut)
                pdf.line((HALF, BODY_TOP), (HALF, BODY_BOTTOM + 28.0), 2.0, INK);
            }
            let shown = &cols[(2 * h).min(cols.len())..(2 * h + 2).min(cols.len())];
            card_half(&mut pdf, x0, card, shown, brand, logo, words, h + 1, halves);
        }
        slot += halves;
    }
    pdf.finish()
}

/// Two letters for a stop (as the bus graph writes them over the trips' ends).
pub fn stop_letters(stop: &str) -> String {
    let s: String = stop.trim().chars().filter(|c| c.is_alphanumeric()).take(2).collect();
    let mut cs = s.chars();
    match (cs.next(), cs.next()) {
        (Some(a), Some(b)) => format!("{}{}", a.to_uppercase(), b.to_lowercase()),
        (Some(a), None) => a.to_uppercase().to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str) -> GraphRow {
        let trip =
            |from: i32, to: i32, inbound: bool| GraphItem { from, to, kind: ItemKind::Trip { inbound }, label: "150".into(), colour: [245, 128, 40], start_stop: "Jo".into(), end_stop: "Re".into() };
        GraphRow {
            name: name.into(),
            sub: "Any vehicle".into(),
            sub2: "(239.9 km)".into(),
            items: vec![
                GraphItem { from: 341, to: 360, kind: ItemKind::Pull, label: String::new(), colour: [0, 0, 0], start_stop: String::new(), end_stop: String::new() },
                trip(360, 415, false),
                trip(420, 475, true),
            ],
            under: vec![(341, 420, "1001".into()), (420, 480, "1002".into())],
        }
    }

    #[test]
    fn a_graph_is_a_pdf_with_its_pages_and_a_table_that_points_at_its_objects() {
        let rows: Vec<GraphRow> = (0..30).map(|k| row(&format!("RB{k}"))).collect();
        let pdf = graph("Bus Graph", &["Depot: Grundorf".into(), "Service: Mon - Fri (€)".into()], "Printed: 1989-05-30 12:00", "Page", &rows);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        if let Ok(p) = std::env::var("ROSTERFLOW_PDF_OUT") {
            std::fs::write(p, &pdf).unwrap();
        }
        // (bytes as Latin-1 chars, every one a byte: its offsets are the file's)
        let text: Vec<u8> = pdf.iter().map(|&b| if b < 0x80 { b } else { b'?' }).collect();
        let text = String::from_utf8(text).unwrap();
        // 30 rows of 46 points on A4 across: three pages
        assert!(text.contains("/Count 3"), "{}", &text[..300]);
        // every offset in the table is where its object begins
        let at = text.rfind("startxref\n").unwrap();
        let xref: usize = text[at + 10..].lines().next().unwrap().parse().unwrap();
        assert!(text[xref..].starts_with("xref"));
        for (k, l) in text[xref..].lines().skip(3).take_while(|l| l.ends_with(" n ")).enumerate() {
            let off: usize = l[..10].parse().unwrap();
            assert!(pdf[off..].starts_with(format!("{} 0 obj", k + 1).as_bytes()), "object {}", k + 1);
        }
        // the euro sign as WinAnsi's, a bracket escaped
        assert!(pdf.windows(3).any(|w| w == [0x80, b'\\', b')']));
        assert_eq!(pdf_string("a(b)"), b"(a\\(b\\))".to_vec());
        assert_eq!(stop_letters("Rennweg"), "Re");
        assert_eq!(stop_letters(" "), "");
    }

    fn card(stops: usize) -> DutyCard {
        let mut lines = vec![CardLine::Centre("Duty start".into()), CardLine::Head("Running no.".into(), "130/01".into()), CardLine::Sub("Line 130 - Trip 1".into())];
        lines.extend((0..stops).map(|k| CardLine::Stop { name: format!("Stop {k}"), time: format!("{:04}", 600 + k), timing: k % 3 == 0 }));
        lines.push(CardLine::Centre("Duty end".into()));
        DutyCard {
            depot: "Grundorf".into(),
            day: ("Day type:".into(), "Sunday".into()),
            date: ("Date:".into(), "Sun 7 Apr 2024".into()),
            number: ("Duty no.:".into(), "1001".into()),
            figures: vec![("Sign on:".into(), "0435".into()), ("Driving time:".into(), "06:26".into())],
            more: vec![("Length:".into(), "08:41".into())],
            lines,
        }
    }

    #[test]
    fn duty_cards_go_two_to_a_sheet_and_a_long_duty_begins_its_own() {
        // (a column holds about 37 stops: 30 fit a half, 100 take two halves)
        assert_eq!(columns(&card(30).lines).len(), 1);
        assert_eq!(columns(&card(100).lines).len(), 3);
        // a heading is never the last line of a column
        for col in columns(&card(100).lines) {
            assert!(!matches!(col.last(), Some(CardLine::Head(..) | CardLine::Sub(_))));
        }
        let brand = Brand { name: "Stadtbus".into(), short: "SB".into(), colours: [[0, 61, 130], [255, 214, 0]], logo: None };
        let words = CardWords { legend: "Timing point".into(), page: "Page".into(), more: "Goes on".into() };
        let count = |pdf: &[u8]| {
            let text: String = pdf.iter().map(|&b| if b < 0x80 { b as char } else { '?' }).collect();
            text.split("/Count ").nth(1).and_then(|r| r.split(' ').next()).and_then(|n| n.parse::<usize>().ok()).unwrap()
        };
        // two short duties share a sheet; a long one begins the next and takes it whole; a short
        // one after it takes the sheet after
        assert_eq!(count(&duty_cards(&[card(10), card(10)], &brand, &words)), 1);
        let three = duty_cards(&[card(10), card(100), card(10)], &brand, &words);
        assert_eq!(count(&three), 3);
        if let Ok(p) = std::env::var("DUTY_CARDS_PDF_OUT") {
            std::fs::write(p, &three).unwrap();
        }
        // the logo picture: one object, its transparency another, both shown by every sheet
        let logo = Image { w: 2, h: 1, components: 3, filter: "", data: vec![255, 0, 0, 0, 0, 255], alpha: Some(vec![]) };
        let pdf = duty_cards(&[card(10), card(100)], &Brand { logo: Some(logo), ..brand.clone() }, &words);
        let text: String = pdf.iter().map(|&b| if b < 0x80 { b as char } else { '?' }).collect();
        assert!(text.contains("5 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /SMask 6 0 R"));
        assert_eq!(text.matches("/XObject << /Im0 5 0 R >>").count(), 2);
        assert!(text.contains("/Kids [7 0 R 9 0 R]"));
        let at = text.rfind("startxref\n").unwrap();
        let xref: usize = text[at + 10..].lines().next().unwrap().parse().unwrap();
        for (k, l) in text[xref..].lines().skip(3).take_while(|l| l.ends_with(" n ")).enumerate() {
            let off: usize = l[..10].parse().unwrap();
            assert!(pdf[off..].starts_with(format!("{} 0 obj", k + 1).as_bytes()), "object {}", k + 1);
        }
        // a light company colour is darkened to be read on paper, a timing point's ground lightened
        let yellow = Brand { colours: [[255, 214, 0], [0, 0, 0]], ..Default::default() };
        assert!(luma(yellow.ink()) < 0.55 && luma(yellow.ground()) > luma(yellow.main()));
        assert_eq!(Brand { colours: [[255, 255, 255], [0, 61, 130]], ..Default::default() }.main(), [0, 61, 130]);
    }
}
