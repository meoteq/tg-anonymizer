use std::path::PathBuf;
use std::collections::{HashSet, HashMap};
use std::fs::File;
use std::io::{Write, BufWriter};
use std::sync::mpsc::Sender;
use scraper::{Html, Selector};
use once_cell::sync::Lazy;
use regex::Regex;

// --- Progress message for thread communication ---
pub enum ProgressMessage {
    Log(String),
    Progress(f32),
    Status(String),
    Finished(usize, PathBuf), // (file count, output path)
    Error(String),
}

// --- Settings structure for optional anonymization ---
#[derive(Clone, Copy)]
pub struct AnonymizeSettings {
    pub hide_names: bool,
    pub hide_phones: bool,
    pub hide_emails: bool,
    pub hide_links: bool,
    pub hide_cards: bool,
    pub hide_addresses: bool,
    pub hide_tokens: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// REGEXES
// ─────────────────────────────────────────────────────────────────────────────

// Credit / debit card numbers (groups of 4 digits, all common separators)
static CARD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:\d{4}[\s\-.]){3}\d{4}\b").unwrap()
});

// Phone numbers — international & local formats
// Requires minimum 7 total digits and a plausible phone structure
// Covers: +7 (999) 123-45-67, 8-800-555-3535, (123) 456-7890, 89991234567, +79991234567
//
// NOTE: because the `regex` crate has no lookaround, boundaries are matched as
// consumed characters (group 1 is the actual phone digits, prefix/suffix are the
// boundary chars). This means two phone numbers written back-to-back separated
// by exactly one non-digit character (e.g. "89991234567,89997654321") may only
// have the first one fully matched, since the separator gets consumed as the
// first match's suffix and is then unavailable as the second match's prefix.
// Acceptable tradeoff for chat exports; rare in practice.
static PHONE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?x)
        (?:^|[^0-9])
        (
          \+?[1-9]\d{0,2}[\s\-.]?\(?\d{3}\)?[\s\-.]+\d{3}[\s\-.]?\d{2}[\s\-.]?\d{2}  # structured format with separators
          |
          \+[1-9]\d{0,2}\d{7,10}  # international format, raw digits (e.g. +79991234567)
          |
          [78]9\d{9}  # Russian mobile format, raw digits (e.g. 89991234567, 79991234567)
        )
        (?:$|[^0-9])
        "
    )
    .unwrap()
});

// E-mail addresses (RFC 5321 simplified)
static EMAIL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}\b").unwrap()
});

// Web links — http/https/ftp + bare t.me / vk.com etc. without protocol
static LINK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(?:https?://|ftp://|www\.)\S+|(?:t\.me|vk\.com|discord\.gg|instagram\.com|twitter\.com|x\.com|youtube\.com|youtu\.be)/\S*",
    )
    .unwrap()
});

// ─── Address patterns ───────────────────────────────────────────────────────
// Russian: ул./улица, пр/проспект, пер/переулок, бульвар, шоссе, наб., etc.
static ADDR_RU: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?iu)\b(ул|улица|пр-?т?|проспект|пер|переулок|б-р|бульвар|шоссе|наб|набережная|пл|площадь|туп|тупик)\.?\s+[А-Яа-яёЁ0-9][\wА-Яа-яёЁ\s]{0,40}(?:,\s*(?:д|дом)\.?\s*\d+[а-яА-Я]?)?(?:,\s*(?:кв|квартира|оф|офис)\.?\s*\d+)?\b"
    )
    .unwrap()
});
// "д. 12, кв. 5" without a street name
static ADDR_RU_SHORT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?iu)\b(д|дом)\.?\s*\d+[а-яА-Я]?[,\s]+(кв|квартира|оф|офис)\.?\s*\d+\b").unwrap()
});

