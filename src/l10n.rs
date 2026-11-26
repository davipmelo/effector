//! UI translation. English text is the key; Portuguese strings come from the macOS app.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

include!("l10n_pt.rs");
include!("l10n_pt_080.rs");
include!("l10n_pt_081.rs");
include!("l10n_pt_082.rs");
include!("l10n_pt_083.rs");

static PORTUGUESE: AtomicBool = AtomicBool::new(false);

pub fn set_portuguese(on: bool) { PORTUGUESE.store(on, Ordering::Relaxed); }
pub fn is_portuguese() -> bool { PORTUGUESE.load(Ordering::Relaxed) }

fn table() -> &'static HashMap<&'static str, &'static str> {
    static T: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    T.get_or_init(|| PT.iter().chain(PT_080.iter()).chain(PT_081.iter()).chain(PT_082.iter()).chain(PT_083.iter()).copied().collect())
}

/// Translates an English UI string.
pub fn tr(s: &str) -> String {
    if !is_portuguese() {
        return s.to_string();
    }
    table().get(s).map(|v| v.to_string()).unwrap_or_else(|| s.to_string())
}

/// Translate with printf-style arguments (%d, %@, %s, %ld are replaced in order).
pub fn trf(s: &str, args: &[&dyn std::fmt::Display]) -> String {
    let t = tr(s);
    let mut out = String::new();
    let mut it = args.iter();
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let mut spec = String::new();
            while let Some(&n) = chars.peek() {
                spec.push(n);
                chars.next();
                if n.is_ascii_alphabetic() && n != 'l' || n == '@' || n == '%' {
                    break;
                }
            }
            if spec == "%" {
                out.push('%');
            } else if let Some(a) = it.next() {
                out += &a.to_string();
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// The system prefers Portuguese.
pub fn system_is_portuguese() -> bool {
    sys_locale::get_locale().map(|l| l.to_lowercase().starts_with("pt")).unwrap_or(false)
}
