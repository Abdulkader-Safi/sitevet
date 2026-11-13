//! `/llms.txt` (llmstxt.org): a Markdown file that points language models at a site's key pages.

use serde::{Deserialize, Serialize};

/// `/llms.txt` as fetched once per audit.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LlmsInfo {
    /// True when it answered 2xx.
    pub exists: bool,
    pub status: u16,
    /// True when it follows the format: an `# H1` title as the first line with content.
    pub valid: bool,
    /// The H1 title.
    pub title: Option<String>,
    /// Markdown links in its lists (`- [name](url)`).
    pub links: u32,
}

impl LlmsInfo {
    pub(crate) fn parse(status: u16, content: &str) -> Self {
        let title = content
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .and_then(|l| l.strip_prefix("# "))
            .map(|t| t.trim().to_string());
        let links = content
            .lines()
            .map(str::trim_start)
            .filter(|l| l.starts_with("- ") || l.starts_with("* "))
            .filter(|l| l.contains("](") && l.contains('['))
            .count() as u32;
        Self { exists: true, status, valid: title.is_some(), title, links }
    }
}

#[cfg(test)]
mod tests {
    use super::LlmsInfo;

    #[test]
    fn reads_the_title_and_links() {
        let ok = LlmsInfo::parse(
            200,
            "\n# Garden shop\n\n> Tools and seeds.\n\n## Docs\n\n- [Guide](https://a.com/g): start here\n- [FAQ](https://a.com/f)\n",
        );
        assert!(ok.valid);
        assert_eq!((ok.title.as_deref(), ok.links), (Some("Garden shop"), 2));
        let html = LlmsInfo::parse(200, "<!doctype html><title>Not found</title>");
        assert!(!html.valid);
        assert!(!LlmsInfo::parse(200, "## Docs only").valid);
    }
}
