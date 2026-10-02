use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use inputkey_core_abstractions::{LanguageMachinePort, LanguageState, LexiconPort};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    None,
    Acute,
    Grave,
    Hook,
    Tilde,
    Dot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Start,
    ViCandidate,
    RawLocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Start,
    Onset,
    Nucleus,
    Coda,
    Complete,
    PendingShape,
    PendingValidation,
    Dead,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VietnameseState {
    pub mode: Mode,
    pub phase: Phase,
    pub raw: String,
    pub fallback: String,
    pub rendered: String,
    pub tone: Tone,
    pub ambiguous: bool,
    pub transformed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub method: String,
    pub simple_telex: bool,
    pub auto_restore: bool,
    pub smart_correction: bool,
    pub double_cancel: bool,
    pub cancel_preference: String,
}

#[derive(Clone)]
struct Snapshot {
    mode: Mode,
    phase: Phase,
    raw: String,
    fallback: String,
    rendered: String,
    chars: Vec<char>,
    tone: Tone,
    tone_key: Option<char>,
    tone_fallback_index: isize,
    tone_was_bare_vowel: bool,
    transformed: bool,
    correction_blocked: bool,
    ambiguous: bool,
    ambig_raw: String,
    ambig_cancel: String,
    ambig_default: String,
    ambig_tail_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FsmEvent {
    Key(char),
    Backspace,
    Escape,
    CommitBoundary,
    CommitRawBoundary,
    Reset,
}

pub struct Machine {
    pub options: Options,
    mode: Mode,
    phase: Phase,
    raw: String,
    fallback: String,
    rendered: String,
    chars: Vec<char>,
    tone: Tone,
    tone_key: Option<char>,
    tone_fallback_index: isize,
    tone_was_bare_vowel: bool,
    transformed: bool,
    correction_blocked: bool,
    ambiguous: bool,
    ambig_raw: String,
    ambig_cancel: String,
    ambig_default: String,
    ambig_tail_count: usize,
    lexicon: Option<Box<dyn LexiconPort>>,
    history: Vec<Snapshot>,
}

#[derive(Clone, Copy)]
struct ToneRow {
    plain: char,
    acute: char,
    grave: char,
    hook: char,
    tilde: char,
    dot: char,
}

const TONE_ROWS: &[ToneRow] = &[
    ToneRow {
        plain: 'a',
        acute: 'á',
        grave: 'à',
        hook: 'ả',
        tilde: 'ã',
        dot: 'ạ',
    },
    ToneRow {
        plain: 'ă',
        acute: 'ắ',
        grave: 'ằ',
        hook: 'ẳ',
        tilde: 'ẵ',
        dot: 'ặ',
    },
    ToneRow {
        plain: 'â',
        acute: 'ấ',
        grave: 'ầ',
        hook: 'ẩ',
        tilde: 'ẫ',
        dot: 'ậ',
    },
    ToneRow {
        plain: 'e',
        acute: 'é',
        grave: 'è',
        hook: 'ẻ',
        tilde: 'ẽ',
        dot: 'ẹ',
    },
    ToneRow {
        plain: 'ê',
        acute: 'ế',
        grave: 'ề',
        hook: 'ể',
        tilde: 'ễ',
        dot: 'ệ',
    },
    ToneRow {
        plain: 'i',
        acute: 'í',
        grave: 'ì',
        hook: 'ỉ',
        tilde: 'ĩ',
        dot: 'ị',
    },
    ToneRow {
        plain: 'o',
        acute: 'ó',
        grave: 'ò',
        hook: 'ỏ',
        tilde: 'õ',
        dot: 'ọ',
    },
    ToneRow {
        plain: 'ô',
        acute: 'ố',
        grave: 'ồ',
        hook: 'ổ',
        tilde: 'ỗ',
        dot: 'ộ',
    },
    ToneRow {
        plain: 'ơ',
        acute: 'ớ',
        grave: 'ờ',
        hook: 'ở',
        tilde: 'ỡ',
        dot: 'ợ',
    },
    ToneRow {
        plain: 'u',
        acute: 'ú',
        grave: 'ù',
        hook: 'ủ',
        tilde: 'ũ',
        dot: 'ụ',
    },
    ToneRow {
        plain: 'ư',
        acute: 'ứ',
        grave: 'ừ',
        hook: 'ử',
        tilde: 'ữ',
        dot: 'ự',
    },
    ToneRow {
        plain: 'y',
        acute: 'ý',
        grave: 'ỳ',
        hook: 'ỷ',
        tilde: 'ỹ',
        dot: 'ỵ',
    },
    ToneRow {
        plain: 'A',
        acute: 'Á',
        grave: 'À',
        hook: 'Ả',
        tilde: 'Ã',
        dot: 'Ạ',
    },
    ToneRow {
        plain: 'Ă',
        acute: 'Ắ',
        grave: 'Ằ',
        hook: 'Ẳ',
        tilde: 'Ẵ',
        dot: 'Ặ',
    },
    ToneRow {
        plain: 'Â',
        acute: 'Ấ',
        grave: 'Ầ',
        hook: 'Ẩ',
        tilde: 'Ẫ',
        dot: 'Ậ',
    },
    ToneRow {
        plain: 'E',
        acute: 'É',
        grave: 'È',
        hook: 'Ẻ',
        tilde: 'Ẽ',
        dot: 'Ẹ',
    },
    ToneRow {
        plain: 'Ê',
        acute: 'Ế',
        grave: 'Ề',
        hook: 'Ể',
        tilde: 'Ễ',
        dot: 'Ệ',
    },
    ToneRow {
        plain: 'I',
        acute: 'Í',
        grave: 'Ì',
        hook: 'Ỉ',
        tilde: 'Ĩ',
        dot: 'Ị',
    },
    ToneRow {
        plain: 'O',
        acute: 'Ó',
        grave: 'Ò',
        hook: 'Ỏ',
        tilde: 'Õ',
        dot: 'Ọ',
    },
    ToneRow {
        plain: 'Ô',
        acute: 'Ố',
        grave: 'Ồ',
        hook: 'Ổ',
        tilde: 'Ỗ',
        dot: 'Ộ',
    },
    ToneRow {
        plain: 'Ơ',
        acute: 'Ớ',
        grave: 'Ờ',
        hook: 'Ở',
        tilde: 'Ỡ',
        dot: 'Ợ',
    },
    ToneRow {
        plain: 'U',
        acute: 'Ú',
        grave: 'Ù',
        hook: 'Ủ',
        tilde: 'Ũ',
        dot: 'Ụ',
    },
    ToneRow {
        plain: 'Ư',
        acute: 'Ứ',
        grave: 'Ừ',
        hook: 'Ử',
        tilde: 'Ữ',
        dot: 'Ự',
    },
    ToneRow {
        plain: 'Y',
        acute: 'Ý',
        grave: 'Ỳ',
        hook: 'Ỷ',
        tilde: 'Ỹ',
        dot: 'Ỵ',
    },
];

fn lower_char(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

fn upper_char(c: char) -> char {
    c.to_uppercase().next().unwrap_or(c)
}

fn strip_tone(c: char) -> char {
    for row in TONE_ROWS {
        if [
            row.plain, row.acute, row.grave, row.hook, row.tilde, row.dot,
        ]
        .contains(&c)
        {
            return row.plain;
        }
    }
    c
}

fn tone_of_rune(c: char) -> Tone {
    for row in TONE_ROWS {
        if c == row.acute {
            return Tone::Acute;
        }
        if c == row.grave {
            return Tone::Grave;
        }
        if c == row.hook {
            return Tone::Hook;
        }
        if c == row.tilde {
            return Tone::Tilde;
        }
        if c == row.dot {
            return Tone::Dot;
        }
        if c == row.plain {
            return Tone::None;
        }
    }
    Tone::None
}

fn add_tone(c: char, tone: Tone) -> char {
    let base = strip_tone(c);
    for row in TONE_ROWS {
        if row.plain == base {
            return match tone {
                Tone::Acute => row.acute,
                Tone::Grave => row.grave,
                Tone::Hook => row.hook,
                Tone::Tilde => row.tilde,
                Tone::Dot => row.dot,
                Tone::None => row.plain,
            };
        }
    }
    c
}

fn shaped_to_plain(c: char) -> char {
    match c {
        'ă' | 'â' => 'a',
        'ê' => 'e',
        'ô' | 'ơ' => 'o',
        'ư' => 'u',
        'Ă' | 'Â' => 'A',
        'Ê' => 'E',
        'Ô' | 'Ơ' => 'O',
        'Ư' => 'U',
        'đ' => 'd',
        'Đ' => 'D',
        _ => c,
    }
}

fn lower_base(c: char) -> char {
    lower_char(shaped_to_plain(strip_tone(c)))
}

fn is_vowel(c: char) -> bool {
    "aeiouy".contains(lower_base(c))
}

fn vowel_signature(c: char) -> char {
    lower_char(strip_tone(c))
}

fn has_vowel(chars: &[char]) -> bool {
    chars.iter().copied().any(is_vowel)
}

fn eligible_vowel_indexes(chars: &[char]) -> Vec<usize> {
    let mut indexes: Vec<usize> = chars
        .iter()
        .enumerate()
        .filter_map(|(i, c)| is_vowel(*c).then_some(i))
        .collect();
    if indexes.len() > 1 {
        if chars.len() >= 2 && lower_base(chars[0]) == 'q' && lower_base(chars[1]) == 'u' {
            indexes.retain(|i| *i != 1);
        }
        if chars.len() >= 2 && lower_base(chars[0]) == 'g' && lower_base(chars[1]) == 'i' {
            indexes.retain(|i| *i != 1);
        }
    }
    indexes
}

const MAIN_PATTERNS: &[(&str, usize)] = &[
    ("uyê", 2),
    ("iêu", 1),
    ("yêu", 1),
    ("ươu", 1),
    ("ươi", 1),
    ("uôi", 1),
    ("oai", 1),
    ("oay", 1),
    ("uây", 1),
    ("uya", 1),
    ("iê", 1),
    ("yê", 1),
    ("uê", 1),
    ("uâ", 1),
    ("uô", 1),
    ("ươ", 1),
    ("oă", 1),
    ("ia", 0),
    ("ai", 0),
    ("ao", 0),
    ("au", 0),
    ("ay", 0),
    ("eo", 0),
    ("oi", 0),
    ("ôi", 0),
    ("ơi", 0),
    ("ui", 0),
    ("ưi", 0),
];

fn choose_tone_index(chars: &[char]) -> Option<usize> {
    let indexes = eligible_vowel_indexes(chars);
    if indexes.is_empty() {
        return None;
    }
    if indexes.len() == 1 {
        return Some(indexes[0]);
    }
    let signature: String = indexes.iter().map(|i| vowel_signature(chars[*i])).collect();
    let last = *indexes.last().expect("indexes is non-empty");
    let has_coda = chars[last + 1..].iter().any(|c| c.is_alphabetic());

    if ["oa", "oe", "uy", "ua", "ưa"].contains(&signature.as_str()) {
        return Some(if has_coda { indexes[1] } else { indexes[0] });
    }
    for (pattern, pos) in MAIN_PATTERNS {
        if signature == *pattern && *pos < indexes.len() {
            return Some(indexes[*pos]);
        }
    }
    for wanted in ['ê', 'ơ', 'ô', 'â', 'ă'] {
        if let Some(index) = indexes
            .iter()
            .copied()
            .find(|i| vowel_signature(chars[*i]) == wanted)
        {
            return Some(index);
        }
    }
    if has_coda && indexes.len() >= 2 {
        return Some(indexes[1]);
    }
    Some(indexes[0])
}

fn apply_tone(chars: &[char], tone: Tone) -> Vec<char> {
    let mut out: Vec<char> = chars.iter().copied().map(strip_tone).collect();
    if tone == Tone::None {
        return out;
    }
    if let Some(i) = choose_tone_index(&out) {
        out[i] = add_tone(out[i], tone);
    }
    out
}

fn last_letter_index(chars: &[char]) -> Option<usize> {
    chars.iter().rposition(|c| c.is_alphabetic())
}

fn apply_repeat_shape(chars: &mut Vec<char>, key: char) -> (bool, bool) {
    let Some(i) = last_letter_index(chars) else {
        return (false, false);
    };
    let ch = strip_tone(chars[i]);
    let key_lower = lower_char(key);
    if lower_base(ch) != key_lower || !"aeo".contains(key_lower) {
        return (false, false);
    }
    let mut desired = match key_lower {
        'a' => 'â',
        'e' => 'ê',
        'o' => 'ô',
        _ => return (false, false),
    };
    if ch.is_uppercase() {
        desired = upper_char(desired);
    }
    if strip_tone(ch) == desired {
        let mut plain = key_lower;
        if ch.is_uppercase() {
            plain = upper_char(plain);
        }
        chars[i] = plain;
        chars.push(key);
        return (true, true);
    }
    if lower_char(ch) == key_lower {
        chars[i] = desired;
        return (true, false);
    }
    (false, false)
}

#[derive(Clone)]
struct Candidate {
    chars: Vec<char>,
    score: isize,
}

fn apply_late_repeat_shape(chars: &mut Vec<char>, key: char, tone: Tone) -> bool {
    let source = chars.clone();
    let lower = lower_char(key);
    if !"aeo".contains(lower) {
        return false;
    }
    let desired = match lower {
        'a' => 'â',
        'e' => 'ê',
        'o' => 'ô',
        _ => return false,
    };
    let mut candidates = Vec::new();
    for i in (0..source.len()).rev() {
        let ch = strip_tone(source[i]);
        if lower_char(ch) != lower {
            continue;
        }
        let mut trial = source.clone();
        trial[i] = if ch.is_uppercase() {
            upper_char(desired)
        } else {
            desired
        };
        let rendered: String = apply_tone(&trial, tone).into_iter().collect();
        let analysis = analyze_transducer(&rendered);
        if !analysis.viable {
            continue;
        }
        let mut score = i as isize;
        if analysis.complete {
            score += 1000;
        }
        if matches!(analysis.phase, Phase::Nucleus | Phase::Complete) {
            score += 100;
        }
        candidates.push(Candidate {
            chars: trial,
            score,
        });
    }
    let Some(best) = candidates.into_iter().max_by_key(|c| c.score) else {
        return false;
    };
    *chars = best.chars;
    true
}

fn apply_d_stroke(chars: &mut Vec<char>, key: char) -> (bool, bool) {
    let Some(i) = last_letter_index(chars) else {
        return (false, false);
    };
    if lower_base(chars[i]) != 'd' {
        return (false, false);
    }
    let ch = strip_tone(chars[i]);
    if matches!(ch, 'đ' | 'Đ') {
        chars[i] = if ch.is_uppercase() { 'D' } else { 'd' };
        chars.push(key);
        return (true, true);
    }
    if matches!(ch, 'd' | 'D') {
        chars[i] = if ch.is_uppercase() { 'Đ' } else { 'đ' };
        return (true, false);
    }
    (false, false)
}

fn apply_late_d_stroke(chars: &mut Vec<char>, tone: Tone) -> bool {
    let source = chars.clone();
    let Some(first) = source.iter().position(|c| c.is_alphabetic()) else {
        return false;
    };
    let ch = strip_tone(source[first]);
    if !matches!(ch, 'd' | 'D') {
        return false;
    }
    let mut trial = source;
    trial[first] = if ch == 'D' { 'Đ' } else { 'đ' };
    let rendered: String = apply_tone(&trial, tone).into_iter().collect();
    if is_valid_syllable(&rendered) {
        *chars = trial;
        true
    } else {
        false
    }
}

fn add_w_candidate(candidates: &mut Vec<Candidate>, trial: Vec<char>, bonus: isize, tone: Tone) {
    let rendered: String = apply_tone(&trial, tone).into_iter().collect();
    let analysis = analyze_transducer(&rendered);
    if !analysis.viable {
        return;
    }
    let mut score = bonus;
    if analysis.complete {
        score += 100;
    }
    if matches!(analysis.phase, Phase::Nucleus | Phase::Complete) {
        score += 20;
    }
    candidates.push(Candidate {
        chars: trial,
        score,
    });
}

fn apply_w_shape(chars: &mut Vec<char>, key: char, standalone: bool, tone: Tone) -> (bool, bool) {
    let source = chars.clone();
    let Some(li) = last_letter_index(&source) else {
        if standalone {
            chars.push(if key == 'W' { 'Ư' } else { 'ư' });
            return (true, false);
        }
        return (false, false);
    };

    let last = strip_tone(source[li]);
    let pi = (0..li).rev().find(|i| source[*i].is_alphabetic());
    let prev = pi.map(|i| strip_tone(source[i]));

    if let (Some(pi), Some(prev)) = (pi, prev) {
        if lower_char(prev) == 'ư' && lower_char(last) == 'ơ' {
            let mut out = source;
            out[pi] = if prev.is_uppercase() { 'U' } else { 'u' };
            out[li] = if last.is_uppercase() { 'O' } else { 'o' };
            out.push(key);
            *chars = out;
            return (true, true);
        }
    }

    let unshaped = match last {
        'ă' => Some('a'),
        'ơ' => Some('o'),
        'ư' => Some('u'),
        'Ă' => Some('A'),
        'Ơ' => Some('O'),
        'Ư' => Some('U'),
        _ => None,
    };
    if let Some(plain) = unshaped {
        let mut out = source;
        out[li] = plain;
        out.push(key);
        *chars = out;
        return (true, true);
    }

    let mut candidates = Vec::new();
    if let (Some(pi), Some(prev)) = (pi, prev) {
        if lower_char(prev) == 'u' && lower_char(last) == 'o' {
            let mut trial = source.clone();
            trial[pi] = if prev.is_uppercase() { 'Ư' } else { 'ư' };
            trial[li] = if last.is_uppercase() { 'Ơ' } else { 'ơ' };
            add_w_candidate(&mut candidates, trial, 300, tone);
        }
    }

    for i in (0..=li).rev() {
        let ch = strip_tone(source[i]);
        let shaped = match ch {
            'a' => Some('ă'),
            'o' => Some('ơ'),
            'u' => Some('ư'),
            'A' => Some('Ă'),
            'O' => Some('Ơ'),
            'U' => Some('Ư'),
            _ => None,
        };
        if let Some(shaped) = shaped {
            let mut trial = source.clone();
            trial[i] = shaped;
            add_w_candidate(&mut candidates, trial, 10 + i as isize, tone);
        }
    }

    if let Some(best) = candidates.into_iter().max_by_key(|c| c.score) {
        *chars = best.chars;
        return (true, false);
    }

    if standalone && !has_vowel(&source) {
        let mut out = source;
        out.push(if key == 'W' { 'Ư' } else { 'ư' });
        *chars = out;
        return (true, false);
    }
    (false, false)
}

const ONSETS: &[&str] = &[
    "ngh", "ch", "gh", "gi", "kh", "ng", "nh", "ph", "qu", "th", "tr", "b", "c", "d", "đ", "g",
    "h", "k", "l", "m", "n", "p", "r", "s", "t", "v", "x", "",
];
const CODAS: &[&str] = &["ch", "ng", "nh", "c", "m", "n", "p", "t", ""];
const OPEN_NUCLEI: &[&str] = &[
    "a", "â", "ă", "e", "ê", "i", "o", "ô", "ơ", "u", "ư", "y", "ai", "ao", "au", "ay", "âu", "ây",
    "eo", "êu", "ia", "iê", "iu", "oa", "oă", "oe", "oi", "ôi", "ơi", "ua", "uâ", "uê", "ui", "uô",
    "uơ", "uy", "ưa", "ưi", "ươ", "ưu", "yê", "iêu", "oai", "oay", "oeo", "uây", "uôi", "uya",
    "uyê", "uyu", "ươi", "ươu", "yêu",
];
const CODA_ROWS: &[(&str, &str)] = &[
    ("a", "c ch m n ng nh p t"),
    ("â", "c m n ng p t"),
    ("ă", "c m n ng p t"),
    ("e", "c ch m n ng nh p t"),
    ("ê", "c ch m n nh p t"),
    ("i", "c ch m n nh p t"),
    ("o", "c m n ng p t"),
    ("ô", "c m n ng p t"),
    ("ơ", "m n p t"),
    ("u", "c m n ng p t"),
    ("ư", "c m n ng t"),
    ("y", "t"),
    ("iê", "c m n ng p t"),
    ("oa", "c ch m n ng nh p t"),
    ("oă", "c m n ng t"),
    ("oe", "n t"),
    ("ua", "n ng t"),
    ("uâ", "n ng t"),
    ("uê", "c ch n nh"),
    ("uô", "c m n ng t"),
    ("uơ", "c m n ng p t"),
    ("ươ", "c m n ng p t"),
    ("uy", "c ch n nh p t"),
    ("yê", "m n ng p t"),
    ("uyê", "n t"),
];

fn is_open_nucleus(nucleus: &str) -> bool {
    OPEN_NUCLEI.contains(&nucleus)
}

fn coda_allowed(nucleus: &str, coda: &str) -> bool {
    CODA_ROWS
        .iter()
        .find(|(n, _)| *n == nucleus)
        .is_some_and(|(_, row)| row.split_whitespace().any(|x| x == coda))
}

fn valid_onset_nucleus(onset: &str, nucleus: &str) -> bool {
    let Some(first) = nucleus.chars().next() else {
        return false;
    };
    let first = lower_base(first);
    if onset == "gi" && first == 'i' {
        return false;
    }
    if onset == "qu" && first == 'u' {
        return false;
    }
    // In the oe nucleus, the leading o carries the /w/ medial. Vietnamese does
    // not combine that medial with a labial onset, so forms such as moe are not
    // valid syllable skeletons while khoe, hoe, loe, xoe, ... remain.
    if nucleus.starts_with("oe") && matches!(onset, "b" | "m" | "p" | "ph" | "v") {
        return false;
    }
    if onset == "k" && !"eiy".contains(first) {
        return false;
    }
    true
}

fn valid_nucleus_coda(onset: &str, nucleus: &str, coda: &str) -> bool {
    if coda.is_empty() {
        return is_open_nucleus(nucleus) && (!matches!(nucleus, "â" | "ă") || onset.is_empty());
    }
    if coda_allowed(nucleus, coda) {
        return true;
    }
    if onset == "qu" && nucleus == "y" && matches!(coda, "n" | "nh") {
        return true;
    }
    onset == "gi" && matches!(nucleus, "e" | "ê") && matches!(coda, "n" | "ng")
}

struct Grammar {
    valid_skeletons: HashSet<String>,
    valid_prefixes: HashSet<String>,
}

fn grammar() -> &'static Grammar {
    static GRAMMAR: OnceLock<Grammar> = OnceLock::new();
    GRAMMAR.get_or_init(|| {
        let mut nuclei: HashSet<&str> = OPEN_NUCLEI.iter().copied().collect();
        nuclei.extend(CODA_ROWS.iter().map(|(n, _)| *n));
        let mut valid_skeletons = HashSet::new();
        let mut valid_prefixes = HashSet::from([String::new()]);
        for onset in ONSETS {
            for nucleus in &nuclei {
                if !valid_onset_nucleus(onset, nucleus) {
                    continue;
                }
                for coda in CODAS {
                    if !valid_nucleus_coda(onset, nucleus, coda) {
                        continue;
                    }
                    let skeleton = format!("{onset}{nucleus}{coda}");
                    valid_skeletons.insert(skeleton.clone());
                    let chars: Vec<char> = skeleton.chars().collect();
                    for i in 1..=chars.len() {
                        valid_prefixes.insert(chars[..i].iter().collect());
                    }
                }
            }
        }
        Grammar {
            valid_skeletons,
            valid_prefixes,
        }
    })
}

fn shape_skeleton(text: &str) -> String {
    text.chars().map(|c| lower_char(strip_tone(c))).collect()
}

fn tone_of_text(text: &str) -> Tone {
    text.chars()
        .map(tone_of_rune)
        .find(|t| *t != Tone::None)
        .unwrap_or(Tone::None)
}

fn is_valid_syllable(rendered: &str) -> bool {
    let skeleton = shape_skeleton(rendered);
    if !grammar().valid_skeletons.contains(&skeleton) {
        return false;
    }
    let tone = tone_of_text(rendered);
    for onset in ONSETS {
        if !skeleton.starts_with(onset) {
            continue;
        }
        let after = &skeleton[onset.len()..];
        for coda in CODAS {
            if !coda.is_empty() && !after.ends_with(coda) {
                continue;
            }
            let nucleus = if coda.is_empty() {
                after
            } else {
                &after[..after.len() - coda.len()]
            };
            if nucleus.is_empty() || !nucleus.chars().all(is_vowel) {
                continue;
            }
            if !valid_onset_nucleus(onset, nucleus) || !valid_nucleus_coda(onset, nucleus, coda) {
                continue;
            }
            if matches!(*coda, "c" | "ch" | "p" | "t") && !matches!(tone, Tone::Acute | Tone::Dot) {
                continue;
            }
            return true;
        }
    }
    false
}

fn tone_key(tone: Tone) -> Option<char> {
    match tone {
        Tone::Acute => Some('s'),
        Tone::Grave => Some('f'),
        Tone::Hook => Some('r'),
        Tone::Tilde => Some('x'),
        Tone::Dot => Some('j'),
        Tone::None => None,
    }
}

fn telex_encoding(rendered: &str) -> String {
    let chars: Vec<char> = rendered
        .chars()
        .map(|c| lower_char(strip_tone(c)))
        .collect();
    let mut out = String::new();
    let mut index = 0usize;
    while index < chars.len() {
        if index + 1 < chars.len() && chars[index] == 'ư' && chars[index + 1] == 'ơ' {
            out.push_str("uow");
            index += 2;
            continue;
        }
        match chars[index] {
            'đ' => out.push_str("dd"),
            'â' => out.push_str("aa"),
            'ă' => out.push_str("aw"),
            'ê' => out.push_str("ee"),
            'ô' => out.push_str("oo"),
            'ơ' => out.push_str("ow"),
            'ư' => out.push_str("uw"),
            c => out.push(c),
        }
        index += 1;
    }
    if let Some(key) = tone_key(tone_of_text(rendered)) {
        out.push(key);
    }
    out
}

fn key_multiset_signature(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().flat_map(char::to_lowercase).collect();
    chars.sort_unstable();
    chars.into_iter().collect()
}

#[derive(Clone)]
struct TelexIntentCandidate {
    rendered: String,
    base: Vec<u8>,
    modifiers: [u8; 26],
    encoded_len: usize,
}

fn telex_intent_candidate(rendered: String) -> Option<TelexIntentCandidate> {
    let base_text: String = rendered.chars().map(lower_base).collect();
    let encoded = telex_encoding(&rendered);
    if !base_text.bytes().all(|b| b.is_ascii_lowercase())
        || !encoded.bytes().all(|b| b.is_ascii_lowercase())
    {
        return None;
    }

    let mut modifiers = [0u8; 26];
    for byte in encoded.bytes() {
        let index = (byte - b'a') as usize;
        modifiers[index] = modifiers[index].saturating_add(1);
    }
    for byte in base_text.bytes() {
        let index = (byte - b'a') as usize;
        if modifiers[index] == 0 {
            return None;
        }
        modifiers[index] -= 1;
    }

    Some(TelexIntentCandidate {
        rendered,
        base: base_text.into_bytes(),
        modifiers,
        encoded_len: encoded.len(),
    })
}

fn telex_intent_index() -> &'static HashMap<String, Vec<TelexIntentCandidate>> {
    static INDEX: OnceLock<HashMap<String, Vec<TelexIntentCandidate>>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let tones = [
            Tone::None,
            Tone::Acute,
            Tone::Grave,
            Tone::Hook,
            Tone::Tilde,
            Tone::Dot,
        ];
        let mut index: HashMap<String, Vec<TelexIntentCandidate>> = HashMap::new();
        for skeleton in &grammar().valid_skeletons {
            let chars: Vec<char> = skeleton.chars().collect();
            for tone in tones {
                let rendered: String = apply_tone(&chars, tone).into_iter().collect();
                if !is_valid_syllable(&rendered) {
                    continue;
                }
                let encoding = telex_encoding(&rendered);
                let signature = key_multiset_signature(&encoding);
                let Some(candidate) = telex_intent_candidate(rendered) else {
                    continue;
                };
                let candidates = index.entry(signature).or_default();
                if !candidates
                    .iter()
                    .any(|existing| existing.rendered == candidate.rendered)
                {
                    candidates.push(candidate);
                }
            }
        }
        index
    })
}

