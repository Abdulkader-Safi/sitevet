//! Text analysis, ported from the TS package's `src/text.ts`. M2 carries the part
//! `content/keyword-stuffing` needs: main-content text, word and sentence tokens, n-grams. M6
//! adds the statistics, readability and other languages.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use scraper::{ElementRef, Html, Node, Selector};
use unicode_segmentation::UnicodeSegmentation;

use crate::rules::kit::sel;

// The TS package's English stop word list.
const EN_STOP: &str = "a about above after again against all also am an and any are aren't as at be because been \
before being below between both but by can can't cannot could couldn't did didn't do does doesn't doing don't down \
during each even ever few for from further get gets got had hadn't has hasn't have haven't having he he'd he'll he's \
her here here's hers herself him himself his how how's however i i'd i'll i'm i've if in into is isn't it it's its \
itself just let's like may me might more most much must mustn't my myself no nor not now of off on once one only or \
other ought our ours ourselves out over own per same shall shan't she she'd she'll she's should shouldn't so some \
such than that that's the their theirs them themselves then there there's these they they'd they'll they're \
they've this those through thus to too under until up upon us very via was wasn't we we'd we'll we're we've were \
weren't what what's when when's where where's whether which while who who's whom why why's will with within without \
won't would wouldn't yet you you'd you'll you're you've your yours yourself yourselves";

static STOP_EN: LazyLock<HashSet<&'static str>> = LazyLock::new(|| EN_STOP.split(' ').collect());

/// Not prose (script, style), or repeated on every page (nav, aside, forms).
const NOISE: &str =
    "script,style,noscript,template,svg,canvas,iframe,nav,aside,form,select,button,[hidden],[aria-hidden=true]";
/// Removed only when reading the whole body.
const CHROME: &str = "body > header, body > footer, [role=banner], [role=contentinfo], [role=navigation]";
/// Elements that end a run of text.
const BLOCKS: &str = "address,article,blockquote,br,dd,div,dl,dt,figcaption,figure,h1,h2,h3,h4,h5,h6,header,footer,hr,li,main,ol,p,pre,section,table,td,th,tr,ul";

static HIDDEN_STYLE: LazyLock<regex::Regex> =
    LazyLock::new(|| crate::rules::kit::re(r"(?i)display\s*:\s*none|visibility\s*:\s*hidden"));

pub(crate) struct Phrase {
    pub(crate) phrase: String,
    pub(crate) count: usize,
    /// Share of all words, 0 to 100, one decimal.
    pub(crate) percent: f64,
}

pub(crate) struct TextAnalysis {
    pub(crate) words: Vec<String>,
    /// 1, 2 and 3-word phrases used more than once, most common first, stop words trimmed.
    pub(crate) ngrams: [Vec<Phrase>; 3],
}

/// The page's base language subtag, "en" when undeclared.
fn base_lang(dom: &Html) -> String {
    let lang = dom.root_element().value().attr("lang").unwrap_or("en").trim().to_lowercase();
    lang.split(['-', '_']).next().filter(|l| !l.is_empty()).unwrap_or("en").to_string()
}

