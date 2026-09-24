# crates/token-balance/src/adapters/

Vendor HTTP fetchers. One module per vendor. Recorded JSON tests only; no live vendor HTTP.

## Chain

- Up: `../AGENTS.md`
- Down: (none)

## Delta

- Never call Moonshot PAYG balance, Anthropic Admin Usage, or xAI prepaid on plan cards
- Never call Exa Team Management `/usage` (`total_cost_usd`) as remaining
- Never commit `exa.json` dashboard cookies
- Never fetch Exa remaining with stock reqwest (Vercel checkpoint 429); use wreq HTTP/1
- Never open Chrome or call chrome-mcp from adapters
- Never poll Firecrawl `/scrape` for remaining; use `/v2/team/credit-usage` only
- Never treat Muse HTTP 429 + `resets_at` as a dead token; it is exhausted Everyday