fn ordered_intent_assignment(
    raw: &[u8],
    base: &[u8],
    raw_index: usize,
    base_index: usize,
    modifiers: &mut [u8; 26],
    require_complete: bool,
) -> bool {
    if raw_index == raw.len() {
        return !require_complete
            || (base_index == base.len() && modifiers.iter().all(|count| *count == 0));
    }

    let key = raw[raw_index];
    if base_index < base.len()
        && key == base[base_index]
        && ordered_intent_assignment(
            raw,
            base,
            raw_index + 1,
            base_index + 1,
            modifiers,
            require_complete,
        )
    {
        return true;
    }

    let modifier_index = (key - b'a') as usize;
    if modifiers[modifier_index] > 0 {
        modifiers[modifier_index] -= 1;
        let matched = ordered_intent_assignment(
            raw,
            base,
            raw_index + 1,
            base_index,
            modifiers,
            require_complete,
        );
        modifiers[modifier_index] += 1;
        if matched {
            return true;
        }
    }

    false
}

fn ordered_telex_intent_matches(
    raw: &str,
    candidate: &TelexIntentCandidate,
    require_complete: bool,
) -> bool {
    if raw.is_empty() || !raw.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    let raw = raw.to_ascii_lowercase();
    if raw.len() > candidate.encoded_len {
        return false;
    }
    let mut modifiers = candidate.modifiers;
    ordered_intent_assignment(
        raw.as_bytes(),
        &candidate.base,
        0,
        0,
        &mut modifiers,
        require_complete,
    )
}

