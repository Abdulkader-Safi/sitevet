//! The rendered stage: a local Chrome (feature `browser`), or PageSpeed Insights when a key is
//! set and the browser is off. It fills [`RenderedData`] for the rendered-stage rules.

#[cfg(feature = "browser")]
mod browser;
mod psi;

use std::collections::HashMap;

use crate::rules::RenderedData;
use crate::{Options, Result};

/// The reason rendered-stage rules show when nothing renders.
pub(crate) const NEEDS_RENDERING: &str = "needs browser or psi_key";

pub(crate) enum Provider {
    #[cfg(feature = "browser")]
    Browser(Box<browser::Renderer>),
    Psi(psi::Psi),
}

/// Picks and starts the provider before the crawl, so a missing Chrome fails the audit early.
pub(crate) async fn prepare(options: &Options) -> Result<Option<Provider>> {
    if options.browser {
        #[cfg(feature = "browser")]
        return Ok(Some(Provider::Browser(Box::new(browser::Renderer::launch(options).await?))));
    }
    let key = options.psi_key.clone().or_else(|| std::env::var("SITEVET_PSI_KEY").ok()).filter(|k| !k.is_empty());
    Ok(key.map(|k| Provider::Psi(psi::Psi::new(k, options.psi_max_pages as usize))))
}

/// What the stage produced: data per page URL, and a reason when it stopped early.
pub(crate) struct Rendered {
    pub(crate) pages: HashMap<String, RenderedData>,
    pub(crate) failed: Option<String>,
}

impl Provider {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            #[cfg(feature = "browser")]
            Provider::Browser(_) => "browser",
            Provider::Psi(_) => "psi",
        }
    }

    /// Renders the pages (PSI: only the first `psi_max_pages`). Keys are the page URLs given.
    pub(crate) async fn render(&mut self, urls: &[String]) -> Rendered {
        match self {
            #[cfg(feature = "browser")]
            Provider::Browser(b) => b.render(urls).await,
            Provider::Psi(p) => p.render(urls).await,
        }
    }

    /// The page's DOM after scripts ran, for `render_crawl`. None without a browser.
    #[cfg_attr(not(feature = "browser"), allow(unused_variables))]
    pub(crate) async fn rendered_html(&self, url: &str) -> Option<String> {
        match self {
            #[cfg(feature = "browser")]
            Provider::Browser(b) => b.rendered_html(url).await,
            Provider::Psi(_) => None,
        }
    }

    /// Stops the browser now rather than at drop.
    pub(crate) async fn close(self) {
        match self {
            #[cfg(feature = "browser")]
            Provider::Browser(b) => (*b).close().await,
            Provider::Psi(_) => {}
        }
    }
}
