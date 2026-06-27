# dragoman

A web server for scholarly metadata with full [DOI Resolution](https://www.doi.org/the-identifier/resources/factsheets/doi-resolution-documentation) and [DOI content negotiation](https://citation.doi.org/docs.html). Send a DOI as the URL path; receive a redirect to the landing page or metadata in any supported format depending on the `Accept` header.

## Installation

### Prerequisites

- Rust 1.75+ ([rustup.rs](https://rustup.rs))

### Install

```bash
cargo install dragoman
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
# Default port 3456
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
ERROR dragoman: failed to bind  port=3456  error=Address already in use (os error 48)
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
  -p, --port <PORT>          TCP port to listen on [env: PORT] [default: 3456]
  -d, --db <PATH>            Local commonmeta SQLite3 database [env: DRAGOMAN_DB]
      --pid-file <PATH>      Write PID to this file on startup [env: DRAGOMAN_PID_FILE]

dragoman stop [OPTIONS]
      --pid-file <PATH>      PID file to read [env: DRAGOMAN_PID_FILE] [default: /tmp/dragoman.pid]
```

## Environment variables

| Variable | Default | Description |
| --- | --- | --- |
| `PORT` | `3456` | TCP port to listen on. |
| `DRAGOMAN_DB` | *(none)* | Path to a local commonmeta SQLite3 database. Metadata is served from the database before falling back to the live API. The server exits on startup if the path is set but the file cannot be opened. |
| `DRAGOMAN_PID_FILE` | *(none)* | Path for the PID file written by `start` and read by `stop`. |
| `RUST_LOG` | `dragoman=info` | Log filter (see [`tracing-subscriber`](https://docs.rs/tracing-subscriber)). Use `dragoman=debug` to log per-request cache hits. |

## Usage

### Redirect (HTML / browser)

When the `Accept` header prefers `text/html` or is absent, dragoman redirects to the DOI's landing page:

```bash
# Follow the redirect
curl -L http://localhost:3456/10.5281/zenodo.1089100

# Inspect the redirect target without following
curl -s -o /dev/null -w "%{redirect_url}" http://localhost:3456/10.5281/zenodo.1089100
# https://zenodo.org/record/1089100
```

### Content negotiation

Send an `Accept` header to receive metadata instead of a redirect.

#### BibTeX

```bash
curl -H "Accept: application/x-bibtex" \
     http://localhost:3456/10.5281/zenodo.1089100
```

#### RIS

```bash
curl -H "Accept: application/x-research-info-systems" \
     http://localhost:3456/10.5281/zenodo.1089100
```

#### CSL (Citeproc) JSON

```bash
curl -H "Accept: application/vnd.citationstyles.csl+json" \
     http://localhost:3456/10.5281/zenodo.1089100
```

#### Crossref

```bash
curl -H "Accept: application/vnd.crossref+json" \
     http://localhost:3456/10.1016/j.jaci.2019.09.015
```

#### Crossref XML

```bash
curl -H "Accept: application/vnd.crossref.unixref+xml" \
     http://localhost:3456/10.1016/j.jaci.2019.09.015
```

#### DataCite

```bash
curl -H "Accept: application/vnd.datacite.datacite+json" \
     http://localhost:3456/10.5281/zenodo.1089100
```

#### Schema.org JSON-LD

```bash
curl -H "Accept: application/vnd.schemaorg.ld+json" \
     http://localhost:3456/10.5281/zenodo.1089100
```

#### InvenioRDM

```bash
curl -H "Accept: application/vnd.inveniordm.v1+json" \
     http://localhost:3456/10.5281/zenodo.1089100
```

#### Formatted citation

`text/x-bibliography` accepts optional `style=` and `locale=` parameters. Style names come from the [CSL style repository](https://github.com/citation-style-language/styles); locale codes from the [CSL locales repository](https://github.com/citation-style-language/locales).

```bash
# APA (default)
curl -H "Accept: text/x-bibliography; style=apa" \
     http://localhost:3456/10.5281/zenodo.1089100

# Vancouver in French
curl -H "Accept: text/x-bibliography; style=vancouver; locale=fr-FR" \
     http://localhost:3456/10.5281/zenodo.1089100
```

### Query parameter overrides

Use `?format=` instead of an `Accept` header:

```bash
curl "http://localhost:3456/10.5281/zenodo.1089100?format=bibtex"
curl "http://localhost:3456/10.5281/zenodo.1089100?format=citation&style=apa&locale=de-DE"
```

## Supported formats

| Accept header | `?format=` value | Notes |
| --- | --- | --- |
| `application/x-bibtex` | `bibtex` | |
| `text/x-bibliography` | `citation` | `style=` and `locale=` params |
| `application/vnd.commonmeta+json` | `commonmeta` | |
| `application/vnd.crossref+json` | `crossref` | |
| `application/vnd.crossref.unixref+xml` | `crossref_xml` | |
| `application/vnd.crossref.unixsd+xml` | `crossref_xml` | alias |
| `application/vnd.citationstyles.csl+json` | `csl` | |
| `application/vnd.datacite.datacite+json` | `datacite` | |
| `application/vnd.datacite.datacite+xml` | `datacite_xml` | |
| `application/vnd.inveniordm.v1+json` | `inveniordm` | |
| `application/x-research-info-systems` | `ris` | |
| `application/vnd.schemaorg.ld+json` | `schemaorg` | |
| `text/html` / *(absent)* | — | 307 redirect to landing page |

## HTTP status codes

| Code | Meaning |
| --- | --- |
| 200 | Metadata returned |
| 307 | Redirect to landing page |
| 404 | DOI not found |
| 406 | Requested content type not supported |
| 502 | Upstream API error |

## Deployment (macOS)

### Installation via Homebrew

dragoman can be installed from the [front-matter Homebrew tap](https://github.com/front-matter/homebrew-tap):

```bash
brew tap front-matter/tap
brew install dragoman
```

This builds dragoman from source (requires Rust, installed automatically as a build dependency) and places the binary at `$(brew --prefix)/bin/dragoman`.

#### Recommended SQLite path

```text
$(brew --prefix)/var/dragoman/commonmeta.sqlite3
```

Which resolves to:

- `/opt/homebrew/var/dragoman/commonmeta.sqlite3` — Apple Silicon
- `/usr/local/var/dragoman/commonmeta.sqlite3` — Intel

#### Place the database

```bash
mkdir -p "$(brew --prefix)/var/dragoman"
cp commonmeta.sqlite3 "$(brew --prefix)/var/dragoman/commonmeta.sqlite3"
```

#### Run as a background service (launchd)

```bash
# Start at login and keep alive
brew services start dragoman

# Check status
brew services info dragoman

# View logs
tail -f "$(brew --prefix)/var/log/dragoman.log"

# Stop the service
brew services stop dragoman
```

`brew services start` installs a launchd plist in `~/Library/LaunchAgents/` and starts the service immediately. It restarts automatically on crash and at login.

To run as a system-level daemon (starts at boot, not tied to a user login), use `sudo brew services start dragoman`. This installs the plist in `/Library/LaunchDaemons/` instead.

#### Configuration

To change the port or other settings, edit the service environment variables and restart:

```bash
# Open the generated plist for editing
open "$(brew --prefix)/opt/dragoman/homebrew.mxcl.dragoman.plist"
brew services restart dragoman
```

### Manual installation (without Homebrew)

```bash
# Install Rust if not already installed
curl https://sh.rustup.rs -sSf | sh

cargo install dragoman
sudo install -m 755 ~/.cargo/bin/dragoman /opt/homebrew/bin/dragoman
```

> **Intel Macs:** replace `/opt/homebrew` with `/usr/local` in all paths below.

#### Run as a launchd daemon

The bundled `com.front-matter.dragoman.plist` targets Apple Silicon paths.

```bash
sudo mkdir -p /opt/homebrew/var/dragoman /opt/homebrew/var/log
sudo cp commonmeta.sqlite3 /opt/homebrew/var/dragoman/commonmeta.sqlite3

sudo cp com.front-matter.dragoman.plist /Library/LaunchDaemons/
sudo launchctl load -w /Library/LaunchDaemons/com.front-matter.dragoman.plist
```

Check logs:

```bash
tail -f /opt/homebrew/var/log/dragoman.log
```

Stop and disable:

```bash
sudo launchctl unload -w /Library/LaunchDaemons/com.front-matter.dragoman.plist
```

### Updating

#### With Homebrew

```bash
brew upgrade dragoman
brew services restart dragoman
```

#### Manual

```bash
cargo install dragoman
sudo install -m 755 ~/.cargo/bin/dragoman /opt/homebrew/bin/dragoman
sudo launchctl kickstart -k system/com.front-matter.dragoman
```

---

## Deployment (Debian / systemd)

This section covers running dragoman as a persistent system service on a Debian 13 server.

### 1. Build the binary

On the server, install Rust and install the binary:

```bash
curl https://sh.rustup.rs -sSf | sh
source ~/.cargo/env
cargo install dragoman
sudo install -m 755 ~/.cargo/bin/dragoman /usr/local/bin/dragoman
```

Or cross-compile locally and copy the binary:

```bash
# macOS → Linux x86-64 (requires cross)
cargo install cross
cross build --release --target x86_64-unknown-linux-gnu
scp target/x86_64-unknown-linux-gnu/release/dragoman user@server:/tmp/dragoman
ssh user@server 'sudo install -m 755 /tmp/dragoman /usr/local/bin/dragoman'
```

### 2. Create system user and directories

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin dragoman
sudo mkdir -p /var/lib/dragoman /etc/dragoman
sudo chown dragoman:dragoman /var/lib/dragoman
```

### 3. Place the SQLite database

The recommended database path is `/var/lib/dragoman/commonmeta.sqlite3`:

```bash
sudo cp commonmeta.sqlite3 /var/lib/dragoman/commonmeta.sqlite3
sudo chown dragoman:dragoman /var/lib/dragoman/commonmeta.sqlite3
```

### 4. Create the environment file

```bash
sudo tee /etc/dragoman/env > /dev/null <<'EOF'
PORT=3456
DRAGOMAN_DB=/var/lib/dragoman/commonmeta.sqlite3
RUST_LOG=dragoman=info
EOF
sudo chmod 640 /etc/dragoman/env
sudo chown root:dragoman /etc/dragoman/env
```

### 5. Install and enable the systemd unit

```bash
sudo cp dragoman.service /etc/systemd/system/dragoman.service
sudo systemctl daemon-reload
sudo systemctl enable --now dragoman
```

Check the service is running:

```bash
sudo systemctl status dragoman
sudo journalctl -u dragoman -f
```

### Updating the binary

```bash
cargo install dragoman
sudo install -m 755 ~/.cargo/bin/dragoman /usr/local/bin/dragoman
sudo systemctl restart dragoman
```

### Updating the database

The database file can be replaced while the service is running. dragoman opens
the SQLite file once at startup; to pick up a new file, restart the service:

```bash
sudo cp commonmeta-new.sqlite3 /var/lib/dragoman/commonmeta.sqlite3
sudo chown dragoman:dragoman /var/lib/dragoman/commonmeta.sqlite3
sudo systemctl restart dragoman
```

### Reverse proxy

#### Caddy (standalone)

[Caddy](https://caddyserver.com) is the recommended reverse proxy for standalone deployments. It handles TLS certificates automatically via Let's Encrypt.

```bash
sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https curl
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' \
  | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' \
  | sudo tee /etc/apt/sources.list.d/caddy-stable.list
sudo apt update && sudo apt install caddy
```

Add a site block to `/etc/caddy/Caddyfile`:

```caddy
doi.example.com {
    reverse_proxy localhost:3456
}
```

Reload Caddy:

```bash
sudo systemctl reload caddy
```

#### Traefik via Coolify

If the server already runs [Coolify](https://coolify.io), its Traefik instance can route directly to the dragoman systemd service. Add a file-provider config to the Coolify dynamic configuration directory:

```bash
sudo tee /data/coolify/proxy/dynamic/dragoman.yml > /dev/null <<'EOF'
http:
  routers:
    dragoman:
      rule: "Host(`doi.example.com`)"
      service: dragoman
      entryPoints:
        - https
      tls:
        certResolver: letsencrypt
  services:
    dragoman:
      loadBalancer:
        servers:
          - url: "http://host.docker.internal:3456"
EOF
```

Traefik picks up the file automatically — no reload needed. `host.docker.internal` resolves to the host from inside the Traefik container; dragoman must listen on all interfaces (`0.0.0.0:3456`, which is the default).

## License

MIT
