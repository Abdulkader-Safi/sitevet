//! Answer engine and content structure (FR-AEO-01 to FR-AEO-05, FR-AEO-07): how easily a search
//! snippet or an AI answer can lift content from the page. Measurements only: none of it moves
//! the score. The TS package lists these in its roadmap without code, so the thresholds are
//! sitevet's own, each named in a constant below.

use std::collections::HashSet;
use std::sync::{Arc, LazyLock};

use scraper::{ElementRef, Html};

use super::{cell, cell_n, num, txt, value};
use crate::report::ValueStatus;
use crate::rules::kit::{all, collapse, re, sel, text};
use crate::text::{h2_sections, in_main, main_text, tokenize};
use crate::{Collector, MetricInput, RuleCx, SchemaNode};

/// Blocks read for an answer near the top (FR-AEO-01).
const ANSWER_BLOCKS: usize = 4;
/// A snippet-sized answer, in words.
const ANSWER_WORDS: std::ops::RangeInclusive<usize> = 15..=80;
/// The most words a short answer under a question heading may have.
const SHORT_ANSWER: usize = 60;
/// Average words per H2 section that reads well as one passage.
const SECTION_BAND: std::ops::RangeInclusive<f64> = 100.0..=300.0;
/// Fewer main-content words than this in the raw HTML, on a page with scripts, suggests the
/// content needs JavaScript.
const MIN_RAW_WORDS: usize = 50;
/// The AI crawlers FR-AEO-05 names, by robots.txt user agent.
const AI_CRAWLERS: [&str; 5] = ["GPTBot", "ClaudeBot", "PerplexityBot", "CCBot", "Google-Extended"];
/// Paragraph-level blocks of the main content.
const BLOCKS: &str = "p,ul,ol,dl,table,blockquote,pre";

static DEFINITION: LazyLock<regex::Regex> =
    LazyLock::new(|| re(r"(?i)^[^.?!]{1,80}?\b(is|are|was|were|means|refers to|describes|stands for)\b"));
static QUESTION_WORD: LazyLock<regex::Regex> =
    LazyLock::new(|| re(r"(?i)^(what|why|how|when|where|who|which|can|could|does|do|did|is|are|should|will|would)\b"));
static RELATED: LazyLock<regex::Regex> = LazyLock::new(|| {
    re(r"(?i)\b(related|further reading|read next|see also|you may also like|more articles|keep reading)\b")
});

pub(super) fn collectors() -> Vec<Arc<dyn Collector>> {
    vec![Arc::new(Answers), Arc::new(TopicCluster), Arc::new(AgentAccess), Arc::new(PageType)]
}

fn words(s: &str) -> usize {
    tokenize(s).len()
}

fn round2(n: f64) -> f64 {
    (n * 100.0).round() / 100.0
}

fn ratio_status(r: f64) -> ValueStatus {
    if r >= 0.75 {
        ValueStatus::Ok
    } else if r >= 0.5 {
        ValueStatus::Warn
    } else {
        ValueStatus::Bad
    }
}

/// Elements matching `css` with no ancestor that also matches it.
fn outermost<'a>(els: Vec<ElementRef<'a>>, css: &str) -> Vec<ElementRef<'a>> {
    let s = sel(css);
    els.into_iter().filter(|e| !e.ancestors().filter_map(ElementRef::wrap).any(|a| s.matches(&a))).collect()
}

fn is_english(cx: &RuleCx<'_>) -> bool {
    cx.page.language.code().as_deref() == Some("en")
}

fn is_question(heading: &str, english: bool) -> bool {
    heading.ends_with(['?', '\u{61f}', '\u{ff1f}']) || (english && QUESTION_WORD.is_match(heading))
}

/// "Term: definition" paragraphs: a bold or `<dfn>` term first, then a colon or dash.
fn term_paragraph(p: ElementRef) -> bool {
    let Some(first) = p.children().find(|c| c.value().as_text().is_none_or(|t| !t.trim().is_empty())) else {
        return false;
    };
    let Some(term) = ElementRef::wrap(first).filter(|e| matches!(e.value().name(), "strong" | "b" | "dfn")) else {
        return false;
    };
    let (all_text, term_text) = (collapse(&text(p)), collapse(&text(term)));
    all_text
        .strip_prefix(&term_text)
        .is_some_and(|rest| !term_text.is_empty() && rest.trim_start().starts_with([':', '-', '\u{2013}']))
}

