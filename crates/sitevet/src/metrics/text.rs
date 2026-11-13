//! Text statistics, readability and keyword tables, from the TS package's
//! `src/metrics/text.ts`. All of it reads [`crate::text::analyze`], so a word counted here is the
//! word `content/keyword-stuffing` counts.

use std::collections::HashSet;
use std::sync::Arc;

use scraper::Html;

use super::{EMPTY, cell, cell_n, num, txt, up_to, value};
use crate::report::{MetricCell, ValueStatus};
use crate::rules::kit::{all, attr, clip, collapse, first, text};
use crate::text::{Phrase, TextAnalysis, analyze, readability};
use crate::{Collector, MetricInput};

const TOP_KEYWORDS: usize = 10;
const NO_STOP_LIST: &str = "No stop word list for this language";
const NOT_AVAILABLE: &str = "Not available for this language";
const NO_SPACES: &str = "Not available for this language: it has no spaces between words";

pub(super) fn collectors() -> Vec<Arc<dyn Collector>> {
    vec![Arc::new(TextStatistics), Arc::new(Readability), Arc::new(Keywords)]
}

fn percent_of(part: usize, whole: usize) -> f64 {
    if whole == 0 { 0.0 } else { (part as f64 * 1000.0 / whole as f64).round() / 10.0 }
}

fn unit(id: &str, label: &str, v: Option<MetricCell>, unit: &str) -> MetricInput {
    MetricInput::value(id, label, v, Some(unit))
}

fn phrase_rows(phrases: &[Phrase]) -> Vec<Vec<MetricCell>> {
    phrases.iter().map(|p| vec![cell(&p.phrase), cell_n(p.count as f64), cell_n(p.percent)]).collect()
}

fn analysis(cx: &crate::RuleCx<'_>, dom: &Html) -> TextAnalysis {
    analyze(dom, &cx.page.language)
}

dom_collector!(TextStatistics, "content", "content/text-statistics", |cx, dom, html| {
    let a = analysis(cx, dom);
    let lang = &cx.page.language;
    let detected = match (&lang.detected, lang.confidence) {
        (Some(_), Some(c)) => format!("Confidence {c}, read from the main content"),
        _ => "Too little text to tell".to_string(),
    };
    let mut out = vec![
        value("content/detected-language", "Detected language", lang.detected.clone().and_then(txt))
            .with_status(if lang.detected.is_some() { ValueStatus::Ok } else { ValueStatus::Missing })
            .with_note(detected),
    ];
    let source = format!("Read from <{}>, nav, footer and hidden elements excluded", a.source);
    if !a.spaced {
        // Counting words needs a dictionary for these scripts; characters are what can be counted.
        out.extend([
            value("content/main-words", "Main content words", None).with_note(NO_SPACES),
            value("content/chars-no-spaces", "Characters", num(a.chars as f64)).with_note(source),
        ]);
        return out;
    }
    let total = a.words.len();
    let avg_sentence = (a.sentences > 0).then(|| (total as f64 * 10.0 / a.sentences as f64).round() / 10.0);
    let stop_pct = a.stop_words.map(|s| percent_of(s, total));
    let water_pct = a.water_words.map(|w| percent_of(w, total));
    let mut water = unit("content/water-percent", "Water", water_pct.and_then(num), "%")
        .with_note("Stop words and filler, the share of words carrying no meaning");
    if let Some(w) = water_pct {
        water = water.with_status(up_to(w, 60.0, 75.0));
    }
    let mut stop_words =
        value("content/stop-words", "Stop words", a.stop_words.and_then(|s| num(s as f64))).with_note(NO_STOP_LIST);
    if let Some(p) = stop_pct {
        stop_words = stop_words.with_note(format!("{p}% of words"));
    }
    out.extend([
        value("content/main-words", "Main content words", num(total as f64)).with_note(source),
        value("content/unique-words", "Unique words", num(a.unique_words as f64))
            .with_note(format!("{}% of words", percent_of(a.unique_words, total))),
        unit("content/unique-words-percent", "Unique words share", num(percent_of(a.unique_words, total)), "%"),
        stop_words,
        unit("content/stop-words-percent", "Stop words share", stop_pct.and_then(num), "%"),
        water,
        unit("content/avg-sentence-length", "Average sentence length", avg_sentence.and_then(num), "words")
            .with_status(avg_sentence.map_or(ValueStatus::Missing, |s| up_to(s, 20.0, 25.0)))
            .with_note("Up to 20 reads comfortably"),
        value("content/chars-no-spaces", "Characters", num(a.chars as f64)).with_note("Main content, spaces excluded"),
    ]);
    out
});

