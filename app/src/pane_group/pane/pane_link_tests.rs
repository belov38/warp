use super::{
    MAX_PANE_LINK_LABEL_CHARS, MAX_PANE_LINK_URL_CHARS, MAX_PANE_LINKS, PaneLink, PaneLinkError,
    remove_link, upsert_link,
};

fn link(label: &str, url: &str) -> PaneLink {
    PaneLink::validate(label, url).expect("valid link")
}

#[test]
fn validate_trims_label_and_accepts_https() {
    let link =
        PaneLink::validate("  DELI-1878  ", "https://linear.app/x/issue/DELI-1878").expect("valid");
    assert_eq!(link.label, "DELI-1878");
    assert_eq!(link.url, "https://linear.app/x/issue/DELI-1878");
}

#[test]
fn validate_accepts_http() {
    assert!(PaneLink::validate("ci", "http://localhost:8080/run/1").is_ok());
}

#[test]
fn validate_rejects_empty_or_whitespace_label() {
    assert_eq!(
        PaneLink::validate("   ", "https://a.b").unwrap_err(),
        PaneLinkError::EmptyLabel
    );
}

#[test]
fn validate_rejects_label_over_64_chars() {
    let long = "x".repeat(MAX_PANE_LINK_LABEL_CHARS + 1);
    assert_eq!(
        PaneLink::validate(&long, "https://a.b").unwrap_err(),
        PaneLinkError::LabelTooLong {
            max: MAX_PANE_LINK_LABEL_CHARS
        }
    );
    let ok = "x".repeat(MAX_PANE_LINK_LABEL_CHARS);
    assert!(PaneLink::validate(&ok, "https://a.b").is_ok());
}

#[test]
fn validate_rejects_control_characters_in_label() {
    assert_eq!(
        PaneLink::validate("a\nb", "https://a.b").unwrap_err(),
        PaneLinkError::LabelHasControlCharacters
    );
    assert_eq!(
        PaneLink::validate("a\u{7}b", "https://a.b").unwrap_err(),
        PaneLinkError::LabelHasControlCharacters
    );
}

#[test]
fn validate_rejects_url_over_2048_chars() {
    let long = format!("https://a.b/{}", "x".repeat(MAX_PANE_LINK_URL_CHARS));
    assert_eq!(
        PaneLink::validate("l", &long).unwrap_err(),
        PaneLinkError::UrlTooLong {
            max: MAX_PANE_LINK_URL_CHARS
        }
    );
}

#[test]
fn validate_rejects_non_http_schemes_and_relative_urls() {
    assert_eq!(
        PaneLink::validate("l", "javascript:alert(1)").unwrap_err(),
        PaneLinkError::UrlSchemeNotAllowed {
            scheme: "javascript".to_owned()
        }
    );
    assert_eq!(
        PaneLink::validate("l", "ftp://host/file").unwrap_err(),
        PaneLinkError::UrlSchemeNotAllowed {
            scheme: "ftp".to_owned()
        }
    );
    assert_eq!(
        PaneLink::validate("l", "/just/a/path").unwrap_err(),
        PaneLinkError::UrlNotParseable
    );
    assert_eq!(
        PaneLink::validate("l", "").unwrap_err(),
        PaneLinkError::UrlNotParseable
    );
}

#[test]
fn upsert_appends_in_order_until_cap() {
    let mut links = Vec::new();
    assert_eq!(upsert_link(&mut links, link("a", "https://a")), Ok(true));
    assert_eq!(upsert_link(&mut links, link("b", "https://b")), Ok(true));
    assert_eq!(upsert_link(&mut links, link("c", "https://c")), Ok(true));
    assert_eq!(
        links.iter().map(|l| l.label.as_str()).collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
    assert_eq!(
        upsert_link(&mut links, link("d", "https://d")),
        Err(PaneLinkError::TooManyLinks {
            max: MAX_PANE_LINKS
        })
    );
    assert_eq!(links.len(), 3);
}

#[test]
fn upsert_replaces_url_in_place_and_reports_no_change_for_identical() {
    let mut links = vec![link("a", "https://a"), link("b", "https://b")];
    assert_eq!(upsert_link(&mut links, link("a", "https://a2")), Ok(true));
    assert_eq!(links[0].url, "https://a2");
    assert_eq!(links[1].label, "b");
    assert_eq!(upsert_link(&mut links, link("a", "https://a2")), Ok(false));
}

#[test]
fn upsert_at_cap_still_replaces_existing_label() {
    let mut links = vec![
        link("a", "https://a"),
        link("b", "https://b"),
        link("c", "https://c"),
    ];
    assert_eq!(upsert_link(&mut links, link("b", "https://b2")), Ok(true));
    assert_eq!(links[1].url, "https://b2");
}

#[test]
fn labels_compare_case_sensitively() {
    let mut links = vec![link("PR", "https://a")];
    assert_eq!(upsert_link(&mut links, link("pr", "https://b")), Ok(true));
    assert_eq!(links.len(), 2);
}

#[test]
fn remove_keeps_order_and_errors_on_unknown_label() {
    let mut links = vec![
        link("a", "https://a"),
        link("b", "https://b"),
        link("c", "https://c"),
    ];
    assert_eq!(remove_link(&mut links, "b"), Ok(()));
    assert_eq!(
        links.iter().map(|l| l.label.as_str()).collect::<Vec<_>>(),
        ["a", "c"]
    );
    assert_eq!(
        remove_link(&mut links, "zzz"),
        Err(PaneLinkError::NoSuchLabel {
            label: "zzz".to_owned()
        })
    );
}

#[test]
fn remove_trims_the_label_argument() {
    let mut links = vec![link("a", "https://a")];
    assert_eq!(remove_link(&mut links, "  a "), Ok(()));
    assert!(links.is_empty());
}

#[test]
fn error_messages_match_the_product_spec() {
    assert_eq!(
        PaneLinkError::TooManyLinks { max: 3 }.to_string(),
        "pane already has 3 links"
    );
    assert_eq!(
        PaneLinkError::NoSuchLabel {
            label: "x".to_owned()
        }
        .to_string(),
        "no link with label \"x\""
    );
}