dom_collector!(Answers, "answer-engine", "answer-engine/answers", |cx, dom, html| {
    let english = is_english(cx);
    let blocks: Vec<ElementRef> =
        outermost(in_main(dom, BLOCKS), BLOCKS).into_iter().filter(|b| words(&text(*b)) > 0).collect();

    // FR-AEO-01: a definition (English) or a snippet-sized paragraph near the top.
    let snippet = blocks.iter().take(ANSWER_BLOCKS).enumerate().find_map(|(i, b)| {
        let t = collapse(&text(*b));
        let n = words(&t);
        (b.value().name() == "p" && ANSWER_WORDS.contains(&n) && (!english || DEFINITION.is_match(&t)))
            .then_some((i + 1, n))
    });
    let kind = if english { "definition" } else { "short answer" };
    let mut out = vec![
        value("answer-engine/answer-block", "Answer near the top", snippet.and_then(|(b, _)| num(b as f64)))
            .with_status(if snippet.is_some() { ValueStatus::Ok } else { ValueStatus::Warn })
            .with_note(match snippet {
                Some((b, n)) => format!("Block {b} is a {kind} of {n} words"),
                None => format!(
                    "No {kind} of {} to {} words in the first {ANSWER_BLOCKS} blocks",
                    ANSWER_WORDS.start(),
                    ANSWER_WORDS.end()
                ),
            }),
        MetricInput::value(
            "answer-engine/answer-words",
            "Answer length",
            snippet.and_then(|(_, n)| num(n as f64)),
            Some("words"),
        ),
    ];

    // FR-AEO-02: question headings with a short answer straight under them.
    let sections = h2_sections(dom);
    let questions: Vec<&crate::text::Section> =
        sections.iter().filter(|s| is_question(&collapse(&text(s.heading)), english)).collect();
    let answered = questions
        .iter()
        .filter(|s| {
            s.first_block.is_some_and(|b| b.value().name() == "p" && (1..=SHORT_ANSWER).contains(&words(&text(b))))
        })
        .count();
    let coverage = (!questions.is_empty()).then(|| round2(answered as f64 / questions.len() as f64));
    out.extend([
        value("answer-engine/question-headings", "Question headings", num(questions.len() as f64))
            .with_note("H2s phrased as a question"),
        value("answer-engine/answered-questions", "Answered questions", num(answered as f64))
            .with_note(format!("Followed directly by a paragraph of up to {SHORT_ANSWER} words")),
        value("answer-engine/question-coverage", "Question coverage", coverage.and_then(num))
            .with_status(coverage.map_or(ValueStatus::Missing, ratio_status))
            .with_note("Answered questions over question headings"),
    ]);

    // FR-AEO-03: answer and passage formats.
    let lists = outermost(in_main(dom, "ul,ol"), "ul,ol");
    let li = sel("li");
    let steps = lists
        .iter()
        .filter(|l| {
            l.value().name() == "ol" && l.children().filter_map(ElementRef::wrap).filter(|c| li.matches(c)).count() >= 2
        })
        .count();
    let definitions = in_main(dom, "dl").len() + in_main(dom, "p").into_iter().filter(|p| term_paragraph(*p)).count();
    let faq = outermost(in_main(dom, "[id*=faq i],[class*=faq i]"), "[id*=faq i],[class*=faq i]").len();
    let formats: [(&str, usize); 5] = [
        ("Lists", lists.len()),
        ("Step lists", steps),
        ("Tables", in_main(dom, "table").len()),
        ("Definition blocks", definitions),
        ("FAQ blocks", faq),
    ];
    out.push(
        MetricInput::table(
            "answer-engine/formats",
            "Answer formats",
            &["Format", "Count"],
            formats.iter().map(|(f, n)| vec![cell(*f), cell_n(*n as f64)]).collect(),
        )
        .with_note("Step lists are ordered lists of 2 or more items. Definition blocks are <dl> lists and \"Term: text\" paragraphs"),
    );

    let sections: Vec<usize> = sections.iter().map(|s| words(&s.text)).collect();
    let avg = (!sections.is_empty())
        .then(|| (sections.iter().sum::<usize>() as f64 * 10.0 / sections.len() as f64).round() / 10.0);
    let (min, max) = (sections.iter().min(), sections.iter().max());
    out.extend([
        value("answer-engine/h2-sections", "H2 sections", num(sections.len() as f64)),
        MetricInput::value("answer-engine/section-words", "Words per H2 section", avg.and_then(num), Some("words"))
            .with_status(avg.map_or(ValueStatus::Missing, |a| {
                if SECTION_BAND.contains(&a) { ValueStatus::Ok } else { ValueStatus::Warn }
            }))
            .with_note(match (min, max) {
                (Some(lo), Some(hi)) => format!(
                    "Average. Shortest {lo}, longest {hi}. {} to {} words reads well as one passage",
                    SECTION_BAND.start(),
                    SECTION_BAND.end()
                ),
                _ => "No H2 sections".to_string(),
            }),
    ]);
    out
});