// ─── Token / API key patterns ────────────────────────────────────────────────
// Telegram bot tokens:  123456789:ABCDEFGxxxxxxxxxxxxxxxxxxxxxxx
static TG_BOT_TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b\d{8,12}:[A-Za-z0-9_\-]{30,60}\b").unwrap()
});
// Generic high-entropy hex tokens. Minimum raised to 40 chars (SHA1+) rather
// than 32 (MD5-length) to cut down on false positives against Telegram
// message/file ids and other incidental 32-char hex strings that show up in
// ordinary chat content but aren't secrets.
static HEX_TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[A-Fa-f0-9]{40,}\b").unwrap()
});
// Common API key patterns: sk-*, ghp_*, ya_*, AIza*, AKIA* etc.
static API_KEY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(?:sk-[A-Za-z0-9]{20,}|ghp_[A-Za-z0-9]{36}|ghs_[A-Za-z0-9]{36}|ya_[A-Za-z0-9_\-]{30,}|AIza[A-Za-z0-9_\-]{35}|AKIA[A-Z0-9]{16}|xoxb-[A-Za-z0-9\-]+|xoxp-[A-Za-z0-9\-]+)\b",
    )
    .unwrap()
});
// Bearer tokens inside text (e.g. "Bearer eyJhb...")
static BEARER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bBearer\s+[A-Za-z0-9\-_=+/]{20,}\b").unwrap()
});
// High-entropy alphanumeric tokens (20+ chars mixing letters and digits).
// Actual redaction additionally requires len >= 24 and a mix of upper/lower/
// digit (see `redact`), which keeps this from firing on ordinary long words,
// usernames, or hashtags.
static ALNUM_TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[A-Za-z0-9]{20,}\b").unwrap()
});

// ─────────────────────────────────────────────────────────────────────────────
// HTML HELPERS
// ─────────────────────────────────────────────────────────────────────────────

/// Recursive HTML text extractor — replaces <br> with newlines.
pub fn extract_text(element: scraper::ElementRef) -> String {
    let mut text = String::new();
    traverse(*element, &mut text);
    text.trim().to_string()
}

fn traverse(node: ego_tree::NodeRef<'_, scraper::node::Node>, text: &mut String) {
    use scraper::node::Node;
    match node.value() {
        Node::Text(t) => {
            text.push_str(&t.text);
        }
        Node::Element(e) => {
            let name = e.name();
            if name == "br" {
                text.push('\n');
            } else {
                for child in node.children() {
                    traverse(child, text);
                }
            }
        }
        _ => {
            for child in node.children() {
                traverse(child, text);
            }
        }
    }
}

/// Extract numeric suffix from filename for natural sorting.
/// E.g. "messages.html" -> 0, "messages2.html" -> 2, "messages14.html" -> 14
fn natural_file_index(path: &PathBuf) -> u64 {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let digits: String = stem
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    digits.parse().unwrap_or(0)
}

