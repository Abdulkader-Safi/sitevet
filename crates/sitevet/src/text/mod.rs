//! Text analysis (FR-TEXT-01 to FR-TEXT-08), ported from the TS package's `src/text.ts`. One
//! pass over the main content gives the statistics, readability, the keyword tables and
//! `content/keyword-stuffing` the same tokens, so they never disagree about what a word is.
//!
//! The TS package read every page as English. Here the language comes from `<html lang>`, else
//! from whatlang. Stop words come from the stop-words ISO lists and keys from the Snowball
//! stemmers, for every language those crates cover. English keeps the TS stop word list: the ISO
//! one counts words such as "home", "page" and "website" as stop words, which empties the keyword
//! tables of real keywords.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use rust_stemmers::{Algorithm, Stemmer};
use scraper::{ElementRef, Html, Node, Selector};
use unicode_segmentation::UnicodeSegmentation;
use whatlang::Script;

use crate::report::LanguageInfo;
use crate::rules::kit::sel;

// The TS package's English stop word list, then the NLTK words it lacks (mostly contraction
// pieces), then function words from the ISO list: pronouns, prepositions and connectors. ISO
// content words ("home", "page", "best", "work") stay out, so they can still be keywords.
const EN_STOP: &str = "a about above after again against all also am an and any are aren't as at be because been \
before being below between both but by can can't cannot could couldn't did didn't do does doesn't doing don't down \
during each even ever few for from further get gets got had hadn't has hasn't have haven't having he he'd he'll he's \
her here here's hers herself him himself his how how's however i i'd i'll i'm i've if in into is isn't it it's its \
itself just let's like may me might more most much must mustn't my myself no nor not now of off on once one only or \
other ought our ours ourselves out over own per same shall shan't she she'd she'll she's should shouldn't so some \
such than that that's the their theirs them themselves then there there's these they they'd they'll they're \
they've this those through thus to too under until up upon us very via was wasn't we we'd we'll we're we've were \
weren't what what's when when's where where's whether which while who who's whom why why's will with within without \
won't would wouldn't yet you you'd you'll you're you've your yours yourself yourselves \
ain aren couldn d didn doesn don hadn hasn haven isn it'd it'll ll m ma mightn mightn't mustn needn needn't o re s \
shan should've shouldn t that'll ve wasn weren won wouldn y \
across afterwards ago ahead almost alone along alongside already although always amid amidst among amongst another \
anybody anyhow anymore anyone anything anyway anyways anywhere apart around aside away became become becomes \
becoming beforehand behind beside besides beyond despite either else elsewhere enough etc every everybody everyone \
everything everywhere except hence hereafter hereby herein hereupon indeed inside instead lest likewise maybe \
meanwhile moreover namely neither nevertheless nobody none nonetheless nothing notwithstanding nowhere often onto \
otherwise perhaps quite rather since somebody somehow someone something sometime sometimes somewhat somewhere soon \
still thence thereafter thereby therefore therein thereof thereupon though throughout thru till together toward \
towards unless unlike unto versus whatever whence whenever whereas whereby wherein whereupon wherever whichever \
whilst whoever whomever whose";

/// Words that pad a sentence without adding meaning. With stop words they make up "water".
const EN_FILLER: &str = "actually absolutely basically certainly clearly completely definitely essentially \
extremely generally highly honestly indeed just literally mostly obviously particularly quite rather really \
simply somewhat totally truly usually various virtually";

static FILLER_EN: LazyLock<HashSet<&'static str>> = LazyLock::new(|| EN_FILLER.split(' ').collect());