/// The links of a related-reading block: a heading such as "Related articles" (links up to the
/// next heading of the same or a higher level), or an element whose class or id says "related".
fn related_links(dom: &Html) -> Option<usize> {
    let a = sel("a[href]");
    for h in all(dom, "h2,h3,h4") {
        if RELATED.is_match(&collapse(&text(h))) {
            let level = h.value().name().as_bytes()[1];
            let stop = |e: &ElementRef| {
                e.value().name().len() == 2
                    && e.value().name().starts_with('h')
                    && e.value().name().as_bytes()[1] <= level
            };
            let n = h
                .next_siblings()
                .filter_map(ElementRef::wrap)
                .take_while(|e| !stop(e))
                .map(|e| e.select(&a).count() + usize::from(a.matches(&e)))
                .sum();
            return Some(n);
        }
    }
    let css = "[id*=related i],[class*=related i]";
    outermost(all(dom, css), css).first().map(|e| e.select(&a).count())
}

dom_collector!(TopicCluster, "answer-engine", "answer-engine/topic-cluster", |cx, dom, html| {
    let main: Vec<&crate::Link> = cx.page.links.iter().filter(|l| l.in_main).collect();
    let internal: Vec<&str> = main
        .iter()
        .filter(|l| l.internal)
        .filter_map(|l| l.url.as_deref())
        .map(|u| u.split('#').next().unwrap_or(u))
        .collect();
    let unique: HashSet<&str> = internal.iter().copied().collect();
    let generic = re(crate::rules::GENERIC_ANCHOR);
    let descriptive = main.iter().filter(|l| !l.text.trim().is_empty() && !generic.is_match(l.text.trim())).count();
    let ratio = (!main.is_empty()).then(|| round2(descriptive as f64 / main.len() as f64));
    let related = related_links(dom);
    let h2: HashSet<String> = in_main(dom, "h2").into_iter().map(|h| collapse(&text(h)).to_lowercase()).collect();
    vec![
        value("answer-engine/main-internal-links", "Internal links in main content", num(internal.len() as f64))
            .with_note(format!("{} unique. Nav, aside, header and footer links excluded", unique.len())),
        value("answer-engine/main-internal-unique", "Unique internal links in main content", num(unique.len() as f64)),
        value("answer-engine/related-reading", "Related reading section", related.and_then(|n| num(n as f64)))
            .with_status(if related.is_some() { ValueStatus::Ok } else { ValueStatus::Missing })
            .with_note(match related {
                Some(n) => format!("{n} links"),
                None => "No related-reading block found".to_string(),
            }),
        value("answer-engine/h2-topics", "Distinct H2 topics", num(h2.len() as f64)),
        value("answer-engine/h3-count", "H3 headings", num(in_main(dom, "h3").len() as f64)),
        value("answer-engine/descriptive-anchors", "Descriptive anchors", ratio.and_then(num))
            .with_status(ratio.map_or(ValueStatus::Missing, |r| {
                if r >= 0.8 {
                    ValueStatus::Ok
                } else if r >= 0.5 {
                    ValueStatus::Warn
                } else {
                    ValueStatus::Bad
                }
            }))
            .with_note(format!("{descriptive} of {} main-content links say where they go", main.len())),
    ]
});

