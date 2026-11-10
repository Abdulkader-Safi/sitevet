//! content rules, ported from the TS package's `src/rules/content.ts`.

use std::sync::Arc;

use super::kit::{all, attr, collapse, first, js_len, text};
use crate::{Rule, RuleResult as R};

const CATEGORY: &str = "content";
const MIN_WORDS_FOR_STUFFING: usize = 100;
const STUFFING_PERCENT: f64 = 5.0;
pub(crate) const HEADINGS: &str = "h1,h2,h3,h4,h5,h6";

pub(super) fn rules() -> Vec<Arc<dyn Rule>> {
    vec![
        Arc::new(HeadingHierarchy),
        Arc::new(WordCount),
        Arc::new(NoEmptyHeadings),
        Arc::new(KeywordStuffing),
        Arc::new(TextHtmlRatio),
        Arc::new(HtmlLang),
    ]
}

/// `$("body").text().replace(/\s+/g, " ").trim()`
fn body_text(dom: &scraper::Html) -> String {
    first(dom, "body").map(|b| collapse(&text(b))).unwrap_or_default()
}

dom_rule!(
    HeadingHierarchy,
    "content/heading-hierarchy",
    "Heading hierarchy",
    Warning,
    "Order headings without skipping levels. Do not jump from H2 straight to H4.",
    |cx, dom, html| {
        let mut prev = 0;
        for h in all(dom, HEADINGS) {
            let level = h.value().name().as_bytes()[1] - b'0';
            if prev != 0 && level > prev + 1 {
                return vec![R::warn(format!("Heading jumps from H{prev} to H{level}"))];
            }
            prev = level;
        }
        vec![R::pass("No skipped heading levels")]
    }
);

dom_rule!(
    WordCount,
    "content/word-count",
    "Word count",
    Info,
    "Add more substantive content. Thin pages rank poorly. Aim for at least 300 meaningful words.",
    |cx, dom, html| {
        let words = body_text(dom).split(' ').filter(|w| !w.is_empty()).count();
        if words < 300 {
            vec![R::warn(format!("Thin content ({words} words)"))]
        } else {
            vec![R::pass(format!("{words} words"))]
        }
    }
);

dom_rule!(
    NoEmptyHeadings,
    "content/no-empty-headings",
    "No empty headings",
    Warning,
    "Remove empty heading tags or give them real text.",
    |cx, dom, html| {
        let empty = all(dom, HEADINGS).into_iter().filter(|h| text(*h).trim().is_empty()).count();
        if empty == 0 { vec![R::pass("No empty headings")] } else { vec![R::warn(format!("{empty} empty heading(s)"))] }
    }
);

dom_rule!(
    KeywordStuffing,
    "content/keyword-stuffing",
    "Keyword stuffing",
    Info,
    "Reduce the repetition of the flagged word and write naturally.",
    |cx, dom, html| {
        let a = crate::text::analyze(dom);
        if a.words.len() < MIN_WORDS_FOR_STUFFING {
            return vec![R::pass("Not enough text to flag stuffing")];
        }
        // The n-gram share weights a phrase by its length, so one ceiling covers words and phrases.
        let mut tops: Vec<&crate::text::Phrase> = a.ngrams.iter().filter_map(|g| g.first()).collect();
        tops.sort_by(|x, y| y.percent.total_cmp(&x.percent));
        match tops.first() {
            Some(top) if top.percent > STUFFING_PERCENT => {
                vec![R::warn(format!("\"{}\" is {}% of words", top.phrase, top.percent))]
            }
            _ => vec![R::pass("No keyword over-repetition")],
        }
    }
);

dom_rule!(
    TextHtmlRatio,
    "content/text-html-ratio",
    "Text to HTML ratio",
    Info,
    "Increase real text relative to markup, or trim bloated HTML.",
    |cx, dom, html| {
        let ratio = js_len(&body_text(dom)) as f64 / js_len(html).max(1) as f64;
        let pct = ratio * 100.0;
        if ratio < 0.1 {
            vec![R::warn(format!("Low text ratio ({pct:.1}%)"))]
        } else {
            vec![R::pass(format!("Text ratio {pct:.1}%"))]
        }
    }
);

dom_rule!(
    HtmlLang,
    "content/html-lang",
    "HTML lang attribute",
    Warning,
    r#"Set the language on the root element, e.g. <html lang="en">."#,
    |cx, dom, html| {
        match attr(dom, "html", "lang").filter(|l| !l.is_empty()) {
            Some(lang) => vec![R::pass(format!("lang=\"{lang}\""))],
            None => vec![R::warn("Missing lang on <html>")],
        }
    }
);

#[cfg(test)]
mod tests {
    use crate::Status::{Pass, Warn};
    use crate::rules::testkit::page;

    #[test]
    fn hierarchy_and_empty_headings() {
        assert_eq!(page("<h1>a</h1><h2>b</h2><h3>c</h3><h2>d</h2>").status("content/heading-hierarchy"), Pass);
        assert_eq!(page("<h1>a</h1><h3>b</h3>").message("content/heading-hierarchy"), "Heading jumps from H1 to H3");
        assert_eq!(page("<h1>a</h1>").status("content/no-empty-headings"), Pass);
        assert_eq!(page("<h1>a</h1><h2> </h2>").message("content/no-empty-headings"), "1 empty heading(s)");
    }

    #[test]
    fn word_count_and_ratio() {
        let long = format!("<p>{}</p>", "word ".repeat(300));
        assert_eq!(page(&long).message("content/word-count"), "300 words");
        assert_eq!(page("<p>few words</p>").message("content/word-count"), "Thin content (2 words)");
        assert_eq!(page(&long).status("content/text-html-ratio"), Pass);
        let heavy = format!("<p>hi</p><div class=\"{}\"></div>", "x".repeat(500));
        assert_eq!(page(&heavy).status("content/text-html-ratio"), Warn);
    }

    #[test]
    fn keyword_stuffing() {
        // 150 distinct words: nothing repeats, so nothing is stuffed.
        let varied: String =
            (0..150u8).map(|i| format!("w{}{} ", char::from(b'a' + i % 26), char::from(b'a' + i / 26))).collect();
        let few = page("<p>compost compost compost</p>");
        assert_eq!(few.message("content/keyword-stuffing"), "Not enough text to flag stuffing");
        let stuffed = page(&format!(
            "<p>{}</p>",
            "compost garden soil leaves water sun air worms bin ".repeat(12) + &"compost ".repeat(30)
        ));
        assert_eq!(stuffed.status("content/keyword-stuffing"), Warn);
        assert_eq!(stuffed.message("content/keyword-stuffing"), "\"compost compost compost\" is 60.9% of words");
        assert_eq!(page(&format!("<p>{varied}</p>")).status("content/keyword-stuffing"), Pass);
    }

    #[test]
    fn html_lang() {
        assert_eq!(page(r#"<html lang="en"><p>x</p></html>"#).message("content/html-lang"), "lang=\"en\"");
        assert_eq!(page("<p>x</p>").status("content/html-lang"), Warn);
    }
}