/// whatlang's ISO 639-3 codes to the 639-1 codes `lang` attributes and stop-words use.
const ISO3_TO_1: [(&str, &str); 70] = [
    ("afr", "af"),
    ("aka", "ak"),
    ("amh", "am"),
    ("ara", "ar"),
    ("aze", "az"),
    ("bel", "be"),
    ("ben", "bn"),
    ("bul", "bg"),
    ("cat", "ca"),
    ("ces", "cs"),
    ("cmn", "zh"),
    ("cym", "cy"),
    ("dan", "da"),
    ("deu", "de"),
    ("ell", "el"),
    ("eng", "en"),
    ("epo", "eo"),
    ("est", "et"),
    ("fin", "fi"),
    ("fra", "fr"),
    ("guj", "gu"),
    ("heb", "he"),
    ("hin", "hi"),
    ("hrv", "hr"),
    ("hun", "hu"),
    ("hye", "hy"),
    ("ind", "id"),
    ("ita", "it"),
    ("jav", "jv"),
    ("jpn", "ja"),
    ("kan", "kn"),
    ("kat", "ka"),
    ("khm", "km"),
    ("kor", "ko"),
    ("lat", "la"),
    ("lav", "lv"),
    ("lit", "lt"),
    ("mal", "ml"),
    ("mar", "mr"),
    ("mkd", "mk"),
    ("mya", "my"),
    ("nep", "ne"),
    ("nld", "nl"),
    ("nob", "no"),
    ("ori", "or"),
    ("pan", "pa"),
    ("pes", "fa"),
    ("pol", "pl"),
    ("por", "pt"),
    ("ron", "ro"),
    ("rus", "ru"),
    ("sin", "si"),
    ("slk", "sk"),
    ("slv", "sl"),
    ("sna", "sn"),
    ("spa", "es"),
    ("srp", "sr"),
    ("swe", "sv"),
    ("tam", "ta"),
    ("tel", "te"),
    ("tgl", "tl"),
    ("tha", "th"),
    ("tuk", "tk"),
    ("tur", "tr"),
    ("ukr", "uk"),
    ("urd", "ur"),
    ("uzb", "uz"),
    ("vie", "vi"),
    ("yid", "yi"),
    ("zul", "zu"),
];

/// Languages written without spaces between words. unicode-segmentation cannot split them.
const NO_SPACE_LANGS: [&str; 6] = ["zh", "ja", "th", "km", "my", "lo"];

/// Not prose (script, style), or repeated on every page (nav, aside, forms).
const NOISE: &str =
    "script,style,noscript,template,svg,canvas,iframe,nav,aside,form,select,button,[hidden],[aria-hidden=true]";
/// Removed only when reading the whole body. Inside `<main>` a header belongs to the content.
const CHROME: &str = "body > header, body > footer, [role=banner], [role=contentinfo], [role=navigation]";
/// Elements that end a run of text, so `<li>One</li><li>Two</li>` reads as two sentences.
const BLOCKS: &str = "address,article,blockquote,br,dd,div,dl,dt,figcaption,figure,h1,h2,h3,h4,h5,h6,header,footer,hr,li,main,ol,p,pre,section,table,td,th,tr,ul";

static HIDDEN_STYLE: LazyLock<regex::Regex> =
    LazyLock::new(|| crate::rules::kit::re(r"(?i)display\s*:\s*none|visibility\s*:\s*hidden"));
static SYLLABLE_END: LazyLock<regex::Regex> =
    LazyLock::new(|| crate::rules::kit::re(r"(?:[^laeiouy]es|ed|[^laeiouy]e)$"));
static VOWEL_RUN: LazyLock<regex::Regex> = LazyLock::new(|| crate::rules::kit::re("[aeiouy]{1,2}"));

pub(crate) struct Phrase {
    /// The most written surface form of the group.
    pub(crate) phrase: String,
    /// The stem key the group was merged on.
    pub(crate) key: String,
    pub(crate) count: usize,
    /// Share of all words, 0 to 100, one decimal.
    pub(crate) percent: f64,
}

/// The language a page is read in: its stop words and stemmer.
pub(crate) struct Lang {
    /// Base language code, None when the page declares none and detection found none.
    pub(crate) code: Option<String>,
    stop: Option<HashSet<&'static str>>,
    stemmer: Option<Stemmer>,
}

impl Lang {
    pub(crate) fn new(code: Option<String>) -> Self {
        let stop = match code.as_deref() {
            Some("en") => Some(EN_STOP.split(' ').collect()),
            Some(c) => stop_words::lookup(c).map(|l| l.iter().copied().collect()),
            None => None,
        };
        let stemmer = code.as_deref().and_then(algorithm).map(Stemmer::create);
        Self { code, stop, stemmer }
    }

    pub(crate) fn is_english(&self) -> bool {
        self.code.as_deref() == Some("en")
    }

    pub(crate) fn has_stop_words(&self) -> bool {
        self.stop.is_some()
    }

    fn is_stop(&self, w: &str) -> bool {
        self.stop.as_ref().is_some_and(|s| s.contains(w))
    }

    /// The grouping key: the Snowball stem, or the word itself without a stemmer.
    pub(crate) fn key(&self, word: &str) -> String {
        match &self.stemmer {
            Some(s) => s.stem(word).into_owned(),
            None => word.to_string(),
        }
    }

