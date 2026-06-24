/// Describes a resolved content negotiation result.
#[derive(Debug, PartialEq)]
pub struct Negotiated {
    pub format: &'static str,
    pub content_type: &'static str,
    /// Extracted from `Accept: text/x-bibliography; style=apa`
    pub style: Option<String>,
    /// Extracted from `Accept: text/x-bibliography; locale=fr-FR`
    pub locale: Option<String>,
}

#[derive(Debug)]
pub enum NegotiateResult {
    /// Serve the matched format.
    Format(Negotiated),
    /// Client wants HTML or sent no specific preference — redirect to landing page.
    Redirect,
    /// Client sent specific types we don't support — return 406.
    NotAcceptable,
}

/// MIME-type-to-format table following citation.doi.org content negotiation spec.
///
/// Ordered by specificity; more specific types appear before generic aliases.
static FORMATS: &[(&str, &str, &str)] = &[
    // (accept_mime, commonmeta_format, response_content_type)
    (
        "application/vnd.commonmeta+json",
        "commonmeta",
        "application/vnd.commonmeta+json; charset=utf-8",
    ),
    // Canonical CSL-JSON MIME per citationstyles.org / citation.doi.org spec
    (
        "application/vnd.citationstyles.csl+json",
        "csl",
        "application/vnd.citationstyles.csl+json; charset=utf-8",
    ),
    (
        "application/vnd.datacite.datacite+json",
        "datacite",
        "application/vnd.datacite.datacite+json; charset=utf-8",
    ),
    (
        "application/vnd.datacite.datacite+xml",
        "datacite_xml",
        "application/vnd.datacite.datacite+xml; charset=utf-8",
    ),
    // Unixref is the canonical Crossref XML format per the spec
    (
        "application/vnd.crossref.unixref+xml",
        "crossref_xml",
        "application/vnd.crossref.unixref+xml; charset=utf-8",
    ),
    // Unixsd is an older Crossref XML variant; map to the same format
    (
        "application/vnd.crossref.unixsd+xml",
        "crossref_xml",
        "application/vnd.crossref.unixsd+xml; charset=utf-8",
    ),
    (
        "application/x-bibtex",
        "bibtex",
        "application/x-bibtex; charset=utf-8",
    ),
    (
        "application/x-research-info-systems",
        "ris",
        "application/x-research-info-systems; charset=utf-8",
    ),
    (
        "application/vnd.schemaorg.ld+json",
        "schemaorg",
        "application/vnd.schemaorg.ld+json; charset=utf-8",
    ),
    (
        "application/ld+json",
        "schemaorg",
        "application/ld+json; charset=utf-8",
    ),
    // style= and locale= parameters are parsed from this entry's Accept value
    (
        "text/x-bibliography",
        "citation",
        "text/x-bibliography; charset=utf-8",
    ),
    // Backwards-compat alias used before the spec clarified the CSL MIME type
    (
        "application/vnd.crossref.unixsd+json",
        "csl",
        "application/vnd.citationstyles.csl+json; charset=utf-8",
    ),
    // Generic JSON fallback → CSL-JSON
    (
        "application/json",
        "csl",
        "application/json; charset=utf-8",
    ),
];