/// The main content as text, with a newline after every block element.
pub(crate) fn main_text(dom: &Html) -> String {
    let pick = ["main", "[role=main]", "article"].into_iter().find(|s| dom.select(&sel(s)).count() == 1);
    let root = match pick {
        Some(s) => dom.select(&sel(s)).next(),
        None => dom.select(&sel("body")).next(),
    };
    let Some(root) = root else { return String::new() };
    let skip = Skip { noise: sel(NOISE), chrome: pick.is_none().then(|| sel(CHROME)), blocks: sel(BLOCKS) };
    let mut out = String::new();
    walk(root, &skip, &mut out);
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
            Node::Element(e) => {
                let Some(child_el) = ElementRef::wrap(child) else { continue };
                let hidden = e.attr("style").is_some_and(|s| HIDDEN_STYLE.is_match(s));
                if skip.noise.matches(&child_el) || hidden || skip.chrome.as_ref().is_some_and(|c| c.matches(&child_el))
                {
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

/// Lowercased word tokens. Numbers and punctuation are dropped.
pub(crate) fn tokenize(text: &str) -> Vec<String> {
    text.unicode_words()
        .filter(|w| w.chars().any(char::is_alphabetic))
        .map(|w| w.to_lowercase().replace('\u{2019}', "'"))
        .collect()
}

/// Merges plurals and possessives, so "development" and "developments" are one key.
pub(crate) fn lemma(word: &str, lang: &str) -> String {
    if lang != "en" {
        return word.to_string();
    }
    let w = word.strip_suffix("'s").unwrap_or(word);
    let len = w.encode_utf16().count();
    if len <= 3 {
        return w.to_string();
    }
    if len > 4
        && let Some(stem) = w.strip_suffix("ies")
    {
        return format!("{stem}y");
    }
    if ["ss", "us", "is"].iter().any(|e| w.ends_with(e)) {
        return w.to_string();
    }
    if ["ches", "shes", "sses", "xes", "zes"].iter().any(|e| w.ends_with(e)) {
        return w[..w.len() - 2].to_string();
    }
    w.strip_suffix('s').unwrap_or(w).to_string()
}

fn round1(n: f64) -> f64 {
    (n * 10.0).round() / 10.0
}

pub(crate) fn analyze(dom: &Html) -> TextAnalysis {
    let lang = base_lang(dom);
    let text = main_text(dom);
    let stop = (lang == "en").then_some(&*STOP_EN);
    let is_stop = |w: &str| stop.is_some_and(|s| s.contains(w));

    let sentences: Vec<Vec<String>> = text
        .split('\n')
        .flat_map(|block| block.split_sentence_bounds().map(tokenize).collect::<Vec<_>>())
        .filter(|s| !s.is_empty())
        .collect();
    let words: Vec<String> = sentences.concat();
    let total = words.len();

    let ngrams = [1usize, 2, 3].map(|n| {
        let mut items = Vec::new();
        for s in &sentences {
            for gram in s.windows(n) {
                if is_stop(&gram[0]) || is_stop(&gram[n - 1]) || (n == 1 && gram[0].encode_utf16().count() < 2) {
                    continue;
                }
                let key = gram.iter().map(|w| lemma(w, &lang)).collect::<Vec<_>>().join(" ");
                items.push((key, gram.join(" ")));
            }
        }
        group(items, total, n).into_iter().filter(|p| p.count > 1).collect()
    });
    TextAnalysis { words, ngrams }
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
        .filter_map(|key| groups.remove(&key))
        .map(|(count, mut forms)| {
            forms.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.encode_utf16().count().cmp(&b.0.encode_utf16().count())));
            let percent = if total > 0 { round1((count * weight * 100) as f64 / total as f64) } else { 0.0 };
            Phrase { phrase: forms.swap_remove(0).0, count, percent }
        })
        .collect();
    // ponytail: plain lowercase order stands in for JS localeCompare on ties.
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.phrase.to_lowercase().cmp(&b.phrase.to_lowercase())));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_text_drops_chrome_and_hidden_text() {
        let dom = Html::parse_document(&crate::rules::testkit::fixture("article.html"));
        let text = main_text(&dom);
        assert!(text.contains("Composting turns kitchen scraps"));
        assert!(!text.contains("deals"), "nav, hidden and footer text is gone: {text}");
        assert!(text.contains("Fruit peels\n"));
    }

    #[test]
    fn lemmas_merge_plurals_only() {
        for (w, l) in [
            ("developments", "development"),
            ("cities", "city"),
            ("boxes", "box"),
            ("class", "class"),
            ("bus", "bus"),
            ("john's", "john"),
            ("gas", "gas"),
        ] {
            assert_eq!(lemma(w, "en"), l, "{w}");
        }
        assert_eq!(lemma("cities", "fr"), "cities");
    }

    #[test]
    fn ngrams_count_repeated_phrases() {
        let dom = Html::parse_document(&crate::rules::testkit::fixture("article.html"));
        let a = analyze(&dom);
        let top = &a.ngrams[0][0];
        assert_eq!(top.phrase, "compost");
        assert!(a.ngrams[1].iter().any(|p| p.phrase == "compost bin" && p.count == 3));
        assert_eq!(tokenize("Don\u{2019}t stop, 42 times!"), ["don't", "stop", "times"]);
    }
}
