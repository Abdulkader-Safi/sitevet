//! robots.txt: obey Disallow rules for our agent and `*`, and name the rule that matched.

use serde::{Deserialize, Serialize};
use texting_robots::Robot;

/// robots.txt as fetched once per audit.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RobotsInfo {
    /// True when it answered 2xx.
    pub exists: bool,
    pub status: u16,
    /// At most 500 KB.
    pub content: String,
    /// `Sitemap:` lines.
    pub sitemaps: Vec<String>,
}

pub(crate) struct RobotsGuard {
    robot: Option<Robot>,
    disallows: Vec<String>,
}

impl RobotsGuard {
    /// `agent` is the product token of the user agent, e.g. `sitevet`. With `obey` off, nothing is blocked.
    pub(crate) fn new(info: &RobotsInfo, agent: &str, obey: bool) -> Self {
        let robot = (obey && info.exists).then(|| Robot::new(agent, info.content.as_bytes()).ok()).flatten();
        let disallows = info
            .content
            .lines()
            .filter_map(|l| {
                let l = l.split('#').next().unwrap_or("").trim();
                let (key, value) = l.split_once(':')?;
                (key.trim().eq_ignore_ascii_case("disallow") && !value.trim().is_empty())
                    .then(|| value.trim().to_string())
            })
            .collect();
        Self { robot, disallows }
    }

    pub(crate) fn allowed(&self, url: &url::Url) -> bool {
        self.robot.as_ref().is_none_or(|r| r.allowed(url.as_str()))
    }

    /// Best effort: the longest Disallow pattern that matches the path. It ignores which agent
    /// group the line sits in, so it is a label for people, not the decision itself.
    pub(crate) fn matching_rule(&self, url: &url::Url) -> Option<String> {
        let path = match url.query() {
            Some(q) => format!("{}?{q}", url.path()),
            None => url.path().to_string(),
        };
        self.disallows
            .iter()
            .filter(|p| glob_prefix(p, &path))
            .max_by_key(|p| p.len())
            .map(|p| format!("Disallow: {p}"))
    }
}

/// robots.txt matching: `*` is any run of characters, a trailing `$` anchors the end, otherwise
/// the pattern is a prefix.
fn glob_prefix(pattern: &str, path: &str) -> bool {
    let (pattern, anchored) = match pattern.strip_suffix('$') {
        Some(p) => (p, true),
        None => (pattern, false),
    };
    let mut parts = pattern.split('*');
    let Some(first) = parts.next() else { return false };
    let Some(mut rest) = path.strip_prefix(first) else { return false };
    let parts: Vec<&str> = parts.collect();
    for (i, part) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        if last && anchored {
            return rest.ends_with(part);
        }
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    !anchored || rest.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guard(txt: &str) -> RobotsGuard {
        let info = RobotsInfo { exists: true, status: 200, content: txt.into(), sitemaps: vec![] };
        RobotsGuard::new(&info, "sitevet", true)
    }

    #[test]
    fn obeys_our_group_and_names_the_rule() {
        let g = guard("User-agent: *\nDisallow: /private\nDisallow: /*.pdf$\n");
        let u = |p: &str| url::Url::parse(&format!("https://s.test{p}")).unwrap();
        assert!(!g.allowed(&u("/private/x")));
        assert_eq!(g.matching_rule(&u("/private/x")).as_deref(), Some("Disallow: /private"));
        assert!(!g.allowed(&u("/doc/a.pdf")));
        assert_eq!(g.matching_rule(&u("/doc/a.pdf")).as_deref(), Some("Disallow: /*.pdf$"));
        assert!(g.allowed(&u("/public")));
        assert!(g.allowed(&u("/doc/a.pdfx")));
    }

    #[test]
    fn ignore_robots_allows_everything() {
        let info =
            RobotsInfo { exists: true, status: 200, content: "User-agent: *\nDisallow: /".into(), sitemaps: vec![] };
        assert!(RobotsGuard::new(&info, "sitevet", false).allowed(&url::Url::parse("https://s.test/a").unwrap()));
    }
}