/// Returns a `NegotiateResult` for the best Accept-header match.
///
/// Processing is left-to-right (first match wins, matching browser convention).
/// `text/html` and `*/*` both resolve to `Redirect`. Any Accept entry that
/// doesn't match a known format or html/wildcard is skipped; if the Accept list
/// is exhausted without any match, `NotAcceptable` is returned (→ HTTP 406).
///
/// Parses `style=` and `locale=` from `text/x-bibliography` parameters, e.g.:
///   `Accept: text/x-bibliography; style=apa; locale=fr-FR`
pub fn negotiate(accept: &str) -> NegotiateResult {
    if accept.trim().is_empty() {
        return NegotiateResult::Redirect;
    }

    for part in accept.split(',') {
        let segments: Vec<&str> = part.split(';').collect();
        let media_type = segments[0].trim();

        if media_type.eq_ignore_ascii_case("text/html") || media_type == "*/*" {
            return NegotiateResult::Redirect;
        }

        for (mime, format, content_type) in FORMATS {
            if media_type.eq_ignore_ascii_case(mime) {
                let (style, locale) = if *format == "citation" {
                    parse_style_locale(&segments[1..])
                } else {
                    (None, None)
                };
                return NegotiateResult::Format(Negotiated {
                    format,
                    content_type,
                    style,
                    locale,
                });
            }
        }
        // Unknown media type — continue to the next preference in the list
    }

    // All Accept entries were unrecognised specific types
    NegotiateResult::NotAcceptable
}

fn parse_style_locale(params: &[&str]) -> (Option<String>, Option<String>) {
    let mut style = None;
    let mut locale = None;
    for param in params {
        if let Some((k, v)) = param.trim().split_once('=') {
            match k.trim() {
                "style" => style = Some(v.trim().to_string()),
                "locale" => locale = Some(v.trim().to_string()),
                _ => {}
            }
        }
    }
    (style, locale)
}