dom_collector!(AgentAccess, "answer-engine", "answer-engine/agent-access", |cx, dom, html| {
    let robots = &cx.site.robots;
    let access: Vec<(&str, bool)> = AI_CRAWLERS
        .iter()
        .map(|agent| {
            let allowed = !robots.exists
                || texting_robots::Robot::new(agent, robots.content.as_bytes())
                    .map_or(true, |r| r.allowed(&cx.page.final_url));
            (*agent, allowed)
        })
        .collect();
    let blocked: Vec<&str> = access.iter().filter(|(_, ok)| !ok).map(|(a, _)| *a).collect();
    let llms = &cx.site.llms;
    let raw_words = words(&main_text(dom).0);
    let mut out = vec![
        MetricInput::table(
            "answer-engine/ai-crawlers",
            "AI crawler access",
            &["Crawler", "This page"],
            access.iter().map(|(a, ok)| vec![cell(*a), cell(if *ok { "allowed" } else { "blocked" })]).collect(),
        )
        .with_note("From robots.txt"),
        value("answer-engine/ai-crawlers-blocked", "AI crawlers blocked", num(blocked.len() as f64))
            .with_status(if blocked.is_empty() { ValueStatus::Ok } else { ValueStatus::Warn })
            .with_note(if blocked.is_empty() {
                "None of the 5".to_string()
            } else {
                format!(
                    "{}. Blocked crawlers can't read or cite this page; fine if that is on purpose",
                    blocked.join(", ")
                )
            }),
        value("answer-engine/llms-txt", "llms.txt", llms.title.clone().and_then(txt))
            .with_status(if llms.valid {
                ValueStatus::Ok
            } else if llms.exists {
                ValueStatus::Warn
            } else {
                ValueStatus::Missing
            })
            .with_note(if llms.valid {
                format!("{} links", llms.links)
            } else if llms.exists {
                "Found, but the first line is not a \"# Title\" heading".to_string()
            } else {
                "Not found".to_string()
            }),
    ];
    let without_js = if cx.page.js_rendered {
        MetricInput::value("answer-engine/content-without-js", "Content without JavaScript", None, Some("words"))
            .with_note("Not measured: --render-crawl reads the page after scripts ran")
    } else {
        let thin = raw_words < MIN_RAW_WORDS && !cx.page.scripts.is_empty();
        MetricInput::value("answer-engine/content-without-js", "Content without JavaScript", num(raw_words as f64), Some("words"))
            .with_status(if thin { ValueStatus::Warn } else { ValueStatus::Ok })
            .with_note(if thin {
                "Little text in the raw HTML on a page that loads scripts. Crawlers that skip JavaScript may see an empty page"
            } else {
                "Main content words in the HTML as served"
            })
    };
    out.push(without_js);
    out
});

/// Page-level schema for pages that are none of the five types: listings, about pages, search.
const OTHER_PAGES: [&str; 5] = ["CollectionPage", "AboutPage", "ItemList", "SearchResultsPage", "ProfilePage"];

/// FR-AEO-07: the page purpose and what gave it away.
fn page_type(cx: &RuleCx<'_>, dom: &Html) -> (Option<&'static str>, String) {
    let nodes: Vec<&SchemaNode> = cx.page.schema.iter().flat_map(|i| i.node.walk()).collect();
    let typed = |names: &[&str]| nodes.iter().flat_map(|n| &n.types).find(|t| names.contains(&t.as_str())).cloned();
    let top = |names: &[&str]| {
        cx.page.schema.iter().flat_map(|i| &i.node.types).find(|t| names.contains(&t.as_str())).cloned()
    };
    // The site root is home, whatever it lists.
    let path = url::Url::parse(&cx.page.final_url).map(|u| u.path().to_lowercase()).unwrap_or_default();
    if path.is_empty() || path == "/" {
        return (Some("home"), "From the URL: the site root".into());
    }
    // A listing or about page says so at the top level, often with article items nested inside,
    // so this runs before the article check.
    if let Some(t) = top(&OTHER_PAGES) {
        return (None, format!("From schema: {t}, which is none of home, article, product, contact or local business"));
    }
    // A page that offers a service is the business's own page.
    if let Some(t) = top(&["Service"]) {
        return (Some("local business"), format!("From schema: {t}"));
    }
    let from_schema = [
        ("product", &["Product"][..]),
        ("article", &["Article", "NewsArticle", "BlogPosting", "TechArticle", "Report", "CreativeWork"][..]),
        ("contact", &["ContactPage"][..]),
    ];
    for (label, names) in from_schema {
        if let Some(t) = typed(names) {
            return (Some(label), format!("From schema: {t}"));
        }
    }
    let h1 = all(dom, "h1").first().map(|h| collapse(&text(*h)).to_lowercase()).unwrap_or_default();
    if path.contains("contact") || h1.contains("contact") {
        return (Some("contact"), "From the URL or H1".into());
    }
    if all(dom, "article").len() == 1 {
        return (Some("article"), "From the page structure: one <article> element".into());
    }
    // ponytail: the loosest signal, kept for pages with no schema that names their purpose.
    let prose = in_main(dom, "p").len() >= 2 && !in_main(dom, "h2").is_empty() && !h1.is_empty();
    if prose {
        return (Some("article"), "From the page structure: an H1, H2 sections and paragraphs".into());
    }
    // Business schema often sits on every page of a site, so it decides only this late.
    match typed(&crate::schema::LOCAL_TYPES) {
        Some(t) => (Some("local business"), format!("From schema: {t}")),
        None => (None, "Could not tell from schema or structure".into()),
    }
}

