/// Describes a resolved content negotiation result.
pub struct Negotiated {
    pub format: &'static str,
    pub content_type: &'static str,
    /// Extracted from `Accept: text/x-bibliography; style=apa`
    pub style: Option<String>,
    /// Extracted from `Accept: text/x-bibliography; locale=fr-FR`
    pub locale: Option<String>,
}

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