    /// The keys present in a short string such as a title.
    pub(crate) fn keys_in(&self, text: &str) -> HashSet<String> {
        tokens(text).map(|(_, w)| self.key(&w)).collect()
    }

    /// Byte ranges in `text` where a word with one of `keys` occurs. Ends are exclusive.
    pub(crate) fn spans(&self, text: &str, keys: &HashSet<String>) -> Vec<(usize, usize)> {
        tokens(text).filter(|(_, w)| keys.contains(&self.key(w))).map(|(i, w)| (i, i + w.len())).collect()
    }
}

fn algorithm(code: &str) -> Option<Algorithm> {
    use Algorithm::*;
    Some(match code {
        "ar" => Arabic,
        "da" => Danish,
        "nl" => Dutch,
        "en" => English,
        "fi" => Finnish,
        "fr" => French,
        "de" => German,
        "el" => Greek,
        "hu" => Hungarian,
        "it" => Italian,
        "no" | "nb" | "nn" => Norwegian,
        "pt" => Portuguese,
        "ro" => Romanian,
        "ru" => Russian,
        "es" => Spanish,
        "sv" => Swedish,
        "ta" => Tamil,
        "tr" => Turkish,
        _ => return None,
    })
}

pub(crate) struct TextAnalysis {
    pub(crate) lang: Lang,
    /// Where the text came from: "main", "article" or "body".
    pub(crate) source: &'static str,
    /// False for scripts without spaces between words. Word metrics are not available then.
    pub(crate) spaced: bool,
    pub(crate) words: Vec<String>,
    pub(crate) sentences: usize,
    /// Characters of main content, whitespace excluded.
    pub(crate) chars: usize,
    /// Characters inside words, for the average word length.
    pub(crate) word_chars: usize,
    /// Distinct stem keys.
    pub(crate) unique_words: usize,
    /// None without a stop word list for the language.
    pub(crate) stop_words: Option<usize>,
    /// Stop words plus English filler. None without a stop word list.
    pub(crate) water_words: Option<usize>,
    /// English only.
    pub(crate) syllables: Option<usize>,
    /// Words of three or more syllables. English only.
    pub(crate) complex_words: Option<usize>,
    /// Per-word stop word counts, most common first.
    pub(crate) stop_table: Vec<Phrase>,
    /// 1, 2 and 3-word phrases used more than once, most common first, stop words trimmed.
    pub(crate) ngrams: [Vec<Phrase>; 3],
}

/// The page language for [`crate::PageData`]: the declared `lang` and a detection from the main
/// content.
pub(crate) fn language(dom: &Html, declared: Option<&str>) -> LanguageInfo {
    let info = whatlang::detect(&main_text(dom).0);
    LanguageInfo {
        declared: declared.map(str::trim).filter(|l| !l.is_empty()).map(str::to_string),
        detected: info.as_ref().map(|i| iso1(i.lang().code())),
        confidence: info.map(|i| (i.confidence() * 100.0).round() / 100.0),
    }
}

fn iso1(code3: &str) -> String {
    ISO3_TO_1.iter().find(|(three, _)| *three == code3).map_or(code3, |(_, one)| one).to_string()
}

/// The element holding the main content, and where it came from: a single `<main>`,
/// `[role=main]` or `<article>`, else `<body>`.
pub(crate) fn main_root(dom: &Html) -> (Option<ElementRef<'_>>, &'static str) {
    let pick = ["main", "[role=main]", "article"].into_iter().find(|s| dom.select(&sel(s)).count() == 1);
    let source = match pick {
        Some("article") => "article",
        Some(_) => "main",
        None => "body",
    };
    (dom.select(&sel(pick.unwrap_or("body"))).next(), source)
}

fn skip_for(source: &str) -> Skip {
    Skip { noise: sel(NOISE), chrome: (source == "body").then(|| sel(CHROME)), blocks: sel(BLOCKS) }
}

impl Skip {
    fn skips(&self, el: &ElementRef) -> bool {
        let hidden = el.value().attr("style").is_some_and(|s| HIDDEN_STYLE.is_match(s));
        hidden || self.noise.matches(el) || self.chrome.as_ref().is_some_and(|c| c.matches(el))
    }
}

/// Elements inside the main content that are not boilerplate (nav, aside, header and footer
/// chrome, hidden), matching `css`, in document order. The same boundary the text statistics use.
pub(crate) fn in_main<'a>(dom: &'a Html, css: &str) -> Vec<ElementRef<'a>> {
    let (Some(root), source) = main_root(dom) else { return Vec::new() };
    let skip = skip_for(source);
    let want = sel(css);
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(el) = stack.pop() {
        if el != root && skip.skips(&el) {
            continue;
        }
        if want.matches(&el) {
            out.push(el);
        }
        let mut kids: Vec<ElementRef> = el.children().filter_map(ElementRef::wrap).collect();
        kids.reverse();
        stack.extend(kids);
    }
    out
}