/// Returns the response Content-Type for a given commonmeta format name.
/// Used when the client overrides via `?format=`.
pub fn content_type_for_format(format: &str) -> Option<&'static str> {
    FORMATS
        .iter()
        .find(|(_, f, _)| *f == format)
        .map(|(_, _, ct)| *ct)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── negotiate() ──────────────────────────────────────────────────────────

    #[test]
    fn empty_accept_redirects() {
        assert!(matches!(negotiate(""), NegotiateResult::Redirect));
    }

    #[test]
    fn html_redirects() {
        assert!(matches!(negotiate("text/html"), NegotiateResult::Redirect));
    }

    #[test]
    fn html_with_charset_redirects() {
        assert!(matches!(
            negotiate("text/html; charset=utf-8"),
            NegotiateResult::Redirect
        ));
    }

    #[test]
    fn wildcard_redirects() {
        assert!(matches!(negotiate("*/*"), NegotiateResult::Redirect));
    }

    #[test]
    fn browser_accept_redirects() {
        // Typical browser Accept header
        let browser = "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";
        assert!(matches!(negotiate(browser), NegotiateResult::Redirect));
    }

    #[test]
    fn unsupported_type_returns_406() {
        assert!(matches!(
            negotiate("application/rdf+xml"),
            NegotiateResult::NotAcceptable
        ));
    }

    #[test]
    fn multiple_unsupported_returns_406() {
        assert!(matches!(
            negotiate("application/rdf+xml, image/png"),
            NegotiateResult::NotAcceptable
        ));
    }

    #[test]
    fn unsupported_then_json_falls_through() {
        // rdf+xml is skipped; application/json is matched as CSL
        match negotiate("application/rdf+xml, application/json") {
            NegotiateResult::Format(n) => assert_eq!(n.format, "csl"),
            other => panic!("expected Format, got {other:?}"),
        }
    }

    #[test]
    fn unsupported_then_wildcard_redirects() {
        assert!(matches!(
            negotiate("application/rdf+xml, */*"),
            NegotiateResult::Redirect
        ));
    }

    #[test]
    fn bibtex_resolves() {
        match negotiate("application/x-bibtex") {
            NegotiateResult::Format(n) => {
                assert_eq!(n.format, "bibtex");
                assert!(n.content_type.contains("application/x-bibtex"));
                assert!(n.style.is_none());
                assert!(n.locale.is_none());
            }
            other => panic!("expected Format, got {other:?}"),
        }
    }

    #[test]
    fn ris_resolves() {
        match negotiate("application/x-research-info-systems") {
            NegotiateResult::Format(n) => assert_eq!(n.format, "ris"),
            other => panic!("expected Format, got {other:?}"),
        }
    }

    #[test]
    fn csl_json_canonical_mime() {
        match negotiate("application/vnd.citationstyles.csl+json") {
            NegotiateResult::Format(n) => {
                assert_eq!(n.format, "csl");
                assert!(n.content_type.contains("citationstyles.csl+json"));
            }
            other => panic!("expected Format, got {other:?}"),
        }
    }

    #[test]
    fn application_json_falls_back_to_csl() {
        match negotiate("application/json") {
            NegotiateResult::Format(n) => assert_eq!(n.format, "csl"),
            other => panic!("expected Format, got {other:?}"),
        }
    }

    #[test]
    fn crossref_unixsd_json_alias_maps_to_csl() {
        match negotiate("application/vnd.crossref.unixsd+json") {
            NegotiateResult::Format(n) => assert_eq!(n.format, "csl"),
            other => panic!("expected Format, got {other:?}"),
        }
    }

    #[test]
    fn schemaorg_ld_json_resolves() {
        match negotiate("application/vnd.schemaorg.ld+json") {
            NegotiateResult::Format(n) => assert_eq!(n.format, "schemaorg"),
            other => panic!("expected Format, got {other:?}"),
        }
    }

    #[test]
    fn ld_json_resolves_to_schemaorg() {
        match negotiate("application/ld+json") {
            NegotiateResult::Format(n) => assert_eq!(n.format, "schemaorg"),
            other => panic!("expected Format, got {other:?}"),
        }
    }

    #[test]
    fn datacite_json_resolves() {
        match negotiate("application/vnd.datacite.datacite+json") {
            NegotiateResult::Format(n) => assert_eq!(n.format, "datacite"),
            other => panic!("expected Format, got {other:?}"),
        }
    }

    #[test]
    fn crossref_xml_resolves() {
        match negotiate("application/vnd.crossref.unixref+xml") {
            NegotiateResult::Format(n) => assert_eq!(n.format, "crossref_xml"),
            other => panic!("expected Format, got {other:?}"),
        }
    }

    #[test]
    fn bibliography_without_params() {
        match negotiate("text/x-bibliography") {
            NegotiateResult::Format(n) => {
                assert_eq!(n.format, "citation");
                assert!(n.style.is_none());
                assert!(n.locale.is_none());
            }
            other => panic!("expected Format, got {other:?}"),
        }
    }

    #[test]
    fn bibliography_with_style() {
        match negotiate("text/x-bibliography; style=apa") {
            NegotiateResult::Format(n) => {
                assert_eq!(n.format, "citation");
                assert_eq!(n.style.as_deref(), Some("apa"));
                assert!(n.locale.is_none());
            }
            other => panic!("expected Format, got {other:?}"),
        }
    }

    #[test]
    fn bibliography_with_style_and_locale() {
        match negotiate("text/x-bibliography; style=apa; locale=de-DE") {
            NegotiateResult::Format(n) => {
                assert_eq!(n.format, "citation");
                assert_eq!(n.style.as_deref(), Some("apa"));
                assert_eq!(n.locale.as_deref(), Some("de-DE"));
            }
            other => panic!("expected Format, got {other:?}"),
        }
    }

    #[test]
    fn mime_matching_is_case_insensitive() {
        match negotiate("Application/X-BibTeX") {
            NegotiateResult::Format(n) => assert_eq!(n.format, "bibtex"),
            other => panic!("expected Format, got {other:?}"),
        }
    }

    // ── content_type_for_format() ─────────────────────────────────────────────

    #[test]
    fn known_formats_return_content_type() {
        for fmt in &["bibtex", "ris", "csl", "schemaorg", "citation", "datacite", "crossref_xml", "commonmeta"] {
            assert!(
                content_type_for_format(fmt).is_some(),
                "format '{fmt}' should have a content type"
            );
        }
    }

    #[test]
    fn unknown_format_returns_none() {
        assert!(content_type_for_format("totally-made-up").is_none());
    }
}
