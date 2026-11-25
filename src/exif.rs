//! Minimal EXIF reader: only the capture date ("DateTimeOriginal"), used by the Camcorder date stamp.
//! Works for JPEG / HEIC / PNG / WebP (an embedded "Exif\0\0" TIFF block) and TIFF-based RAW files.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DateTaken {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

/// Capture date of an image file (cached per file content).
pub fn date_taken_cached(id: uuid::Uuid, data: &[u8]) -> Option<DateTaken> {
    static CACHE: OnceLock<Mutex<HashMap<uuid::Uuid, Option<DateTaken>>>> = OnceLock::new();
    let c = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(v) = c.lock().unwrap_or_else(|e| e.into_inner()).get(&id) {
        return *v;
    }
    let v = date_taken(data);
    c.lock().unwrap_or_else(|e| e.into_inner()).insert(id, v);
    v
}

pub fn date_taken(data: &[u8]) -> Option<DateTaken> {
    // a TIFF file (DNG, NEF, CR2, ARW…) is itself the EXIF container
    if data.len() > 8 && (&data[..4] == b"II*\0" || &data[..4] == b"MM\0*") {
        if let Some(d) = parse_tiff(data) {
            return Some(d);
        }
    }
    // otherwise look for the "Exif\0\0" marker (JPEG APP1, HEIC item, WebP/PNG chunk)
    let limit = data.len().min(8 << 20);
    let hay = &data[..limit];
    let mut i = 0;
    while let Some(p) = find(&hay[i..], b"Exif\0\0") {
        let start = i + p + 6;
        if let Some(d) = parse_tiff(&data[start..]) {
            return Some(d);
        }
        i = start;
    }
    // PNG eXIf chunks have no "Exif\0\0" prefix
    if let Some(p) = find(hay, b"eXIf") {
        if let Some(d) = parse_tiff(&data[p + 4..]) {
            return Some(d);
        }
    }
    None
}

fn find(h: &[u8], n: &[u8]) -> Option<usize> { h.windows(n.len()).position(|w| w == n) }

fn parse_tiff(t: &[u8]) -> Option<DateTaken> {
    if t.len() < 8 {
        return None;
    }
    let le = match &t[..2] {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    let u16_at = |o: usize| -> Option<u16> {
        let b = t.get(o..o + 2)?;
        Some(if le { u16::from_le_bytes([b[0], b[1]]) } else { u16::from_be_bytes([b[0], b[1]]) })
    };
    let u32_at = |o: usize| -> Option<u32> {
        let b = t.get(o..o + 4)?;
        Some(if le { u32::from_le_bytes([b[0], b[1], b[2], b[3]]) } else { u32::from_be_bytes([b[0], b[1], b[2], b[3]]) })
    };
    // returns (tag → value offset or inline value) for one IFD
    let read_ifd = |off: usize| -> Option<Vec<(u16, u16, u32, usize)>> {
        let n = u16_at(off)? as usize;
        if n > 1000 {
            return None;
        }
        let mut v = vec![];
        for k in 0..n {
            let e = off + 2 + k * 12;
            let tag = u16_at(e)?;
            let typ = u16_at(e + 2)?;
            let count = u32_at(e + 4)?;
            v.push((tag, typ, count, e + 8));
        }
        Some(v)
    };
    let ascii = |count: u32, field: usize| -> Option<String> {
        let len = count as usize;
        let off = if len <= 4 { field } else { u32_at(field)? as usize };
        let b = t.get(off..off + len.min(64))?;
        Some(String::from_utf8_lossy(b).trim_end_matches('\0').to_string())
    };
    let ifd0 = read_ifd(u32_at(4)? as usize)?;
    let mut fallback = None;
    for &(tag, _, count, field) in &ifd0 {
        if tag == 0x0132 {
            fallback = ascii(count, field).and_then(|s| parse_date(&s));
        }
        if tag == 0x8769 {
            let sub = u32_at(field)? as usize;
            if let Some(exif) = read_ifd(sub) {
                for &(t2, _, c2, f2) in &exif {
                    if t2 == 0x9003 || t2 == 0x9004 {
                        if let Some(d) = ascii(c2, f2).and_then(|s| parse_date(&s)) {
                            return Some(d);
                        }
                    }
                }
            }
        }
    }
    fallback
}

/// "YYYY:MM:DD HH:MM:SS"
fn parse_date(s: &str) -> Option<DateTaken> {
    let s = s.trim();
    let n = |a: usize, b: usize| s.get(a..b).and_then(|x| x.trim().parse::<u32>().ok());
    let year = n(0, 4)? as i32;
    let (month, day) = (n(5, 7)?, n(8, 10)?);
    if year < 1900 || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(DateTaken { year, month, day, hour: n(11, 13).unwrap_or(12), minute: n(14, 16).unwrap_or(0), second: n(17, 19).unwrap_or(0) })
}

impl DateTaken {
    /// Today (for photos without a date), in local-ish UTC.
    pub fn now() -> DateTaken {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        let days = secs.div_euclid(86400);
        let rem = secs.rem_euclid(86400);
        // civil-from-days (Howard Hinnant)
        let z = days + 719468;
        let era = z.div_euclid(146097);
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let y = (yoe + era * 400 + if m <= 2 { 1 } else { 0 }) as i32;
        DateTaken { year: y, month: m, day: d, hour: (rem / 3600) as u32, minute: ((rem % 3600) / 60) as u32, second: (rem % 60) as u32 }
    }
}