fn apply_input_case(raw: &str, rendered: &str) -> String {
    let letters: Vec<char> = raw.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        return rendered.to_owned();
    }
    if letters.iter().all(|c| c.is_uppercase()) {
        return rendered.chars().flat_map(char::to_uppercase).collect();
    }
    if letters[0].is_uppercase() && letters.iter().skip(1).all(|c| c.is_lowercase()) {
        let mut chars = rendered.chars();
        let Some(first) = chars.next() else {
            return String::new();
        };
        let mut out: String = first.to_uppercase().collect();
        out.extend(chars);
        return out;
    }
    rendered.to_owned()
}

fn intent_preference(raw: &str, rendered: &str) -> isize {
    let skeleton = shape_skeleton(rendered);
    let lower_raw = raw.to_ascii_lowercase();
    let mut score = 0isize;
    if lower_raw.contains('w') {
        if skeleton.contains("ươ") {
            score += 300;
        } else if skeleton.contains('ă') || skeleton.contains('ơ') || skeleton.contains('ư') {
            score += 20;
        }
    }
    if is_valid_syllable(rendered) {
        score += 100;
    }
    score
}

fn resolve_telex_intent(raw: &str) -> Option<String> {
    if raw.len() < 3 || !raw.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let signature = key_multiset_signature(raw);
    let candidates = telex_intent_index().get(&signature)?;
    let mut ranked: Vec<(isize, &TelexIntentCandidate)> = candidates
        .iter()
        .filter(|candidate| ordered_telex_intent_matches(raw, candidate, true))
        .map(|candidate| (intent_preference(raw, &candidate.rendered), candidate))
        .collect();
    ranked.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.rendered.cmp(&right.1.rendered))
    });
    let (best_score, best) = ranked.first().copied()?;
    if ranked
        .get(1)
        .is_some_and(|(second_score, _)| *second_score == best_score)
    {
        return None;
    }
    Some(apply_input_case(raw, &best.rendered))
}

