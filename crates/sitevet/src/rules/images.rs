//! images rules, ported from the TS package's `src/rules/images.ts`.

use std::sync::Arc;

use super::kit::re;
use crate::{Image, Rule, RuleResult as R};

const CATEGORY: &str = "images";

pub(super) fn rules() -> Vec<Arc<dyn Rule>> {
    vec![
        Arc::new(ImgAlt),
        Arc::new(ImgDimensions),
        Arc::new(ImgModernFormat),
        Arc::new(ImgLazyLoading),
        Arc::new(ImgFilenameQuality),
        Arc::new(ImgEmptySrc),
    ]
}

fn abs(i: &Image) -> &str {
    i.url.as_deref().unwrap_or("")
}

fn set(v: &Option<String>) -> bool {
    v.as_deref().is_some_and(|s| !s.is_empty())
}

dom_rule!(
    ImgAlt,
    "images/img-alt",
    "Image alt text",
    Error,
    r#"Add descriptive alt text to each image. Use alt="" only for purely decorative images."#,
    |cx, dom, html| {
        let missing: Vec<&Image> = cx.page.images.iter().filter(|i| i.alt.is_none()).collect();
        if missing.is_empty() {
            return vec![R::pass(format!("All {} images have alt", cx.page.images.len()))];
        }
        let sample: Vec<&str> = missing.iter().take(3).map(|i| i.src.as_str()).collect();
        vec![R::fail(format!("{} image(s) missing alt", missing.len())).with_evidence(sample.join(", "))]
    }
);

dom_rule!(
    ImgDimensions,
    "images/img-dimensions",
    "Image dimensions",
    Warning,
    "Set width and height attributes on images to prevent layout shift.",
    |cx, dom, html| {
        let missing = cx.page.images.iter().filter(|i| !set(&i.width) || !set(&i.height)).count();
        if missing == 0 {
            vec![R::pass("All images have width and height")]
        } else {
            vec![R::warn(format!("{missing} image(s) missing width/height"))]
        }
    }
);

dom_rule!(
    ImgModernFormat,
    "images/img-modern-format",
    "Modern image formats",
    Info,
    "Serve images as WebP or AVIF for smaller files and faster loads.",
    |cx, dom, html| {
        let legacy_re = re(r"(?i)\.(jpg|jpeg|png|gif)(\?|$)");
        let legacy = cx.page.images.iter().filter(|i| legacy_re.is_match(abs(i))).count();
        if legacy == 0 {
            vec![R::pass("No legacy image formats")]
        } else {
            vec![R::warn(format!("{legacy} image(s) not using WebP/AVIF"))]
        }
    }
);

dom_rule!(
    ImgLazyLoading,
    "images/img-lazy-loading",
    "Lazy loading",
    Info,
    r#"Add loading="lazy" to images below the fold."#,
    |cx, dom, html| {
        let images = &cx.page.images;
        if images.len() <= 1 {
            return vec![R::pass("Too few images to need lazy loading")];
        }
        let eager = images[1..].iter().filter(|i| i.loading.as_deref() != Some("lazy")).count();
        if eager == 0 {
            vec![R::pass("Below-the-fold images lazy loaded")]
        } else {
            vec![R::warn(format!("{eager} image(s) without loading=\"lazy\""))]
        }
    }
);

dom_rule!(
    ImgFilenameQuality,
    "images/img-filename-quality",
    "Image filenames",
    Info,
    "Rename images to descriptive, keyword-relevant filenames instead of IMG_1234.jpg.",
    |cx, dom, html| {
        let bad_re = re(r"(?i)/(img|image|dsc|screenshot)?[-_]?\d{3,}\.\w+$");
        let bad = cx.page.images.iter().filter(|i| bad_re.is_match(abs(i))).count();
        if bad == 0 {
            vec![R::pass("Descriptive image filenames")]
        } else {
            vec![R::warn(format!("{bad} image(s) with non-descriptive filenames"))]
        }
    }
);

dom_rule!(
    ImgEmptySrc,
    "images/img-empty-src",
    "Image src present",
    Warning,
    "Give every <img> a valid src, or remove the tag.",
    |cx, dom, html| {
        let empty = cx.page.images.iter().filter(|i| i.src.trim().is_empty()).count();
        if empty == 0 {
            vec![R::pass("All images have src")]
        } else {
            vec![R::fail(format!("{empty} image(s) with empty src"))]
        }
    }
);

#[cfg(test)]
mod tests {
    use crate::Status::{Fail, Pass, Warn};
    use crate::rules::testkit::page;

    #[test]
    fn alt_dimensions_src() {
        let good = page(r#"<img src="/a.webp" alt="" width="1" height="1">"#);
        assert_eq!(good.status("images/img-alt"), Pass);
        assert_eq!(good.status("images/img-dimensions"), Pass);
        assert_eq!(good.status("images/img-empty-src"), Pass);
        let bad = page(r#"<img src="/IMG_1234.jpg"><img src="">"#);
        assert_eq!(bad.message("images/img-alt"), "2 image(s) missing alt");
        assert_eq!(bad.results("images/img-alt")[0].evidence.as_deref(), Some("/IMG_1234.jpg, "));
        assert_eq!(bad.status("images/img-dimensions"), Warn);
        assert_eq!(bad.status("images/img-empty-src"), Fail);
    }

    #[test]
    fn format_lazy_filename() {
        assert_eq!(page(r#"<img src="/a.webp">"#).status("images/img-modern-format"), Pass);
        assert_eq!(page(r#"<img src="/a.JPG?v=2">"#).status("images/img-modern-format"), Warn);
        assert_eq!(page(r#"<img src="/a"><img src="/b" loading="lazy">"#).status("images/img-lazy-loading"), Pass);
        assert_eq!(
            page(r#"<img src="/a"><img src="/b">"#).message("images/img-lazy-loading"),
            "1 image(s) without loading=\"lazy\""
        );
        assert_eq!(page(r#"<img src="/hero-shot.webp">"#).status("images/img-filename-quality"), Pass);
        assert_eq!(page(r#"<img src="/IMG_1234.jpg">"#).status("images/img-filename-quality"), Warn);
    }
}
