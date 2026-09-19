# exa-wreq-http1-checkpoint

- **Domain:** adapters
- **Date:** 2026-09-19
- **Pattern-Key:** exa-wreq-http1-checkpoint
- **Confidence:** high
- **Evidence:** `/tmp/wreq-exa-probe` Chrome131/137 HTTP 200 `credits_json`; reqwest/curl 429 Vercel checkpoint; `wreq` 0.16.1 needs rustc 1.98 so lock 0.15.3; cmake required to compile `boring-sys2`
- **Status:** active

## Situation

`GET dashboard.exa.ai/api/get-credits` with a valid cookie returned remaining JSON via urllib and 429 HTML via reqwest/curl. Official remaining API still does not exist.

## Decision

Fetch with `wreq` 0.15.3 + `wreq-util` 0.1.0, `Emulation::Chrome131`, `.http1_only()`. Rejected: Chrome/chrome-mcp; stock reqwest HTTP/1; `wreq` 0.16 (MSRV 1.98 vs workspace 1.85).

## Reasoning

Vercel Security Checkpoint fingerprints TLS/HTTP2. AgentOS measured this host: HTTP/2 429, Chrome TLS + HTTP/1.1 200. Live probe matched.

## Outcome

Probe: HTTP 200 `orbCreditsInCents`. Adapter `get_credits` uses that client. Build needs `cmake`.

## Lesson

Do not fetch Exa remaining with stock reqwest. Use wreq Chrome emulation and HTTP/1.1. Pin wreq 0.15.x until rustc ≥ 1.98.
