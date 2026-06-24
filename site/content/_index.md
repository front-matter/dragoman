+++
title = "dragoman"
description = "DOI content negotiation server"
+++

Send a DOI as the URL path. Receive a redirect to the landing page, or metadata in any supported format based on your `Accept` header.

## Usage

```http
GET {{ base_url() }}/{doi}
```

Example — fetch BibTeX for a Zenodo record:

```bash
curl -H "Accept: application/x-bibtex" \
     {{ base_url() }}/10.5281/zenodo.1089100
```

Use `?format=` instead of an `Accept` header:

```text
{{ base_url() }}/10.5281/zenodo.1089100?format=bibtex
```

## Supported formats

| Accept header | `format=` |
| --- | --- |
| `application/x-bibtex` | `bibtex` |
| `text/x-bibliography` | `citation` |
| `application/vnd.commonmeta+json` | `commonmeta` |
| `application/vnd.crossref.unixref+xml` | `crossref_xml` |
| `application/vnd.citationstyles.csl+json` | `csl` |
| `application/vnd.datacite.datacite+json` | `datacite` |
| `application/vnd.datacite.datacite+xml` | `datacite_xml` |
| `application/vnd.inveniordm.v1+json` | `inveniordm` |
| `application/x-research-info-systems` | `ris` |
| `application/vnd.schemaorg.ld+json` | `schemaorg` |
| `text/html` / *absent* | redirect to landing page |

## Query parameters

| Parameter | Description |
| --- | --- |
| `format` | Override `Accept` header |
| `style` | CSL citation style (e.g. `apa`, `vancouver`); only with `citation` |
| `locale` | Citation locale (e.g. `fr-FR`); only with `citation` |
| `source` | Override registration agency: `crossref` or `datacite` |

## Content negotiation with quality values

Multiple types with `q=` weights are supported per [RFC 7231](https://www.rfc-editor.org/rfc/rfc7231#section-5.3.2):

```bash
curl -H "Accept: application/vnd.citationstyles.csl+json;q=0.9, application/x-bibtex" \
     {{ base_url() }}/10.5281/zenodo.1089100
```