dom_collector!(Readability, "content", "content/readability", |cx, dom, html| {
    let a = analysis(cx, dom);
    let r = readability(&a);
    let not_available = if a.spaced { NOT_AVAILABLE } else { NO_SPACES };
    let avg_word =
        (a.spaced && !a.words.is_empty()).then(|| (a.word_chars as f64 * 10.0 / a.words.len() as f64).round() / 10.0);
    let formula = |id: &str, label: &str, v: Option<f64>, status: Option<ValueStatus>, note: &str| {
        let m = value(id, label, v.and_then(num)).with_note(if r.is_some() { note } else { not_available });
        match status {
            Some(s) => m.with_status(s),
            None => m,
        }
    };
    vec![
        formula(
            "content/flesch-reading-ease",
            "Flesch reading ease",
            r.as_ref().map(|r| r.ease),
            r.as_ref().map(|r| {
                if r.ease >= 60.0 {
                    ValueStatus::Ok
                } else if r.ease >= 30.0 {
                    ValueStatus::Warn
                } else {
                    ValueStatus::Bad
                }
            }),
            r.as_ref().map_or("", |r| r.label),
        ),
        formula(
            "content/flesch-kincaid-grade",
            "Flesch-Kincaid grade",
            r.as_ref().map(|r| r.grade),
            r.as_ref().map(|r| up_to(r.grade, 9.0, 12.0)),
            "US school grade, 9 or lower suits a general audience",
        ),
        formula(
            "content/gunning-fog",
            "Gunning Fog index",
            r.as_ref().map(|r| r.fog),
            r.as_ref().map(|r| up_to(r.fog, 12.0, 16.0)),
            "Years of schooling to read it first time, 12 or lower",
        ),
        unit("content/avg-word-length", "Average word length", avg_word.and_then(num), "characters")
            .with_note(if a.spaced { "Letters per word in the main content" } else { NO_SPACES }),
    ]
});