#[derive(Clone, Copy)]
struct Analysis {
    viable: bool,
    complete: bool,
    phase: Phase,
}

fn analyze_prefix(rendered: &str) -> Analysis {
    let skeleton = shape_skeleton(rendered);
    if skeleton.is_empty() {
        return Analysis {
            viable: true,
            complete: false,
            phase: Phase::Start,
        };
    }
    if !grammar().valid_prefixes.contains(&skeleton) {
        return Analysis {
            viable: false,
            complete: false,
            phase: Phase::Dead,
        };
    }
    if grammar().valid_skeletons.contains(&skeleton) {
        return Analysis {
            viable: true,
            complete: true,
            phase: Phase::Complete,
        };
    }
    let mut seen_vowel = false;
    let mut seen_coda = false;
    for c in skeleton.chars() {
        if is_vowel(c) {
            if seen_coda {
                return Analysis {
                    viable: true,
                    complete: false,
                    phase: Phase::Coda,
                };
            }
            seen_vowel = true;
        } else if seen_vowel {
            seen_coda = true;
        }
    }
    if !seen_vowel {
        Analysis {
            viable: true,
            complete: false,
            phase: Phase::Onset,
        }
    } else if seen_coda {
        Analysis {
            viable: true,
            complete: false,
            phase: Phase::Coda,
        }
    } else {
        Analysis {
            viable: true,
            complete: false,
            phase: Phase::Nucleus,
        }
    }
}

