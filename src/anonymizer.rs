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
// Covers: +7 (999) 123-45-67, 8-800-555-3535, (123) 456-7890, etc.
static PHONE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?x)
        (?:\+?[1-9]\d{0,2}[\s\-.]?)?   # optional country code (1-3 digits)
        (?:\(?\d{3}\)?[\s\-.]?)          # area code (3 digits)
        \d{3}[\s\-.]?                    # first digit group
        \d{2}[\s\-.]?\d{2}              # last two groups (xx-xx or xxxx)
        ",
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
// Generic high-entropy tokens: 32+ hex chars or base64-like strings
static HEX_TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[A-Fa-f0-9]{32,}\b").unwrap()
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
// PROCESSING
// ─────────────────────────────────────────────────────────────────────────────

/// Apply all enabled redactions to a single message string.
fn redact(mut text: String, settings: &AnonymizeSettings, names_re: &Option<Regex>, name_map_lower: &HashMap<String, String>) -> String {
    // Order matters: tokens first (most specific), then cards, phones, emails, links, addresses
    if settings.hide_tokens {
        text = TG_BOT_TOKEN_RE.replace_all(&text, "[TOKEN]").into_owned();
        text = BEARER_RE.replace_all(&text, "[TOKEN]").into_owned();
        text = API_KEY_RE.replace_all(&text, "[TOKEN]").into_owned();
        text = HEX_TOKEN_RE.replace_all(&text, "[TOKEN]").into_owned();
    }
    if settings.hide_cards {
        text = CARD_RE.replace_all(&text, "[CARD]").into_owned();
    }
    if settings.hide_phones {
        text = PHONE_RE.replace_all(&text, "[PHONE]").into_owned();
    }
    if settings.hide_emails {
        text = EMAIL_RE.replace_all(&text, "[EMAIL]").into_owned();
    }
    if settings.hide_links {
        text = LINK_RE.replace_all(&text, "[LINK]").into_owned();
    }
    if settings.hide_addresses {
        text = ADDR_RU.replace_all(&text, "[ADDRESS]").into_owned();
        text = ADDR_RU_SHORT.replace_all(&text, "[ADDRESS]").into_owned();
    }
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

    let mut unique_names = HashSet::new();

    // ── Phase 1: collect unique sender names (only if hide_names enabled) ────
    if settings.hide_names {
        status("Step 1/2: Scanning sender names...".to_string());
        log("Phase 1: Collecting unique sender names...".to_string());

        for (idx, file_path) in files.iter().enumerate() {
            let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();
            log(format!("Scanning senders in: {}", file_name));

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
            for msg_element in document.select(&message_selector) {
                if let Some(name_element) = msg_element.select(&from_name_selector).next() {
                    // Use only direct text nodes to avoid capturing nested date elements
                    let trimmed = direct_text(name_element);
                    if !trimmed.is_empty() {
                        unique_names.insert(trimmed);
                    }
                }
            }

            // Update progress of phase 1 (up to 20% of total)
            let phase1_progress = 0.2 * ((idx + 1) as f32 / files.len() as f32);
            let _ = tx.send(ProgressMessage::Progress(phase1_progress));
        }

        log(format!(
            "Sender name scan complete. Found {} participants.",
            unique_names.len()
        ));
    } else {
        log("Name anonymization is disabled, skipping Phase 1.".to_string());
        let _ = tx.send(ProgressMessage::Progress(0.2));
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

    status("Step 2/2: Anonymizing & saving...".to_string());
    log(format!(
        "Phase 2: Cleaning and writing conversations ({} files)...",
        files.len()
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

    let mut current_sender = String::new();

    // ── Phase 2: anonymize & write ───────────────────────────────────────────
    for (idx, file_path) in files.iter().enumerate() {
        let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();
        log(format!(
            "Processing messages in: {} ({}/{})",
            file_name,
            idx + 1,
            files.len()
        ));

        let html_content = match std::fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(err) => {
                log(format!("ERROR reading {}: {}", file_name, err));
                let _ = tx.send(ProgressMessage::Error(format!(
                    "Failed to read {}: {}",
                    file_name, err
                )));
                return;
            }
        };
        {
            let document = Html::parse_document(&html_content);
            let msg_count = document.select(&message_selector).count();
            log(format!("  -> Found {} messages in {}", msg_count, file_name));

            for msg_element in document.select(&message_selector) {
                // 1. Extract/update sender name (direct text only — avoids nested date spans)
                if let Some(name_element) = msg_element.select(&from_name_selector).next() {
                    let trimmed = direct_text(name_element);
                    if !trimmed.is_empty() {
                        if settings.hide_names {
                            if let Some(placeholder) = name_map.get(&trimmed) {
                                current_sender = placeholder.clone();
                            } else {
                                current_sender = trimmed;
                            }
                        } else {
                            current_sender = trimmed;
                        }
                    }
                }

                // 2. Extract message text and apply all enabled redactions
                let message_text = if let Some(text_element) =
                    msg_element.select(&text_selector).next()
                {
                    let raw = extract_text(text_element);
                    if !raw.is_empty() {
                        Some(redact(raw, &settings, &names_re, &name_map_lower))
                    } else {
                        None
                    }
                } else if msg_element.select(&media_selector).next().is_some() {
                    Some("[MEDIA]".to_string())
                } else {
                    None
                };

                // 3. Write message in Markdown bold format
                if let Some(text) = message_text {
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
            }
        } // document memory released here

        // Update progress of phase 2 (from 20% to 100%)
        let phase2_progress = 0.2 + 0.8 * ((idx + 1) as f32 / files.len() as f32);
        let _ = tx.send(ProgressMessage::Progress(phase2_progress));
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
