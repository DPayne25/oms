# Multi-Account Trade Copier / OMS

A local-first order management system that takes a single trade intent and executes it
across a managed pool of broker accounts concurrently, with position size calculated
per-account from risk percentage — not entered by hand.

Built for running a CTA-style operation across many small/large accounts without manual
per-terminal execution.

## 1. Problem

Executing the same trade idea across multiple TradeLocker (and eventually Oanda) accounts
by hand introduces:

- **Execution latency** — accounts get filled at different prices depending on how fast
  you click through each terminal.
- **Inconsistent risk** — manually sizing qty per account is error-prone, especially
  across accounts with different balances.
- **No unified record** — trade history lives inside each broker terminal, not in one
  place you can query or report from.

## 2. Solution — v1 Build Checklist

v1 scope is **TradeLocker only, market orders only**. Oanda and DB ingestion are staged
for after v1 works end-to-end on TradeLocker.

**Auth / account bootstrap** — done
- [x] `load_config` — reads `config.toml` into `HashMap<String, TLAccountState>`
- [x] `ensure_fresh_token` / `refresh_all_tokens` — per-account login + refresh
- [x] `fetch_account_info`, `fetch_instrument_info` — pull balance + instrument list per account

**Historical data pipeline** — done
- [x] `fetch_order_history` + `fetch_config` — pull raw order rows + column config
- [x] `zip_orders` — pairs positional array rows against named columns
- [x] `group_position_id` — groups zipped rows by `positionId`

**Risk engine** — not started
- [x] `calculate_pct_to_unit_size` (or lot size equivalent) — turn
      `(balance, risk_pct, entry_price, stop_loss)` into a qty per account
- [-] Wire the result into `NewOrder.qty` (currently unset/hardcoded in test calls)

**Execution fan-out** — not started ← **you are here**
- [ ] Given one `OrderIntent`, iterate all TradeLocker accounts concurrently
      (`tokio::spawn` per account)
- [x] Resolve `tradable_instrument_id` / `route_id` per account
      (`find_route_id_and_instrument_id` exists — needs to be called in the dispatch path)
- [x] Pull live entry price via `/trade/quotes` (`get_current_prices` exists but
      doesn't parse the response yet — still printing raw text)
- [x] Call `place_new_order` per account, collect results (success + failure) back
      on the main task
- [ ] Decide: `join_all` vs `JoinSet` for handling a hung/slow broker connection
      without blocking the others

**Persistence** — not started
- [ ] Schema exists (per your notes) but no Rust code writes to it yet
- [ ] Map fan-out execution results (`NewOrder` + broker response) into your
      `trades` / `child_executions` table
- [ ] `ON CONFLICT ... DO UPDATE` upsert path, `lastModified` as watermark

**Oanda integration** — not started
- [ ] Everything above, ported once the TradeLocker path is proven

## 3. Installation

```bash
git clone <this-repo>
cd oms
cp config.example.toml config.toml   # fill in per-account TL credentials
cargo build
```

`config.toml` format (one `[account_name]` block per TradeLocker account):

```toml
[account_1]
tl_url = "https://demo.tradelocker.com"
tl_email = "you@example.com"
tl_password = "..."
tl_server = "..."
tl_account_id = "..."
```

Requires a running local PostgreSQL instance once the persistence layer is wired up
(not required to run v1 execution-only).

## 4. Pipeline

Current (historical data, working):

```
load_config → refresh_all_tokens → get_all_account_info
            → get_all_order_history_info → get_configuration
            → zip_all_order_history → group_position_id
```

Target (v1 execution fan-out, in progress):

```
              OrderIntent (single input)
                      │
        ┌─────────────┼──────────────┐
        ▼              ▼              ▼
  Account 1        Account 2   ...  Account N
  risk calc        risk calc        risk calc
  qty from bal      qty from bal    qty from bal
        │              │              │
  place_new_order  place_new_order  place_new_order
        │              │              │
        └──────────────┴──────────────┘
                      ▼
              collect results
                      ▼
           write to PostgreSQL (not started)
```

## 5. Data Sources / Brokers

| Broker      | Protocol                  | Status                                      |
|-------------|----------------------------|----------------------------------------------|
| TradeLocker | JSON REST + Bearer token   |    Fan-out dispatcher in progress. |
| Oanda       | REST v20 JSON / WebSocket  | Not started                                   |
| MatchTrader | —                          | Not started                                   |
| DXTrade     | —                          | Not started                                   |
| MetaTrader 5| FFI (`extern "C"`)         | Not started                                   |

---

*This README tracks v1 scope only. Multi-broker adapter trait (`BrokerAdapter`), FIX/FFI
adapters, and the full parent/child relational execution ledger are out of scope until
the TradeLocker fan-out + DB write path is proven end-to-end.*