/// Title, description and every heading, as (element, text) with whitespace collapsed.
fn heading_texts(dom: &Html) -> Vec<(String, String)> {
    let mut out = vec![
        ("Title".to_string(), first(dom, "title").map(text).unwrap_or_default()),
        ("Description".to_string(), attr(dom, r#"meta[name="description"]"#, "content").unwrap_or_default()),
    ];
    out.extend(all(dom, "h1,h2,h3,h4,h5,h6").into_iter().map(|h| (h.value().name().to_uppercase(), text(h))));
    out.into_iter().map(|(e, t)| (e, collapse(&t))).filter(|(_, t)| !t.is_empty()).collect()
}

dom_collector!(Keywords, "content", "content/keywords", |cx, dom, html| {
    let a = analysis(cx, dom);
    let mut out = Vec::new();
    for (i, grams) in a.ngrams.iter().enumerate() {
        if !grams.is_empty() {
            let n = i + 1;
            out.push(MetricInput::table(
                format!("content/top-words-{n}"),
                format!("Top {n}-word phrases"),
                &["Phrase", "Count", "%"],
                phrase_rows(grams),
            ));
        }
    }
    if !a.stop_table.is_empty() {
        out.push(MetricInput::table(
            "content/stop-word-table",
            "Stop words",
            &["Word", "Count", "%"],
            phrase_rows(&a.stop_table),
        ));
    }
    let keywords: Vec<&Phrase> = a.ngrams[0].iter().take(TOP_KEYWORDS).collect();
    if keywords.is_empty() {
        return out;
    }
    let keys_of = |css: &str, attribute: Option<&str>| {
        let t = match attribute {
            Some(name) => attr(dom, css, name).unwrap_or_default(),
            None => first(dom, css).map(text).unwrap_or_default(),
        };
        a.lang.keys_in(&t)
    };
    let (title, desc, h1) =
        (keys_of("title", None), keys_of(r#"meta[name="description"]"#, Some("content")), keys_of("h1", None));
    let yes_no = |set: &HashSet<String>, k: &str| cell(if set.contains(k) { "yes" } else { "no" });
    out.push(
        MetricInput::table(
            "content/keyword-relevance",
            "Keyword relevance",
            &["Keyword", "Count", "Title", "Description", "H1"],
            keywords
                .iter()
                .map(|k| {
                    vec![
                        cell(&k.phrase),
                        cell_n(k.count as f64),
                        yes_no(&title, &k.key),
                        yes_no(&desc, &k.key),
                        yes_no(&h1, &k.key),
                    ]
                })
                .collect(),
        )
        .with_note("Top keywords and whether each appears in the title, description and H1"),
    );
    let keys: HashSet<String> = keywords.iter().map(|k| k.key.clone()).collect();
    let rows = heading_texts(dom)
        .into_iter()
        .map(|(element, t)| {
            let spans = a.lang.spans(&t, &keys);
            let mut matched: Vec<String> = Vec::new();
            for (s, e) in &spans {
                let w = t[*s..*e].to_lowercase();
                if !matched.contains(&w) {
                    matched.push(w);
                }
            }
            let spans = spans.iter().map(|(s, e)| format!("{s}-{e}")).collect::<Vec<_>>().join(" ");
            vec![
                cell(element),
                cell(clip(&t, 70)),
                cell(if matched.is_empty() { EMPTY.to_string() } else { matched.join(", ") }),
                cell(spans),
            ]
        })
        .collect();
    out.push(
        MetricInput::table_capped(
            "content/keyword-highlights",
            "Keywords in headings",
            &["Element", "Text", "Keywords", "Spans"],
            rows,
            40,
        )
        .with_note("Spans are start-end byte offsets into the full text, end exclusive"),
    );
    out
});

#[cfg(test)]
mod tests {
    use crate::report::ValueStatus::Ok;
    use crate::rules::testkit::{fixture, n, page, row, rows, t, value};

    fn on(name: &str) -> crate::rules::testkit::Page {
        page(&fixture(name))
    }

    fn v(p: &crate::rules::testkit::Page, id: &str) -> Option<crate::MetricCell> {
        value(&p.metric(id)).0
    }

    fn note(p: &crate::rules::testkit::Page, id: &str) -> String {
        value(&p.metric(id)).2.unwrap_or_default()
    }

    // Hand-counted. Words: garden soil guide / good soil feeds the garden / compost makes soil
    // rich and dark / water the garden soil every morning: 20 words, 4 sentences. Stop words:
    // the, the, and, every. 14 distinct stems. 97 non-space characters, 94 of them in words.
    // Syllables (the TS estimate) 28, one complex word ("every"):
    // ease = 206.835 - 1.015 * 5 - 84.6 * 1.4 = 83.3, grade = 0.39 * 5 + 11.8 * 1.4 - 15.59 = 2.9,
    // fog = 0.4 * (5 + 100 / 20) = 4.
    #[test]
    fn english_statistics_and_readability() {
        let p = on("text-en.html");
        assert_eq!(v(&p, "content/detected-language"), t("en"));
        assert_eq!(v(&p, "content/main-words"), n(20.0));
        assert_eq!(v(&p, "content/unique-words"), n(14.0));
        assert_eq!(v(&p, "content/unique-words-percent"), n(70.0));
        assert_eq!(v(&p, "content/stop-words"), n(4.0));
        assert_eq!(v(&p, "content/stop-words-percent"), n(20.0));
        assert_eq!(v(&p, "content/water-percent"), n(20.0));
        assert_eq!(v(&p, "content/avg-sentence-length"), n(5.0));
        assert_eq!(v(&p, "content/chars-no-spaces"), n(97.0));
        assert_eq!(v(&p, "content/avg-word-length"), n(4.7));
        let ease = value(&p.metric("content/flesch-reading-ease"));
        assert_eq!((ease.0, ease.1, ease.2.as_deref()), (n(83.3), Some(Ok), Some("Easy")));
        assert_eq!(v(&p, "content/flesch-kincaid-grade"), n(2.9));
        // "just" is a stop word and a filler word: water counts it once.
        let just = page(r#"<html lang="en"><p>It is just fine. Really just fine.</p></html>"#);
        assert_eq!(v(&just, "content/stop-words"), n(4.0));
        assert_eq!(v(&just, "content/water-percent"), n(71.4));
        assert_eq!(v(&p, "content/gunning-fog"), n(4.0));
    }

    #[test]
    fn english_keyword_tables() {
        let p = on("text-en.html");
        assert_eq!(rows(&p.metric("content/top-words-1")), [row(&["soil", "4", "20"]), row(&["garden", "3", "15"])]);
        // The repeated 2-word phrase comes first. Its share counts both words: 2 * 2 / 20.
        assert_eq!(rows(&p.metric("content/top-words-2"))[0], row(&["garden soil", "2", "20"]));
        assert_eq!(
            rows(&p.metric("content/keyword-relevance")),
            [row(&["soil", "4", "yes", "yes", "yes"]), row(&["garden", "3", "yes", "no", "yes"])]
        );
        let highlights = rows(&p.metric("content/keyword-highlights"));
        assert_eq!(highlights[0], row(&["Title", "Garden soil tips", "garden, soil", "0-6 7-11"]));
        assert_eq!(highlights[1], row(&["Description", "How to build rich soil.", "soil", "18-22"]));
        assert_eq!(highlights[2], row(&["H1", "Garden soil guide", "garden, soil", "0-6 7-11"]));
        // Nav and footer text stays out of the tables.
        assert!(!rows(&p.metric("content/top-words-1")).iter().any(|r| r[0] == t("deals").unwrap()));
    }

    // No lang attribute: detection decides. 17 words in 4 sentences. In the ISO Arabic list:
    // في, من, كل, يوم. 14 distinct words.
    #[test]
    fn arabic_is_detected_and_never_gets_an_english_score() {
        let p = on("text-ar.html");
        assert_eq!(v(&p, "content/detected-language"), t("ar"));
        assert_eq!(v(&p, "content/main-words"), n(17.0));
        assert_eq!(v(&p, "content/stop-words"), n(4.0));
        assert_eq!(v(&p, "content/stop-words-percent"), n(23.5));
        assert_eq!(v(&p, "content/unique-words"), n(14.0));
        assert_eq!(v(&p, "content/avg-sentence-length"), n(4.3));
        for id in ["content/flesch-reading-ease", "content/flesch-kincaid-grade", "content/gunning-fog"] {
            assert_eq!(v(&p, id), None, "{id}");
            assert_eq!(note(&p, id), "Not available for this language");
        }
        assert!(v(&p, "content/avg-word-length").is_some());
        assert_eq!(rows(&p.metric("content/top-words-1"))[0][0], t("الحديقة").unwrap());
    }

    // 13 words in 3 sentences. In the ISO French list: le (3), des, les.
    #[test]
    fn french_gets_counts_and_the_neutral_fallback() {
        let p = on("text-fr.html");
        assert_eq!(v(&p, "content/main-words"), n(13.0));
        assert_eq!(v(&p, "content/stop-words"), n(5.0));
        assert_eq!(v(&p, "content/stop-words-percent"), n(38.5));
        assert_eq!(v(&p, "content/avg-sentence-length"), n(4.3));
        assert_eq!(v(&p, "content/flesch-reading-ease"), None);
        assert_eq!(rows(&p.metric("content/top-words-1"))[0], row(&["jardin", "3", "23.1"]));
    }

    // 花园指南 (4) + 好的土壤滋养花园。(9) + 每天早上给花园浇水。(10) = 23 characters.
    #[test]
    fn chinese_counts_characters_only() {
        let p = on("text-zh.html");
        assert_eq!(v(&p, "content/chars-no-spaces"), n(23.0));
        assert_eq!(v(&p, "content/main-words"), None);
        assert!(note(&p, "content/main-words").contains("no spaces between words"));
        for id in ["content/flesch-reading-ease", "content/avg-word-length"] {
            assert_eq!(v(&p, id), None, "{id}");
            assert!(note(&p, id).starts_with("Not available for this language"), "{id}");
        }
        assert!(p.metrics().iter().all(|m| !m.id.starts_with("content/top-words")));
        assert_eq!(p.message("content/keyword-stuffing"), "Word-based check not available for this language");
    }

    #[test]
    fn footer_repeats_stay_out_of_keywords() {
        let p = on("boilerplate.html");
        let top = rows(&p.metric("content/top-words-1"));
        assert_eq!(top[0], row(&["bread", "3", "18.8"]));
        assert!(!top.iter().any(|r| r[0] == t("pizza").unwrap()));
    }
}