/// The main content as text, with a newline after every block element, and where it came from.
pub(crate) fn main_text(dom: &Html) -> (String, &'static str) {
    let (root, source) = main_root(dom);
    let Some(root) = root else { return (String::new(), source) };
    let mut out = String::new();
    walk(root, &skip_for(source), &mut out);
    (out, source)
}

/// One H2 section of the main content: its heading, its text and its first block.
pub(crate) struct Section<'a> {
    pub(crate) heading: ElementRef<'a>,
    pub(crate) text: String,
    /// The first paragraph-level element after the heading, however deeply either is nested.
    pub(crate) first_block: Option<ElementRef<'a>>,
}

/// The main content split at each H2, in document order, so a heading wrapped in its own `<div>`
/// still owns the text that follows it. An H1 ends a section without starting one.
pub(crate) fn h2_sections(dom: &Html) -> Vec<Section<'_>> {
    let (Some(root), source) = main_root(dom) else { return Vec::new() };
    let skip = skip_for(source);
    let blocks = sel("p,ul,ol,dl,table,blockquote,pre");
    let mut out: Vec<Section> = Vec::new();
    let mut open = false;
    let mut stack = vec![root];
    while let Some(el) = stack.pop() {
        if el != root && skip.skips(&el) {
            continue;
        }
        match el.value().name() {
            "h2" => {
                out.push(Section { heading: el, text: String::new(), first_block: None });
                open = true;
                continue;
            }
            "h1" => {
                open = false;
                continue;
            }
            _ => {}
        }
        if open && let Some(s) = out.last_mut() {
            if s.first_block.is_none() && blocks.matches(&el) {
                s.first_block = Some(el);
            }
            for t in el.children().filter_map(|c| c.value().as_text()) {
                s.text.push_str(t);
                s.text.push(' ');
            }
        }
        let mut kids: Vec<ElementRef> = el.children().filter_map(ElementRef::wrap).collect();
        kids.reverse();
        stack.extend(kids);
    }
    out
}

struct Skip {
    noise: Selector,
    chrome: Option<Selector>,
    blocks: Selector,
}

fn walk(el: ElementRef, skip: &Skip, out: &mut String) {
    for child in el.children() {
        match child.value() {
            Node::Text(t) => out.push_str(t),
            Node::Element(_) => {
                let Some(child_el) = ElementRef::wrap(child) else { continue };
                if skip.skips(&child_el) {
                    continue;
                }
                walk(child_el, skip, out);
                if skip.blocks.matches(&child_el) {
                    out.push('\n');
                }
            }
            _ => {}
        }
    }
}

/// Lowercased word tokens with their byte offset. Numbers and punctuation are dropped.
fn tokens(text: &str) -> impl Iterator<Item = (usize, String)> + '_ {
    text.unicode_word_indices()
        .filter(|(_, w)| w.chars().any(char::is_alphabetic))
        // The curly apostrophe is 3 bytes and "'" is 1, so the offset stays on the original.
        .map(|(i, w)| (i, w.to_lowercase()))
}

/// Lowercased word tokens. Numbers and punctuation are dropped.
pub(crate) fn tokenize(text: &str) -> Vec<String> {
    tokens(text).map(|(_, w)| w.replace('\u{2019}', "'")).collect()
}

/// Fewer main-content words than this give no SimHash: too little text to compare.
pub(crate) const SIMHASH_MIN_WORDS: usize = 50;

