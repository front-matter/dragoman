# dragoman

A web server for scholarly PID (Persistent Identifier) resolution with full [DOI content negotiation](https://citation.doi.org/docs.html). Send a DOI as the URL path; receive a redirect to the landing page or metadata in any supported format depending on the `Accept` header.

## Installation

### Prerequisites

- Rust 1.75+ ([rustup.rs](https://rustup.rs))

### Install

```bash
git clone https://github.com/front-matter/dragoman
cd dragoman
cargo install --path .
```

This builds a release binary and installs it to `~/.cargo/bin/dragoman`. Make sure `~/.cargo/bin` is on your `PATH` (the Rust installer adds this automatically).

## Local SQLite database

dragoman can serve metadata directly from a local SQLite database in the [commonmeta](https://commonmeta.org) format, bypassing the live Crossref/DataCite APIs. This dramatically reduces latency and API load for high-traffic deployments.

### Database format

The database is a SQLite3 file with a single `works` table whose columns map one-to-one to the commonmeta v1.0 schema. The `id` column is the canonical DOI URL (e.g. `https://doi.org/10.5281/zenodo.1234`). Complex fields (contributors, references, …) are stored as JSON text.

You can build a database from any commonmeta-supported source using the [commonmeta](https://github.com/front-matter/commonmeta-rs) CLI.

## Running the server

### Start

```bash
# Default port 3000
dragoman start

# Custom port
dragoman start --port 8080

# With a local database
dragoman start --db /data/commonmeta-2026-06-15.sqlite3

# Write a PID file so the process can be stopped later
dragoman start --pid-file /tmp/dragoman.pid

# All options together
dragoman start --port 8080 --db /data/commonmeta-2026-06-15.sqlite3 --pid-file /tmp/dragoman.pid
```

Options can also be supplied as environment variables (flags take precedence):

```bash
PORT=8080 DRAGOMAN_DB=/data/commonmeta-2026-06-15.sqlite3 RUST_LOG=dragoman=debug dragoman start
```

During development you can use `cargo run` in place of the installed binary:

```bash
# Run from the project root — the sqlite3 file in the root is loaded by filename
cargo run -- start --db commonmeta-2026-06-15.sqlite3

# Or with a full path
cargo run -- start --db /data/commonmeta-2026-06-15.sqlite3
```

### Error: port already in use

If the chosen port is already in use, the server logs an error and exits:

```text
ERROR dragoman: failed to bind  port=3000  error=Address already in use (os error 48)
```

Choose a different port with `--port` or stop the process that holds the port.

### Error: database file not found

If `--db` points to a path that does not exist, the server exits at startup before accepting any traffic:

```text
ERROR dragoman: failed to open database  path=…  error=sqlite file not found: '…'
```

Pass the correct path or an absolute path to avoid working-directory ambiguity.

### Stop

```bash
# Stop using the default PID file location (/tmp/dragoman.pid)
dragoman stop

# Stop using a custom PID file
dragoman stop --pid-file /var/run/dragoman.pid
```

`dragoman stop` sends `SIGTERM` to the running process. The server handles the signal gracefully: it finishes in-flight requests and removes the PID file before exiting. Pressing `Ctrl-C` has the same effect.

## CLI reference

```text
dragoman <COMMAND>

Commands:
  start  Start the server (runs in the foreground)
  stop   Stop a running server by sending SIGTERM to its PID file
  help   Print help

dragoman start [OPTIONS]
  -p, --port <PORT>          TCP port to listen on [env: PORT] [default: 3000]
  -d, --db <PATH>            Local commonmeta SQLite3 database [env: DRAGOMAN_DB]
      --pid-file <PATH>      Write PID to this file on startup [env: DRAGOMAN_PID_FILE]

dragoman stop [OPTIONS]
      --pid-file <PATH>      PID file to read [env: DRAGOMAN_PID_FILE] [default: /tmp/dragoman.pid]
```

## Environment variables

| Variable | Default | Description |
| --- | --- | --- |
| `PORT` | `3000` | TCP port to listen on. |
| `DRAGOMAN_DB` | *(none)* | Path to a local commonmeta SQLite3 database. Metadata is served from the database before falling back to the live API. The server exits on startup if the path is set but the file cannot be opened. |
| `DRAGOMAN_PID_FILE` | *(none)* | Path for the PID file written by `start` and read by `stop`. |
| `RUST_LOG` | `dragoman=info` | Log filter (see [`tracing-subscriber`](https://docs.rs/tracing-subscriber)). Use `dragoman=debug` to log per-request cache hits. |

## Usage

### Redirect (HTML / browser)

When the `Accept` header prefers `text/html` or is absent, dragoman redirects to the DOI's landing page:

```bash
# Follow the redirect
curl -L http://localhost:3000/10.5281/zenodo.1089100

# Inspect the redirect target without following
curl -s -o /dev/null -w "%{redirect_url}" http://localhost:3000/10.5281/zenodo.1089100
# https://zenodo.org/record/1089100
```

### Content negotiation

Send an `Accept` header to receive metadata instead of a redirect.

#### BibTeX

```bash
curl -H "Accept: application/x-bibtex" \
     http://localhost:3000/10.5281/zenodo.1089100
```

#### CSL-JSON

```bash
curl -H "Accept: application/vnd.citationstyles.csl+json" \
     http://localhost:3000/10.5281/zenodo.1089100
```

#### DataCite JSON

```bash
curl -H "Accept: application/vnd.datacite.datacite+json" \
     http://localhost:3000/10.5281/zenodo.1089100
```

#### RIS

```bash
curl -H "Accept: application/x-research-info-systems" \
     http://localhost:3000/10.5281/zenodo.1089100
```

#### Crossref XML

```bash
curl -H "Accept: application/vnd.crossref.unixref+xml" \
     http://localhost:3000/10.1016/j.jaci.2019.09.015
```

#### Schema.org JSON-LD

```bash
curl -H "Accept: application/vnd.schemaorg.ld+json" \
     http://localhost:3000/10.5281/zenodo.1089100
```

#### Formatted citation

`text/x-bibliography` accepts optional `style=` and `locale=` parameters. Style names come from the [CSL style repository](https://github.com/citation-style-language/styles); locale codes from the [CSL locales repository](https://github.com/citation-style-language/locales).

```bash
# APA (default)
curl -H "Accept: text/x-bibliography; style=apa" \
     http://localhost:3000/10.5281/zenodo.1089100

# Vancouver in French
curl -H "Accept: text/x-bibliography; style=vancouver; locale=fr-FR" \
     http://localhost:3000/10.5281/zenodo.1089100
```

### Query parameter overrides

Use `?format=` instead of an `Accept` header:

```bash
curl "http://localhost:3000/10.5281/zenodo.1089100?format=bibtex"
curl "http://localhost:3000/10.5281/zenodo.1089100?format=citation&style=apa&locale=de-DE"
```

Force a specific registration agency (useful for testing):

```bash
curl -H "Accept: application/x-bibtex" \
     "http://localhost:3000/10.5281/zenodo.1089100?source=datacite"
```

## Supported formats

| Accept header | `?format=` value | Notes |
| --- | --- | --- |
| `application/vnd.citationstyles.csl+json` | `csl` | |
| `application/vnd.commonmeta+json` | `commonmeta` | |
| `application/vnd.datacite.datacite+json` | `datacite` | |
| `application/vnd.datacite.datacite+xml` | `datacite_xml` | |
| `application/vnd.crossref.unixref+xml` | `crossref_xml` | |
| `application/vnd.crossref.unixsd+xml` | `crossref_xml` | alias |
| `application/x-bibtex` | `bibtex` | |
| `application/x-research-info-systems` | `ris` | |
| `application/vnd.schemaorg.ld+json` | `schemaorg` | |
| `text/x-bibliography` | `citation` | `style=` and `locale=` params |
| `text/html` / *(absent)* | — | 307 redirect to landing page |

## HTTP status codes

| Code | Meaning |
| --- | --- |
| 200 | Metadata returned |
| 307 | Redirect to landing page |
| 404 | DOI not found |
| 406 | Requested content type not supported |
| 502 | Upstream API error |

## License

MIT
