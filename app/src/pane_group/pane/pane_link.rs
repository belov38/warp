//! User-defined `{label, url}` links attached to a pane and rendered as chips
//! on the pane's Vertical Tabs card. See `specs/GH16011/product.md`.

use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};
use url::Url;

pub const MAX_PANE_LINKS: usize = 3;
pub const MAX_PANE_LINK_LABEL_CHARS: usize = 64;
pub const MAX_PANE_LINK_URL_CHARS: usize = 2048;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneLink {
    pub label: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaneLinkError {
    EmptyLabel,
    LabelTooLong { max: usize },
    LabelHasControlCharacters,
    UrlTooLong { max: usize },
    UrlNotParseable,
    UrlSchemeNotAllowed { scheme: String },
    TooManyLinks { max: usize },
    NoSuchLabel { label: String },
}

impl Display for PaneLinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyLabel => write!(f, "label cannot be empty"),
            Self::LabelTooLong { max } => write!(f, "label is longer than {max} characters"),
            Self::LabelHasControlCharacters => {
                write!(f, "label cannot contain control characters")
            }
            Self::UrlTooLong { max } => write!(f, "url is longer than {max} characters"),
            Self::UrlNotParseable => write!(f, "url must be an absolute http(s) URL"),
            Self::UrlSchemeNotAllowed { scheme } => {
                write!(
                    f,
                    "url scheme \"{scheme}\" is not allowed; use http or https"
                )
            }
            Self::TooManyLinks { max } => write!(f, "pane already has {max} links"),
            Self::NoSuchLabel { label } => write!(f, "no link with label \"{label}\""),
        }
    }
}

impl PaneLink {
    pub fn validate(label: &str, url: &str) -> Result<PaneLink, PaneLinkError> {
        let label = label.trim();
        if label.is_empty() {
            return Err(PaneLinkError::EmptyLabel);
        }
        if label.chars().count() > MAX_PANE_LINK_LABEL_CHARS {
            return Err(PaneLinkError::LabelTooLong {
                max: MAX_PANE_LINK_LABEL_CHARS,
            });
        }
        if label.chars().any(char::is_control) {
            return Err(PaneLinkError::LabelHasControlCharacters);
        }

        let url = url.trim();
        if url.chars().count() > MAX_PANE_LINK_URL_CHARS {
            return Err(PaneLinkError::UrlTooLong {
                max: MAX_PANE_LINK_URL_CHARS,
            });
        }
        let parsed = Url::parse(url).map_err(|_| PaneLinkError::UrlNotParseable)?;
        match parsed.scheme() {
            "http" | "https" => {}
            scheme => {
                return Err(PaneLinkError::UrlSchemeNotAllowed {
                    scheme: scheme.to_owned(),
                });
            }
        }

        Ok(PaneLink {
            label: label.to_owned(),
            url: url.to_owned(),
        })
    }
}

/// Appends `link`, or replaces the URL of the existing link with the same
/// label in place. Returns `Ok(true)` when the list changed.
pub fn upsert_link(links: &mut Vec<PaneLink>, link: PaneLink) -> Result<bool, PaneLinkError> {
    if let Some(existing) = links
        .iter_mut()
        .find(|existing| existing.label == link.label)
    {
        if existing.url == link.url {
            return Ok(false);
        }
        existing.url = link.url;
        return Ok(true);
    }
    if links.len() >= MAX_PANE_LINKS {
        return Err(PaneLinkError::TooManyLinks {
            max: MAX_PANE_LINKS,
        });
    }
    links.push(link);
    Ok(true)
}

/// Removes the link whose label equals `label` (after trimming), keeping the
/// order of the remaining links.
pub fn remove_link(links: &mut Vec<PaneLink>, label: &str) -> Result<(), PaneLinkError> {
    let label = label.trim();
    let Some(index) = links.iter().position(|link| link.label == label) else {
        return Err(PaneLinkError::NoSuchLabel {
            label: label.to_owned(),
        });
    };
    links.remove(index);
    Ok(())
}

#[cfg(test)]
#[path = "pane_link_tests.rs"]
mod tests;