fn hypothetical(rendered: &str) -> Vec<String> {
    let skeleton = shape_skeleton(rendered);
    if skeleton.is_empty() {
        return Vec::new();
    }
    let chars: Vec<char> = skeleton.chars().collect();
    let last = *chars.last().expect("non-empty");
    let mut out = Vec::new();
    let mut replace_last = |replacement: char| {
        let mut trial = chars.clone();
        let len = trial.len();
        trial[len - 1] = replacement;
        out.push(trial.iter().collect());
    };
    match last {
        'a' => {
            replace_last('â');
            replace_last('ă');
        }
        'e' => replace_last('ê'),
        'o' => {
            replace_last('ô');
            replace_last('ơ');
        }
        'u' => replace_last('ư'),
        'd' => replace_last('đ'),
        _ => {}
    }
    if chars.ends_with(&['u', 'o']) {
        let mut trial = chars[..chars.len() - 2].to_vec();
        trial.extend(['ư', 'ơ']);
        out.push(trial.into_iter().collect());
    }
    out
}

fn analyze_transducer(rendered: &str) -> Analysis {
    let direct = analyze_prefix(rendered);
    if direct.viable {
        return direct;
    }
    for candidate in hypothetical(rendered) {
        if analyze_prefix(&candidate).viable {
            return Analysis {
                viable: true,
                complete: false,
                phase: Phase::PendingShape,
            };
        }
    }
    direct
}

fn rune_remove_at(text: &str, index: isize) -> String {
    if index < 0 {
        return text.to_owned();
    }
    let mut chars: Vec<char> = text.chars().collect();
    let index = index as usize;
    if index >= chars.len() {
        return text.to_owned();
    }
    chars.remove(index);
    chars.into_iter().collect()
}

fn drop_last_rune(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    chars.pop();
    chars.into_iter().collect()
}