dom_collector!(PageType, "answer-engine", "answer-engine/page-type", |cx, dom, html| {
    let (hint, why) = page_type(cx, dom);
    vec![
        value("answer-engine/page-type", "Page type", hint.and_then(txt))
            .with_status(if hint.is_some() { ValueStatus::Ok } else { ValueStatus::Missing })
            .with_note(why),
    ]
});

#[cfg(test)]
mod tests {
    use crate::report::ValueStatus::{Missing, Ok, Warn};
    use crate::rules::testkit::{Page, fixture, n, page, row, rows, t, value};

    fn v(p: &Page, id: &str) -> Option<crate::MetricCell> {
        value(&p.metric(id)).0
    }

    // Hand-counted: block 1 has 6 words, block 2 is a 40-word definition. Four question H2s,
    // three followed by a paragraph (the second by a list). Sections: 7, 11, 8, 7 and 20 words.
    #[test]
    fn answers_questions_formats_and_sections() {
        let p = page(&fixture("answers.html"));
        let block = value(&p.metric("answer-engine/answer-block"));
        assert_eq!((block.0, block.1), (n(2.0), Some(Ok)));
        assert_eq!(block.2.as_deref(), Some("Block 2 is a definition of 40 words"));
        assert_eq!(v(&p, "answer-engine/answer-words"), n(40.0));
        assert_eq!(v(&p, "answer-engine/question-headings"), n(4.0));
        assert_eq!(v(&p, "answer-engine/answered-questions"), n(3.0));
        assert_eq!(v(&p, "answer-engine/question-coverage"), n(0.75));
        assert_eq!(
            rows(&p.metric("answer-engine/formats")),
            [
                row(&["Lists", "1"]),
                row(&["Step lists", "1"]),
                row(&["Tables", "1"]),
                row(&["Definition blocks", "2"]),
                row(&["FAQ blocks", "1"])
            ]
        );
        assert_eq!(v(&p, "answer-engine/h2-sections"), n(5.0));
        let words = value(&p.metric("answer-engine/section-words"));
        assert_eq!((words.0, words.1), (n(10.6), Some(Warn)));
        assert!(words.2.unwrap().starts_with("Average. Shortest 7, longest 20."));
    }