/// 256-bit SimHash over word 5-shingles (FR-SITE-02): pages that share most of their phrasing
/// get hashes a few bits apart. Each shingle's FNV-1a hash is stretched into 4 independent
/// 64-bit words with splitmix64, so the value is stable across Rust versions. 64 bits alone
/// proved too noisy: one changed paragraph in twelve moved it 15 bits, half the way to unrelated.
pub(crate) fn simhash(words: &[String]) -> Option<[u64; 4]> {
    if words.len() < SIMHASH_MIN_WORDS {
        return None;
    }
    let mut votes = [0i32; 256];
    for shingle in words.windows(5) {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for w in shingle {
            for b in w.bytes().chain(*b" ") {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
        for k in 0..4u64 {
            let mut z = h ^ k.wrapping_mul(0x9e37_79b9_7f4a_7c15);
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^= z >> 31;
            for bit in 0..64 {
                votes[k as usize * 64 + bit] += if z >> bit & 1 == 1 { 1 } else { -1 };
            }
        }
    }
    let mut out = [0u64; 4];
    for (i, v) in votes.iter().enumerate() {
        if *v > 0 {
            out[i / 64] |= 1 << (i % 64);
        }
    }
    Some(out)
}

/// English syllable estimate, good to about one syllable per long word.
fn syllables(word: &str) -> usize {
    let w: String = word.to_lowercase().chars().filter(char::is_ascii_lowercase).collect();
    if w.len() <= 3 {
        return 1;
    }
    let w = SYLLABLE_END.replace(&w, "");
    let w = w.strip_prefix('y').unwrap_or(&w);
    VOWEL_RUN.find_iter(w).count().max(1)
}

fn round1(n: f64) -> f64 {
    (n * 10.0).round() / 10.0
}

fn spaced(text: &str, code: Option<&str>) -> bool {
    match whatlang::detect_script(text) {
        Some(s) => !matches!(
            s,
            Script::Mandarin | Script::Hiragana | Script::Katakana | Script::Thai | Script::Khmer | Script::Myanmar
        ),
        None => !code.is_some_and(|c| NO_SPACE_LANGS.contains(&c)),
    }
}

// ponytail: each text collector and the stuffing rule analyse the page again (4 passes, about a
// millisecond each). Cache on the page context if profiling ever shows it.
pub(crate) fn analyze(dom: &Html, language: &LanguageInfo) -> TextAnalysis {
    let lang = Lang::new(language.code());
    let (text, source) = main_text(dom);
    let chars = text.chars().filter(|c| !c.is_whitespace()).count();
    let spaced = spaced(&text, lang.code.as_deref());

    let sentences: Vec<Vec<String>> = if spaced {
        text.split('\n')
            .flat_map(|block| block.split_sentence_bounds().map(tokenize).collect::<Vec<_>>())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    };
    let words: Vec<String> = sentences.concat();
    let total = words.len();

    let ngrams = [1usize, 2, 3].map(|n| {
        let mut items = Vec::new();
        for s in &sentences {
            for gram in s.windows(n) {
                // A phrase that opens or closes on a stop word ("of the") is not a keyword.
                // Stop words inside one ("cost of living") are fine.
                if lang.is_stop(&gram[0])
                    || lang.is_stop(&gram[n - 1])
                    || (n == 1 && gram[0].encode_utf16().count() < 2)
                {
                    continue;
                }
                let key = gram.iter().map(|w| lang.key(w)).collect::<Vec<_>>().join(" ");
                items.push((key, gram.join(" ")));
            }
        }
        // A phrase written once is not a pattern.
        group(items, total, n).into_iter().filter(|p| p.count > 1).collect()
    });

    let english = lang.is_english();
    let stop_hits: Vec<&String> = words.iter().filter(|w| lang.is_stop(w)).collect();
    let stop_words = lang.has_stop_words().then_some(stop_hits.len());
    // A word on both lists ("just", "quite") counts once. The TS package counted it twice.
    let filler =
        if english { words.iter().filter(|w| FILLER_EN.contains(w.as_str()) && !lang.is_stop(w)).count() } else { 0 };
    TextAnalysis {
        source,
        spaced,
        sentences: sentences.len(),
        chars,
        word_chars: words.iter().map(|w| w.chars().count()).sum(),
        unique_words: words.iter().map(|w| lang.key(w)).collect::<HashSet<_>>().len(),
        stop_words,
        water_words: stop_words.map(|s| s + filler),
        syllables: english.then(|| words.iter().map(|w| syllables(w)).sum()),
        complex_words: english.then(|| words.iter().filter(|w| syllables(w) >= 3).count()),
        stop_table: group(stop_hits.iter().map(|w| (w.to_string(), w.to_string())).collect(), total, 1),
        ngrams,
        words,
        lang,
    }
}

/// Counts by key, keeping the most written surface form (the shorter one on a tie).
fn group(items: Vec<(String, String)>, total: usize, weight: usize) -> Vec<Phrase> {
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, (usize, Vec<(String, usize)>)> = HashMap::new();
    for (key, surface) in items {
        let g = groups.entry(key.clone()).or_insert_with(|| {
            order.push(key);
            (0, Vec::new())
        });
        g.0 += 1;
        match g.1.iter_mut().find(|(s, _)| *s == surface) {
            Some(f) => f.1 += 1,
            None => g.1.push((surface, 1)),
        }
    }
    let mut out: Vec<Phrase> = order
        .into_iter()
        .filter_map(|key| groups.remove(&key).map(|g| (key, g)))
        .map(|(key, (count, mut forms))| {
            forms.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.encode_utf16().count().cmp(&b.0.encode_utf16().count())));
            let percent = if total > 0 { round1((count * weight * 100) as f64 / total as f64) } else { 0.0 };
            Phrase { phrase: forms.swap_remove(0).0, key, count, percent }
        })
        .collect();
    // ponytail: plain lowercase order stands in for JS localeCompare on ties.
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.phrase.to_lowercase().cmp(&b.phrase.to_lowercase())));
    out
}