impl Machine {
    pub fn new(mut options: Options, lexicon: Option<Box<dyn LexiconPort>>) -> Self {
        if options.method.is_empty() {
            options.method = "telex".to_owned();
        }
        if options.cancel_preference.is_empty() {
            options.cancel_preference = "explicit".to_owned();
        }
        Self {
            options,
            mode: Mode::Start,
            phase: Phase::Start,
            raw: String::new(),
            fallback: String::new(),
            rendered: String::new(),
            chars: Vec::new(),
            tone: Tone::None,
            tone_key: None,
            tone_fallback_index: -1,
            tone_was_bare_vowel: false,
            transformed: false,
            correction_blocked: false,
            ambiguous: false,
            ambig_raw: String::new(),
            ambig_cancel: String::new(),
            ambig_default: String::new(),
            ambig_tail_count: 0,
            lexicon,
            history: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.mode = Mode::Start;
        self.phase = Phase::Start;
        self.raw.clear();
        self.fallback.clear();
        self.rendered.clear();
        self.chars.clear();
        self.tone = Tone::None;
        self.tone_key = None;
        self.tone_fallback_index = -1;
        self.tone_was_bare_vowel = false;
        self.transformed = false;
        self.correction_blocked = false;
        self.ambiguous = false;
        self.ambig_raw.clear();
        self.ambig_cancel.clear();
        self.ambig_default.clear();
        self.ambig_tail_count = 0;
    }

    fn capture(&self) -> Snapshot {
        Snapshot {
            mode: self.mode,
            phase: self.phase,
            raw: self.raw.clone(),
            fallback: self.fallback.clone(),
            rendered: self.rendered.clone(),
            chars: self.chars.clone(),
            tone: self.tone,
            tone_key: self.tone_key,
            tone_fallback_index: self.tone_fallback_index,
            tone_was_bare_vowel: self.tone_was_bare_vowel,
            transformed: self.transformed,
            correction_blocked: self.correction_blocked,
            ambiguous: self.ambiguous,
            ambig_raw: self.ambig_raw.clone(),
            ambig_cancel: self.ambig_cancel.clone(),
            ambig_default: self.ambig_default.clone(),
            ambig_tail_count: self.ambig_tail_count,
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.mode = snapshot.mode;
        self.phase = snapshot.phase;
        self.raw = snapshot.raw;
        self.fallback = snapshot.fallback;
        self.rendered = snapshot.rendered;
        self.chars = snapshot.chars;
        self.tone = snapshot.tone;
        self.tone_key = snapshot.tone_key;
        self.tone_fallback_index = snapshot.tone_fallback_index;
        self.tone_was_bare_vowel = snapshot.tone_was_bare_vowel;
        self.transformed = snapshot.transformed;
        self.correction_blocked = snapshot.correction_blocked;
        self.ambiguous = snapshot.ambiguous;
        self.ambig_raw = snapshot.ambig_raw;
        self.ambig_cancel = snapshot.ambig_cancel;
        self.ambig_default = snapshot.ambig_default;
        self.ambig_tail_count = snapshot.ambig_tail_count;
    }

    fn adopt_smart_candidate(&mut self, rendered: String) -> String {
        self.tone = tone_of_text(&rendered);
        self.tone_key = tone_key(self.tone);
        self.tone_fallback_index = self
            .tone_key
            .and_then(|key| {
                self.fallback
                    .chars()
                    .enumerate()
                    .filter(|(_, c)| lower_char(*c) == key)
                    .map(|(index, _)| index as isize)
                    .last()
            })
            .unwrap_or(-1);
        self.tone_was_bare_vowel = false;
        self.chars = rendered.chars().map(strip_tone).collect();
        self.rendered = rendered;
        let analysis = analyze_prefix(&self.rendered);
        self.phase = analysis.phase;
        self.mode = Mode::ViCandidate;
        self.transformed = true;
        self.rendered.clone()
    }

    fn render_candidate(&mut self) -> String {
        self.rendered = apply_tone(&self.chars, self.tone).into_iter().collect();
        let analysis = analyze_transducer(&self.rendered);
        self.phase = analysis.phase;

        if !analysis.viable && self.transformed {
            // The current event may leave a speculative Telex rendering that is
            // not a valid Vietnamese prefix. Keep it visible for this event only;
            // the next printable event will fall back to the physical raw stream.
            self.mode = Mode::ViCandidate;
            self.phase = Phase::PendingValidation;
        } else if self.options.auto_restore && !analysis.viable {
            self.mode = Mode::RawLocked;
            self.rendered = self.fallback.clone();
        } else if !self.raw.is_empty() {
            self.mode = Mode::ViCandidate;
        } else {
            self.mode = Mode::Start;
        }
        self.rendered.clone()
    }

    fn clear_tone(&mut self) {
        self.tone = Tone::None;
        self.tone_key = None;
        self.tone_fallback_index = -1;
        self.tone_was_bare_vowel = false;
    }

    fn set_tone(&mut self, tone: Tone, key: char) {
        self.tone = tone;
        self.tone_key = Some(lower_char(key));
        self.tone_fallback_index = self.fallback.chars().count() as isize - 1;
        self.tone_was_bare_vowel =
            self.chars.len() == 1 && "aeo".contains(vowel_signature(self.chars[0]));
        self.transformed = true;
    }

    fn cancel_tone(&mut self) {
        self.fallback = drop_last_rune(&self.fallback);
        if self.tone_fallback_index >= 0 {
            self.fallback = rune_remove_at(&self.fallback, self.tone_fallback_index);
        }
        self.clear_tone();
        self.transformed = true;
    }

    fn lock_literal_escape(&mut self) {
        self.fallback = drop_last_rune(&self.fallback);
        self.correction_blocked = true;
        self.mode = Mode::RawLocked;
        self.phase = Phase::Dead;
        self.rendered = self.fallback.clone();
        self.transformed = true;
    }

    fn repeats_shape(&self, key: char) -> bool {
        if self.history.len() < 2 {
            return false;
        }
        let current = &self.history[self.history.len() - 1];
        let Some(last_raw) = current.raw.chars().last() else {
            return false;
        };
        if lower_char(last_raw) != lower_char(key) {
            return false;
        }
        let before = &self.history[self.history.len() - 2];
        if before.tone != current.tone || before.chars.len() != current.chars.len() {
            return false;
        }
        let mut changed = false;
        for (current_char, before_char) in current.chars.iter().zip(&before.chars) {
            if current_char == before_char {
                continue;
            }
            if shaped_to_plain(*current_char) != *before_char {
                return false;
            }
            changed = true;
        }
        changed
    }

    fn repeats_bracket_shape(&self, key: char) -> bool {
        if self.options.simple_telex
            || !self.options.method.eq_ignore_ascii_case("telex")
            || self.history.len() < 2
        {
            return false;
        }
        let expected = match key {
            '[' => 'ư',
            ']' => 'ơ',
            _ => return false,
        };
        let current = &self.history[self.history.len() - 1];
        let before = &self.history[self.history.len() - 2];
        if current.raw != format!("{}{}", before.raw, key) || current.tone != before.tone {
            return false;
        }
        if current.chars.len() != before.chars.len() + 1 {
            return false;
        }
        if current.chars[..before.chars.len()] != before.chars {
            return false;
        }
        current
            .chars
            .last()
            .is_some_and(|c| lower_char(strip_tone(*c)) == expected)
    }

    fn choose_ambiguous_literal(&self, _final: bool) -> String {
        let raw_hit = self
            .lexicon
            .as_ref()
            .is_some_and(|lexicon| lexicon.has_word(&self.ambig_raw));
        let cancel_hit = self
            .lexicon
            .as_ref()
            .is_some_and(|lexicon| lexicon.has_word(&self.ambig_cancel));
        if self.ambig_tail_count == 0 {
            return self.ambig_cancel.clone();
        }
        if raw_hit && !cancel_hit {
            return self.ambig_raw.clone();
        }
        if cancel_hit && !raw_hit {
            return self.ambig_cancel.clone();
        }
        if self.ambig_default == "raw" {
            self.ambig_raw.clone()
        } else {
            self.ambig_cancel.clone()
        }
    }

    fn repeat_tone_literal(&mut self, key: char) {
        self.clear_tone();
        self.chars.push(key);
        self.lock_literal_escape();
    }

    fn recover_known_raw_after_cancel(&mut self) -> bool {
        if !self.options.auto_restore || self.lexicon.is_none() || self.raw == self.fallback {
            return false;
        }
        let lexicon = self.lexicon.as_ref().expect("checked");
        if !lexicon.has_word(&self.raw) || lexicon.has_word(&self.fallback) {
            return false;
        }
        self.fallback = self.raw.clone();
        self.chars = self.raw.chars().collect();
        self.clear_tone();
        self.rendered = self.raw.clone();
        self.transformed = false;
        true
    }

    fn finish_explicit_cancel(&mut self) {
        self.correction_blocked = true;
        self.ambiguous = false;
        self.ambig_raw.clear();
        self.ambig_cancel.clear();
        self.ambig_default.clear();
        self.ambig_tail_count = 0;
        self.mode = Mode::ViCandidate;
        self.phase = Phase::Complete;
        self.transformed = false;
        self.rendered = self.fallback.clone();
    }

    fn prefer_raw_tone_before_shape(&self, key: char) -> bool {
        self.options.auto_restore
            && self.options.method == "telex"
            && self.tone != Tone::None
            && self.tone_was_bare_vowel
            && self.chars.len() == 1
            && "aeo".contains(lower_char(key))
            && vowel_signature(self.chars[0]) == lower_char(key)
    }

    fn transition_telex(&mut self, key: char) {
        let lower = lower_char(key);
        let tone = match lower {
            's' => Some(Tone::Acute),
            'f' => Some(Tone::Grave),
            'r' => Some(Tone::Hook),
            'x' => Some(Tone::Tilde),
            'j' => Some(Tone::Dot),
            _ => None,
        };

        if tone.is_some() || lower == 'z' {
            if has_vowel(&self.chars) {
                if lower == 'z' {
                    if self.tone != Tone::None {
                        self.cancel_tone();
                        return;
                    }
                } else if self.tone == tone.expect("tone key") {
                    self.repeat_tone_literal(key);
                    return;
                } else {
                    self.set_tone(tone.expect("tone key"), key);
                    return;
                }
            }
            self.chars.push(key);
            return;
        }

        if "aeo".contains(lower) {
            if let Some(i) = last_letter_index(&self.chars) {
                if lower_base(self.chars[i]) == lower {
                    let (changed, escape) = apply_repeat_shape(&mut self.chars, key);
                    if changed {
                        if escape {
                            self.lock_literal_escape();
                        } else {
                            self.transformed = true;
                        }
                        return;
                    }
                }
            }
            if apply_late_repeat_shape(&mut self.chars, key, self.tone) {
                self.transformed = true;
                return;
            }
        }

        if lower == 'd' {
            if let Some(i) = last_letter_index(&self.chars) {
                if lower_base(self.chars[i]) == 'd' {
                    let (changed, escape) = apply_d_stroke(&mut self.chars, key);
                    if changed {
                        if escape {
                            self.lock_literal_escape();
                        } else {
                            self.transformed = true;
                        }
                        return;
                    }
                }
            }
            if apply_late_d_stroke(&mut self.chars, self.tone) {
                self.transformed = true;
                return;
            }
        }

        if lower == 'w' {
            let (changed, escape) =
                apply_w_shape(&mut self.chars, key, !self.options.simple_telex, self.tone);
            if changed {
                if escape {
                    self.lock_literal_escape();
                } else {
                    self.transformed = true;
                }
                return;
            }
        }

        if !self.options.simple_telex && key == '[' {
            self.chars.push('ư');
            self.transformed = true;
            return;
        }
        if !self.options.simple_telex && key == ']' {
            self.chars.push('ơ');
            self.transformed = true;
            return;
        }
        self.chars.push(key);
    }

    fn apply_vni_shape(chars: &mut Vec<char>, digit: char) -> (bool, bool) {
        let Some(li) = last_letter_index(chars) else {
            return (false, false);
        };
        let ch = strip_tone(chars[li]);
        let signature = lower_char(ch);

        let replace = |chars: &mut Vec<char>, desired: char, plain: char| -> (bool, bool) {
            if signature == desired {
                chars[li] = if ch.is_uppercase() {
                    upper_char(plain)
                } else {
                    plain
                };
                chars.push(digit);
                return (true, true);
            }
            if signature == plain {
                chars[li] = if ch.is_uppercase() {
                    upper_char(desired)
                } else {
                    desired
                };
                return (true, false);
            }
            (false, false)
        };

        match digit {
            '6' => match lower_base(ch) {
                'a' => replace(chars, 'â', 'a'),
                'e' => replace(chars, 'ê', 'e'),
                'o' => replace(chars, 'ô', 'o'),
                _ => (false, false),
            },
            '8' if lower_base(ch) == 'a' => replace(chars, 'ă', 'a'),
            '9' if lower_base(ch) == 'd' => {
                if matches!(ch, 'đ' | 'Đ') {
                    chars[li] = if ch.is_uppercase() { 'D' } else { 'd' };
                    chars.push(digit);
                    (true, true)
                } else {
                    chars[li] = if ch.is_uppercase() { 'Đ' } else { 'đ' };
                    (true, false)
                }
            }
            '7' => {
                let pi = (0..li).rev().find(|i| chars[*i].is_alphabetic());
                if let Some(pi) = pi {
                    if lower_base(chars[pi]) == 'u' && lower_base(ch) == 'o' {
                        let previous = strip_tone(chars[pi]);
                        if lower_char(previous) == 'ư' && signature == 'ơ' {
                            chars[pi] = if previous.is_uppercase() { 'U' } else { 'u' };
                            chars[li] = if ch.is_uppercase() { 'O' } else { 'o' };
                            chars.push(digit);
                            return (true, true);
                        }
                        if lower_char(previous) == 'u' && signature == 'o' {
                            chars[pi] = if previous.is_uppercase() { 'Ư' } else { 'ư' };
                            chars[li] = if ch.is_uppercase() { 'Ơ' } else { 'ơ' };
                            return (true, false);
                        }
                    }
                }
                if lower_base(ch) == 'o' {
                    replace(chars, 'ơ', 'o')
                } else if lower_base(ch) == 'u' {
                    replace(chars, 'ư', 'u')
                } else {
                    (false, false)
                }
            }
            _ => (false, false),
        }
    }

    fn transition_vni(&mut self, key: char) {
        let tone = match key {
            '1' => Some(Tone::Acute),
            '2' => Some(Tone::Grave),
            '3' => Some(Tone::Hook),
            '4' => Some(Tone::Tilde),
            '5' => Some(Tone::Dot),
            _ => None,
        };
        if tone.is_some() || key == '0' {
            if has_vowel(&self.chars) {
                if key == '0' {
                    if self.tone != Tone::None {
                        self.cancel_tone();
                        return;
                    }
                } else if self.tone == tone.expect("tone digit") {
                    self.repeat_tone_literal(key);
                    return;
                } else {
                    self.set_tone(tone.expect("tone digit"), key);
                    return;
                }
            }
            self.chars.push(key);
            return;
        }

        if "6789".contains(key) {
            let (changed, escape) = Self::apply_vni_shape(&mut self.chars, key);
            if changed {
                if escape {
                    self.lock_literal_escape();
                } else {
                    self.transformed = true;
                }
                return;
            }
            if key == '9' && apply_late_d_stroke(&mut self.chars, self.tone) {
                self.transformed = true;
                return;
            }
        }
        self.chars.push(key);
    }

    fn apply_key(&mut self, key: char) -> String {
        self.history.push(self.capture());

        if self.phase == Phase::PendingValidation {
            self.mode = Mode::RawLocked;
            self.phase = Phase::Dead;
            self.rendered = self.fallback.clone();
            self.chars = self.fallback.chars().collect();
            self.clear_tone();
            self.transformed = false;
        }

        self.raw.push(key);

        if self.ambiguous {
            let previous_last = self.ambig_raw.chars().last();
            let literal_repeat = self.ambig_tail_count == 0
                && previous_last.is_some_and(|c| lower_char(c) == lower_char(key));
            self.ambig_tail_count += 1;
            self.ambig_raw.push(key);
            self.ambig_cancel.push(key);
            if literal_repeat {
                self.fallback = self.ambig_cancel.clone();
                self.chars = self.fallback.chars().collect();
                self.finish_explicit_cancel();
                return self.rendered.clone();
            }
            self.rendered = self.choose_ambiguous_literal(false);
            self.fallback = self.rendered.clone();
            self.mode = Mode::RawLocked;
            self.phase = Phase::Dead;
            return self.rendered.clone();
        }

        self.fallback.push(key);

        if self.repeats_bracket_shape(key) {
            self.lock_literal_escape();
            return self.rendered.clone();
        }

        if self.mode == Mode::RawLocked {
            if self.recover_known_raw_after_cancel() {
                return self.rendered.clone();
            }
            self.rendered = self.fallback.clone();
            self.phase = Phase::Dead;
            return self.rendered.clone();
        }

        if self.repeats_shape(key) {
            self.lock_literal_escape();
            return self.rendered.clone();
        }

        if self.prefer_raw_tone_before_shape(key) {
            self.mode = Mode::RawLocked;
            self.phase = Phase::Dead;
            self.rendered = self.fallback.clone();
            return self.rendered.clone();
        }

        if self.options.method.eq_ignore_ascii_case("vni") {
            self.transition_vni(key);
        } else {
            self.transition_telex(key);
        }

        if self.mode == Mode::RawLocked {
            self.rendered = self.fallback.clone();
            self.phase = Phase::Dead;
            return self.rendered.clone();
        }

        self.render_candidate()
    }

    fn apply_backspace(&mut self) -> String {
        let Some(snapshot) = self.history.pop() else {
            self.clear();
            return String::new();
        };
        self.restore(snapshot);
        self.rendered.clone()
    }

    fn apply_escape(&mut self) -> String {
        self.correction_blocked = true;
        self.mode = Mode::RawLocked;
        self.phase = Phase::Dead;
        if self.ambiguous && !self.ambig_raw.is_empty() {
            self.rendered = self.ambig_raw.clone();
            self.fallback = self.rendered.clone();
            self.ambiguous = false;
            return self.rendered.clone();
        }
        self.rendered = if !self.fallback.is_empty() {
            self.fallback.clone()
        } else {
            self.raw.clone()
        };
        self.rendered.clone()
    }

    fn apply_finalize(&mut self) -> String {
        if self.raw.is_empty() {
            return String::new();
        }
        if self.ambiguous {
            self.rendered = self.choose_ambiguous_literal(true);
            self.fallback = self.rendered.clone();
            return self.rendered.clone();
        }
        if self.mode == Mode::RawLocked {
            return self.rendered.clone();
        }
        if self.options.auto_restore
            && self.transformed
            && has_vowel(&self.chars)
            && !is_valid_syllable(&self.rendered)
        {
            self.mode = Mode::RawLocked;
            self.phase = Phase::Dead;
            self.rendered = self.fallback.clone();
        }
        self.rendered.clone()
    }

    fn apply_correct_boundary(&mut self) -> String {
        if self.raw.is_empty() {
            return String::new();
        }

        // Explicit literal/cancel states have already made their decision. A
        // boundary corrector must never reinterpret them as another language
        // candidate.
        if self.ambiguous || self.correction_blocked {
            return self.apply_finalize();
        }

        if self.options.smart_correction && self.options.method.eq_ignore_ascii_case("telex") {
            if let Some(corrected) = resolve_telex_intent(&self.raw) {
                if corrected != self.rendered {
                    return self.adopt_smart_candidate(corrected);
                }
            }
        }

        self.apply_finalize()
    }

    fn apply_commit_boundary(&mut self) -> String {
        let committed = self.apply_finalize();
        self.history.clear();
        self.clear();
        committed
    }

    fn apply_commit_raw_boundary(&mut self) -> String {
        let committed = self.raw.clone();
        self.history.clear();
        self.clear();
        committed
    }

    pub fn raw_text(&self) -> &str {
        &self.raw
    }

    pub fn rendered_text(&self) -> &str {
        &self.rendered
    }

    pub fn state(&self) -> VietnameseState {
        VietnameseState {
            mode: self.mode,
            phase: self.phase,
            raw: self.raw.clone(),
            fallback: self.fallback.clone(),
            rendered: self.rendered.clone(),
            tone: self.tone,
            ambiguous: self.ambiguous,
            transformed: self.transformed,
        }
    }

    fn apply_reset(&mut self) {
        self.history.clear();
        self.clear();
    }

    fn transition(&mut self, event: FsmEvent) -> String {
        match event {
            FsmEvent::Key(key) => self.apply_key(key),
            FsmEvent::Backspace => self.apply_backspace(),
            FsmEvent::Escape => self.apply_escape(),
            FsmEvent::CommitBoundary => self.apply_commit_boundary(),
            FsmEvent::CommitRawBoundary => self.apply_commit_raw_boundary(),
            FsmEvent::Reset => {
                self.apply_reset();
                String::new()
            }
        }
    }

    pub fn type_key(&mut self, key: char) -> String {
        self.transition(FsmEvent::Key(key))
    }

    pub fn backspace(&mut self) -> String {
        self.transition(FsmEvent::Backspace)
    }

    pub fn escape(&mut self) -> String {
        self.transition(FsmEvent::Escape)
    }

    pub fn finalize(&mut self) -> String {
        self.apply_finalize()
    }

    pub fn correct_boundary(&mut self) -> String {
        self.apply_correct_boundary()
    }

    pub fn commit_boundary(&mut self) -> String {
        self.transition(FsmEvent::CommitBoundary)
    }

    pub fn commit_raw_boundary(&mut self) -> String {
        self.transition(FsmEvent::CommitRawBoundary)
    }

    pub fn reset(&mut self) {
        let _ = self.transition(FsmEvent::Reset);
    }

    pub fn has_history(&self) -> bool {
        !self.history.is_empty()
    }
}

pub fn process(raw: &str, options: Options, lexicon: Option<Box<dyn LexiconPort>>) -> String {
    let mut machine = Machine::new(options, lexicon);
    for key in raw.chars() {
        machine.type_key(key);
    }
    machine.rendered_text().to_owned()
}

impl LanguageMachinePort for Machine {
    fn accepts_key(&self, key: char) -> bool {
        if self.options.method.eq_ignore_ascii_case("vni") {
            key.is_ascii_alphanumeric()
        } else {
            key.is_ascii_alphabetic() || matches!(key, '[' | ']')
        }
    }

    fn type_key(&mut self, key: char) -> String {
        Machine::type_key(self, key)
    }

    fn backspace(&mut self) -> String {
        Machine::backspace(self)
    }

    fn escape(&mut self) -> String {
        Machine::escape(self)
    }

    fn finalize(&mut self) -> String {
        self.apply_finalize()
    }

    fn correct_boundary(&mut self) -> String {
        Machine::correct_boundary(self)
    }

    fn reset(&mut self) {
        Machine::reset(self);
    }

    fn state(&self) -> LanguageState {
        let state = Machine::state(self);
        LanguageState {
            raw: state.raw,
            rendered: state.rendered,
            mode: match state.mode {
                Mode::Start => "start",
                Mode::ViCandidate => "candidate",
                Mode::RawLocked => "raw",
            }
            .to_owned(),
            phase: match state.phase {
                Phase::Start => "start",
                Phase::Onset => "onset",
                Phase::Nucleus => "nucleus",
                Phase::Coda => "coda",
                Phase::Complete => "complete",
                Phase::PendingShape => "pending_shape",
                Phase::PendingValidation => "pending_validation",
                Phase::Dead => "dead",
            }
            .to_owned(),
        }
    }

    fn rendered(&self) -> String {
        Machine::rendered_text(self).to_owned()
    }

    fn raw(&self) -> String {
        Machine::raw_text(self).to_owned()
    }

    fn history_active(&self) -> bool {
        !Machine::raw_text(self).is_empty()
    }
}