    #[test]
    fn no_answer_near_the_top() {
        let p = page(r#"<html lang="en"><main><p>Short.</p><h2>Plain heading</h2><p>Text.</p></main></html>"#);
        assert_eq!(value(&p.metric("answer-engine/answer-block")).1, Some(Warn));
        assert_eq!(value(&p.metric("answer-engine/question-coverage")).1, Some(Missing));
    }

    #[test]
    fn topic_cluster_reads_main_content_only() {
        let p = page(&fixture("topic.html"));
        // /soil, /soil#ph and /water. Nav, aside and footer links are left out.
        assert_eq!(v(&p, "answer-engine/main-internal-links"), n(3.0));
        assert_eq!(v(&p, "answer-engine/main-internal-unique"), n(2.0));
        assert_eq!(v(&p, "answer-engine/related-reading"), n(2.0));
        assert_eq!(v(&p, "answer-engine/h2-topics"), n(2.0));
        assert_eq!(v(&p, "answer-engine/h3-count"), n(1.0));
        let anchors = value(&p.metric("answer-engine/descriptive-anchors"));
        assert_eq!((anchors.0, anchors.1), (n(0.75), Some(Warn)));
    }

    #[test]
    fn ai_crawler_access_from_robots() {
        let mut p = page("<main><p>Hi</p></main>");
        p.site.robots.content = "User-agent: GPTBot\nDisallow: /\n\nUser-agent: *\nAllow: /\n".into();
        assert_eq!(
            rows(&p.metric("answer-engine/ai-crawlers")),
            [
                row(&["GPTBot", "blocked"]),
                row(&["ClaudeBot", "allowed"]),
                row(&["PerplexityBot", "allowed"]),
                row(&["CCBot", "allowed"]),
                row(&["Google-Extended", "allowed"])
            ]
        );
        assert_eq!(v(&p, "answer-engine/ai-crawlers-blocked"), n(1.0));
        assert_eq!(value(&p.metric("answer-engine/llms-txt")).1, Some(Missing));
        p.site.llms = crate::LlmsInfo::parse(200, "# Shop\n\n- [Guide](https://example.com/g)\n");
        let llms = value(&p.metric("answer-engine/llms-txt"));
        assert_eq!((llms.0, llms.1, llms.2.as_deref()), (t("Shop"), Some(Ok), Some("1 links")));
    }

    #[test]
    fn content_that_needs_javascript() {
        let empty = page(r#"<body><div id="root"></div><script src="/app.js"></script></body>"#);
        let m = value(&empty.metric("answer-engine/content-without-js"));
        assert_eq!((m.0, m.1), (n(0.0), Some(Warn)));
        let full = page(&fixture("answers.html"));
        assert_eq!(value(&full.metric("answer-engine/content-without-js")).1, Some(Ok));
    }

    #[test]
    fn page_type_hints() {
        let article = page(&fixture("article.html")).url("https://example.com/guides/compost");
        assert_eq!(v(&article, "answer-engine/page-type"), t("article"));
        let blog = r#"<script type="application/ld+json">{"@type":"BlogPosting","headline":"Post"}</script>"#;
        let home = page(blog).url("https://example.com/");
        assert_eq!(v(&home, "answer-engine/page-type"), t("home"));
        let product = page(r#"<script type="application/ld+json">{"@type":"Product","name":"Spade"}</script>"#);
        let m = value(&product.metric("answer-engine/page-type"));
        assert_eq!((m.0, m.2.as_deref()), (t("product"), Some("From schema: Product")));
        assert_eq!(v(&page("<p>x</p>"), "answer-engine/page-type"), None);
    }

    #[test]
    fn listings_and_case_studies() {
        let ld = |json: &str| format!(r#"<script type="application/ld+json">{json}</script>"#);
        // A blog index nests BlogPosting items, and is still not an article.
        let listing = page(&format!(
            "{}{}",
            ld(r#"{"@type":"CollectionPage","mainEntity":{"@type":"ItemList","itemListElement":[{"@type":"BlogPosting"}]}}"#),
            fixture("article.html")
        ))
        .url("https://example.com/blog");
        let m = value(&listing.metric("answer-engine/page-type"));
        assert_eq!((m.0, m.1), (None, Some(Missing)));
        assert!(m.2.unwrap().starts_with("From schema: CollectionPage, which is none of"));
        let service =
            page(&format!("{}{}", ld(r#"{"@type":"Service","name":"Figma plugins"}"#), fixture("article.html")))
                .url("https://example.com/services/figma");
        assert_eq!(v(&service, "answer-engine/page-type"), t("local business"));
        let case_study = page(&ld(r#"{"@type":"CreativeWork","name":"Vouch"}"#)).url("https://example.com/work/vouch");
        assert_eq!(v(&case_study, "answer-engine/page-type"), t("article"));
        let single = page("<article><h1>Terms</h1><p>Text.</p></article>").url("https://example.com/terms");
        let m = value(&single.metric("answer-engine/page-type"));
        assert_eq!((m.0, m.2.as_deref()), (t("article"), Some("From the page structure: one <article> element")));
        // Many <article> cards are a listing, not an article.
        let cards = page("<main><h1>Work</h1><article><p>A</p></article><article><p>B</p></article></main>")
            .url("https://example.com/work");
        assert_eq!(v(&cards, "answer-engine/page-type"), None);
    }

    #[test]
    fn wrapped_headings_still_own_their_sections() {
        let p = page(
            r#"<html lang="en"><main><div class="head"><h2>What is it?</h2></div>
            <div class="body"><p>A short answer here.</p><p>More text here now.</p></div></main></html>"#,
        );
        assert_eq!(v(&p, "answer-engine/answered-questions"), n(1.0));
        assert_eq!(v(&p, "answer-engine/section-words"), n(8.0));
    }

    #[test]
    fn site_wide_business_schema_does_not_hide_the_page_purpose() {
        let ld = r#"<script type="application/ld+json">{"@type":"ProfessionalService","name":"Studio"}</script>"#;
        let home = page(ld).url("https://example.com/");
        assert_eq!(v(&home, "answer-engine/page-type"), t("home"));
        let about = page(ld).url("https://example.com/about");
        assert_eq!(v(&about, "answer-engine/page-type"), t("local business"));
        let post = page(&format!("{ld}{}", fixture("article.html"))).url("https://example.com/guides/compost");
        assert_eq!(v(&post, "answer-engine/page-type"), t("article"));
    }
}