/// Extract only direct (non-nested) text from an element — avoids picking up
/// timestamps or dates that Telegram puts inside child spans of from_name.
fn direct_text(element: scraper::ElementRef) -> String {
    use scraper::node::Node;
    let mut result = String::new();
    for child in element.children() {
        if let Node::Text(t) = child.value() {
            result.push_str(&t.text);
        }
    }
    result.trim().to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// IN-MEMORY REPRESENTATION (built during the single HTML-parsing pass)
// ─────────────────────────────────────────────────────────────────────────────

enum MessageBody {
    Text(String),
    Media,
}

/// One HTML `div.message.default` element can carry a sender-name update
/// and/or a message body. Both are captured here so the second (in-memory)
/// pass can reproduce the exact same "carry current sender forward" logic
/// the original single-pass version used, without touching the disk again.
struct RawEntry {
    sender_update: Option<String>,
    body: Option<MessageBody>,
}

// ─────────────────────────────────────────────────────────────────────────────
// PROCESSING
// ─────────────────────────────────────────────────────────────────────────────

/// Apply all enabled redactions to a single message string.
fn redact(mut text: String, settings: &AnonymizeSettings, names_re: &Option<Regex>, name_map_lower: &HashMap<String, String>) -> String {
    // 1. Replace links and emails first to avoid matching tokens/codes inside URLs
    if settings.hide_links {
        text = LINK_RE.replace_all(&text, "[LINK]").into_owned();
    }
    if settings.hide_emails {
        text = EMAIL_RE.replace_all(&text, "[EMAIL]").into_owned();
    }

    // 2. Hide tokens
    if settings.hide_tokens {
        text = TG_BOT_TOKEN_RE.replace_all(&text, "[TOKEN]").into_owned();
        text = BEARER_RE.replace_all(&text, "[TOKEN]").into_owned();
        text = API_KEY_RE.replace_all(&text, "[TOKEN]").into_owned();
        text = HEX_TOKEN_RE.replace_all(&text, "[TOKEN]").into_owned();
        text = ALNUM_TOKEN_RE.replace_all(&text, |caps: &regex::Captures| {
            let matched = caps.get(0).unwrap().as_str();
            // Stricter token validation: length >= 24, contains mixed case and digits
            if matched.len() >= 24 {
                let has_digit = matched.chars().any(|c| c.is_ascii_digit());
                let has_upper = matched.chars().any(|c| c.is_ascii_uppercase());
                let has_lower = matched.chars().any(|c| c.is_ascii_lowercase());
                if has_digit && has_upper && has_lower {
                    return "[TOKEN]".to_string();
                }
            }
            matched.to_string()
        }).into_owned();
    }

    // 3. Hide cards
    if settings.hide_cards {
        text = CARD_RE.replace_all(&text, "[CARD]").into_owned();
    }

    // 4. Hide phones (preserving boundary characters)
    if settings.hide_phones {
        text = PHONE_RE.replace_all(&text, |caps: &regex::Captures| {
            let full_match = caps.get(0).unwrap().as_str();
            let phone_match = caps.get(1).unwrap().as_str();
            let prefix_len = full_match.find(phone_match).unwrap();
            let prefix = &full_match[..prefix_len];
            let suffix = &full_match[prefix_len + phone_match.len()..];
            format!("{}[PHONE]{}", prefix, suffix)
        }).into_owned();
    }

    // 5. Hide addresses
    if settings.hide_addresses {
        text = ADDR_RU.replace_all(&text, "[ADDRESS]").into_owned();
        text = ADDR_RU_SHORT.replace_all(&text, "[ADDRESS]").into_owned();
    }

    // 6. Hide names
    if settings.hide_names {
        if let Some(re) = names_re {
            text = re
                .replace_all(&text, |caps: &regex::Captures| {
                    if let Some(matched) = caps.get(1) {
                        let key = matched.as_str().to_lowercase();
                        name_map_lower
                            .get(&key)
                            .cloned()
                            .unwrap_or_else(|| matched.as_str().to_string())
                    } else {
                        caps.get(0).unwrap().as_str().to_string()
                    }
                })
                .into_owned();
        }
    }
    text
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN ENTRY POINT
// ─────────────────────────────────────────────────────────────────────────────

pub fn run_processing(
    files: Vec<PathBuf>,
    output_path: PathBuf,
    settings: AnonymizeSettings,
    tx: Sender<ProgressMessage>,
) {
    let log = |msg: String| {
        let _ = tx.send(ProgressMessage::Log(msg));
    };

    let status = |msg: String| {
        let _ = tx.send(ProgressMessage::Status(msg));
    };

    // Sort files in natural order: messages.html (0), messages2.html (2), ... messages14.html (14)
    let mut files = files;
    files.sort_by_key(|p| natural_file_index(p));
    log(format!(
        "Files sorted in natural order: {}",
        files
            .iter()
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ));

    let message_selector = Selector::parse("div.message.default").unwrap();
    let from_name_selector = Selector::parse("div.from_name").unwrap();
    let text_selector = Selector::parse("div.text").unwrap();
    let media_selector = Selector::parse("div.media_wrap").unwrap();

    let mut unique_names: HashSet<String> = HashSet::new();
    let mut raw_entries: Vec<RawEntry> = Vec::new();

    // ── Single pass: read + parse every file once, extract everything we
    // need (sender updates, message bodies, unique names) into memory. This
    // replaces the old two-pass approach (scan names, then re-read/re-parse
    // every file to write output) with one disk read and one HTML parse per
    // file, at the cost of holding extracted text in memory for the run.
    status("Step 1/2: Reading & extracting messages...".to_string());
    log("Phase 1: Parsing files and extracting messages...".to_string());

    for (idx, file_path) in files.iter().enumerate() {
        let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();
        log(format!(
            "Parsing: {} ({}/{})",
            file_name,
            idx + 1,
            files.len()
        ));

        let html_content = match std::fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(err) => {
                let _ = tx.send(ProgressMessage::Error(format!(
                    "Failed to read {}: {}",
                    file_name, err
                )));
                return;
            }
        };

        let document = Html::parse_document(&html_content);
        let msg_count = document.select(&message_selector).count();
        log(format!("  -> Found {} messages in {}", msg_count, file_name));

        for msg_element in document.select(&message_selector) {
            // Sender name (direct text only — avoids nested date spans)
            let sender_update = msg_element
                .select(&from_name_selector)
                .next()
                .map(direct_text)
                .filter(|s| !s.is_empty());

            if let Some(name) = &sender_update {
                if settings.hide_names {
                    unique_names.insert(name.clone());
                }
            }

            // Message body: text takes priority, then media, then nothing
            let body = if let Some(text_element) = msg_element.select(&text_selector).next() {
                let raw = extract_text(text_element);
                if !raw.is_empty() {
                    Some(MessageBody::Text(raw))
                } else {
                    None
                }
            } else if msg_element.select(&media_selector).next().is_some() {
                Some(MessageBody::Media)
            } else {
                None
            };

            if sender_update.is_some() || body.is_some() {
                raw_entries.push(RawEntry { sender_update, body });
            }
        }

        // Extraction phase progress: 0% -> 85%
        let extract_progress = 0.85 * ((idx + 1) as f32 / files.len() as f32);
        let _ = tx.send(ProgressMessage::Progress(extract_progress));
    }

    if settings.hide_names {
        log(format!(
            "Sender scan complete. Found {} participants.",
            unique_names.len()
        ));
    } else {
        log("Name anonymization is disabled.".to_string());
    }

    // Assign placeholders [Participant N] — sort longest name first to avoid
    // partial-match collisions (e.g. "John Doe" before "John")
    let mut names_list: Vec<String> = unique_names.into_iter().collect();
    names_list.sort_by(|a, b| b.len().cmp(&a.len()));

    let mut name_map: HashMap<String, String> = HashMap::new();
    let mut name_map_lower: HashMap<String, String> = HashMap::new();

    if settings.hide_names {
        for (idx, name) in names_list.iter().enumerate() {
            let placeholder = format!("[Participant {}]", idx + 1);
            log(format!("Participant: \"{}\" -> {}", name, placeholder));
            name_map.insert(name.clone(), placeholder.clone());
            name_map_lower.insert(name.to_lowercase(), placeholder);
        }
    }

    // Compile a combined regex for all participant names (single-pass replacement)
    let names_re: Option<Regex> = if settings.hide_names && !names_list.is_empty() {
        let escaped: Vec<String> = names_list.iter().map(|n| regex::escape(n)).collect();
        let pattern = format!(r"(?i)\b({})\b", escaped.join("|"));
        match Regex::new(&pattern) {
            Ok(re) => Some(re),
            Err(err) => {
                log(format!(
                    "Warning: failed to compile combined names regex: {}",
                    err
                ));
                None
            }
        }
    } else {
        None
    };

    status("Step 2/2: Anonymizing & saving...".to_string());
    log(format!(
        "Phase 2: Redacting and writing {} entries...",
        raw_entries.len()
    ));

    // Create the output file with BufWriter
    let file = match File::create(&output_path) {
        Ok(f) => f,
        Err(err) => {
            let _ = tx.send(ProgressMessage::Error(format!(
                "Failed to create output file: {}",
                err
            )));
            return;
        }
    };
    let mut out_file = BufWriter::new(file);

    let mut current_sender = String::new();
    let total_entries = raw_entries.len().max(1);

    // ── Phase 2: redact & write, entirely from memory (no disk re-reads) ────
    for (idx, entry) in raw_entries.into_iter().enumerate() {
        if let Some(name) = entry.sender_update {
            current_sender = if settings.hide_names {
                name_map.get(&name).cloned().unwrap_or(name)
            } else {
                name
            };
        }

        if let Some(body) = entry.body {
            let text = match body {
                MessageBody::Text(raw) => redact(raw, &settings, &names_re, &name_map_lower),
                MessageBody::Media => "[MEDIA]".to_string(),
            };
            let sender = if current_sender.is_empty() {
                "[Unknown]"
            } else {
                &current_sender
            };
            if let Err(err) = writeln!(out_file, "**{}**: {}\n", sender, text) {
                let _ = tx.send(ProgressMessage::Error(format!(
                    "Error writing to file: {}",
                    err
                )));
                return;
            }
        }

        // Writing phase progress: 85% -> 100%
        if idx % 200 == 0 || idx + 1 == total_entries {
            let write_progress = 0.85 + 0.15 * ((idx + 1) as f32 / total_entries as f32);
            let _ = tx.send(ProgressMessage::Progress(write_progress));
        }
    }

    // Flush BufWriter to ensure all data is written to disk
    if let Err(err) = out_file.flush() {
        let _ = tx.send(ProgressMessage::Error(format!(
            "Failed to flush output file: {}",
            err
        )));
        return;
    }

    log("All files processed successfully!".to_string());
    let _ = tx.send(ProgressMessage::Finished(files.len(), output_path));
}