pub(crate) struct Readability {
    pub(crate) ease: f64,
    pub(crate) label: &'static str,
    pub(crate) grade: f64,
    pub(crate) fog: f64,
}

/// Flesch reading ease, Flesch-Kincaid grade and Gunning Fog. English text only (the formulas
/// are tuned to English syllables), None otherwise or with no sentences.
pub(crate) fn readability(a: &TextAnalysis) -> Option<Readability> {
    let (syl, complex) = (a.syllables?, a.complex_words?);
    if a.words.is_empty() || a.sentences == 0 {
        return None;
    }
    let words = a.words.len() as f64;
    let wps = words / a.sentences as f64;
    let spw = syl as f64 / words;
    let ease = round1(206.835 - 1.015 * wps - 84.6 * spw);
    let label = if ease >= 60.0 {
        "Easy"
    } else if ease >= 30.0 {
        "Medium"
    } else {
        "Difficult"
    };
    Some(Readability {
        ease,
        label,
        grade: round1(0.39 * wps + 11.8 * spw - 15.59),
        fog: round1(0.4 * (wps + 100.0 * complex as f64 / words)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze_html(html: &str) -> TextAnalysis {
        let dom = Html::parse_document(html);
        let lang = dom.root_element().value().attr("lang");
        analyze(&dom, &language(&dom, lang))
    }

    #[test]
    fn main_text_drops_chrome_and_hidden_text() {
        let dom = Html::parse_document(&crate::rules::testkit::fixture("article.html"));
        let (text, source) = main_text(&dom);
        assert_eq!(source, "main");
        assert!(text.contains("Composting turns kitchen scraps"));
        assert!(!text.contains("deals"), "nav, hidden and footer text is gone: {text}");
        assert!(text.contains("Fruit peels\n"));
    }

    #[test]
    fn stems_group_word_forms_per_language() {
        let en = Lang::new(Some("en".into()));
        assert_eq!(en.key("developments"), en.key("development"));
        assert_eq!(en.key("cities"), en.key("city"));
        assert_ne!(en.key("class"), en.key("bus"));
        let none = Lang::new(Some("xx".into()));
        assert_eq!(none.key("cities"), "cities");
        assert!(!none.has_stop_words());
        assert!(Lang::new(Some("ar".into())).has_stop_words());
    }

    #[test]
    fn every_whatlang_language_has_a_two_letter_code() {
        for l in whatlang::Lang::all() {
            assert_eq!(iso1(l.code()).len(), 2, "{l:?}");
        }
    }

    #[test]
    fn ngrams_count_repeated_phrases() {
        let a = analyze_html(&crate::rules::testkit::fixture("article.html"));
        assert_eq!(a.ngrams[0][0].phrase, "compost");
        assert!(a.ngrams[1].iter().any(|p| p.phrase == "compost bin" && p.count == 3));
        assert_eq!(tokenize("Don\u{2019}t stop, 42 times!"), ["don't", "stop", "times"]);
    }

    #[test]
    fn spans_are_byte_ranges() {
        let en = Lang::new(Some("en".into()));
        let keys = en.keys_in("compost");
        let text = "Café compost: Composting bins";
        let spans = en.spans(text, &keys);
        assert_eq!(spans, [(6, 13), (15, 25)]);
        assert_eq!(&text[6..13], "compost");
        assert_eq!(&text[15..25], "Composting");
    }

    #[test]
    fn syllable_estimates() {
        // Reference values from the TS package's `syllables`.
        for (w, n) in [("the", 1), ("reading", 2), ("table", 2), ("beautiful", 4), ("yesterday", 3), ("readability", 5)]
        {
            assert_eq!(syllables(w), n, "{w}");
        }
    }
}
