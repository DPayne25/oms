---
title: OMS v1 — Revised Project Description
date: 2026-07-11
method: solve the maze backwards
---

# OMS v1 — Revised Project Description

> **Solve the maze backwards.** This document is deliberately structured end-first: the intended output is stated up front, then every section below it exists only because it's a prerequisite for that output. If a piece of work doesn't trace back to Section 1, it doesn't belong in v1 — it's a fork in the maze you're not taking yet.

---

## 1. The End Target

A trader (you) opens a web page, fills in an order intent (instrument, side, stop loss, take profit — market order only), submits it, and within seconds every account defined in `config.toml` has either a filled position or a logged per-account failure — sized correctly for that account's balance, sent concurrently rather than one-at-a-time. From that same interface, you can see the floating (unrealized) balance aggregated across all accounts, and you can modify an open position without going account-by-account manually.

That's it. That's v1. Everything below exists to make that sentence true.

---

## 2. Working Backward: What Has to Be True

Starting from the end target and asking "what does this require?" at each step:

**To see aggregate floating balance across accounts**
← requires per-account live balance/PnL polling or push
← requires the account already having an open position and a fresh token
← requires the web UI to have a display surface, not just an input form

**To edit an open position**
← requires knowing which position (broker-assigned ticket/position ID) belongs to which account
← requires a broker adapter method for modify, not just create
← requires the original order's route/instrument context to still be resolvable

**To get a filled position on every account concurrently**
← requires a broker adapter per platform (TradeLocker done; OANDA, MT5 not started)
← requires a fan-out dispatcher (currently you loop sequentially in `main.rs`; the concurrent version — `tokio::spawn` per account — is not yet built)
← requires per-account position sizing (risk % → lot size), already implemented for TradeLocker via `calculate_lot_size`
← requires a fresh auth token per account before dispatch (`ensure_all_fresh` — done)
← requires the account's tradable instrument ID and route ID resolved for the requested symbol (`find_route_id_and_instrument_id` — done)

**To submit an order intent from a web page**
← requires the web UI to serialize into something that matches your `OrderIntent` shape (instrument, setup, side, stop_loss, take_profit)
← requires a local HTTP endpoint for the UI to POST to — **this doesn't exist yet**. Right now `OrderIntent` is only ever constructed in Rust (`main.rs` hardcodes a `NewOrder` directly). There is no server process listening for the web UI's submission.

This last one is the actual gap between where you are and the end target. Everything else on this list has either been built already or is a known, scoped piece of work (OANDA adapter, MT5 adapter, fan-out dispatcher). The web UI's backend — something that accepts a POST and hands it to the dispatcher — does not exist in the current codebase at all.

---

## 3. Scope Boundaries for v1

| In Scope | Out of Scope (v1) |
|---|---|
| Market orders only | Pending/limit orders |
| Web UI → order intent → concurrent dispatch across all configured accounts | Sequential/manual per-account execution (current `main.rs` behavior is a scaffold, not the target) |
| Position sizing from risk % (already implemented for TradeLocker) | Cross-pair leverage adjustments, correlation-aware sizing |
| Edit an open position (SL/TP modification at minimum) | Full order-type editing, partial closes (unless trivial to include) |
| Aggregate floating balance display across accounts | Per-trade PnL attribution, historical analytics |
| Broker adapters: TradeLocker, OANDA, MT5 (home server) | StoneX/FIX, DXTrade — explicitly deferred |
| In-memory account state (`HashMap<String, TLAccountState>` pattern, one struct type per broker or a shared trait) | PostgreSQL persistence — explicitly deferred to v2 |
| Config-driven account list (`config.toml`) | Dynamic account onboarding through the UI |

**Explicitly deferred to v2 (next phase):** PostgreSQL ingestion of execution results, the `parent_orders`/`child_executions` schema, the post-trade behavioral reflection gate, and Paralexia journal integration. None of these are prerequisites for the v1 end target — they become relevant once you need to persist and analyze what v1 already executed.

---

## 4. Architecture Sketch (Backward-Derived)

```
[ Web UI: Order Intent Form ]
        │  POST order intent (instrument, side, SL, TP)
        ▼
[ Local HTTP server / endpoint ]   ← does not exist yet — the actual gap
        │  deserialize → OrderIntent
        ▼
[ Concurrent Dispatcher ]           ← sequential loop exists; tokio::spawn fan-out does not
        │  per account, concurrently:
        ├──► [ TradeLocker Adapter ]   ← implemented (auth, sizing, place_new_order confirmed live)
        ├──► [ OANDA Adapter ]         ← not started
        └──► [ MT5 Home Server Adapter ] ← not started
        │
        ▼
[ In-memory account/position state ]
        │  polled or pushed
        ▼
[ Web UI: Aggregate Balance / Position Edit ]
```

Two things worth naming honestly here, since they're the actual unresolved questions blocking the architecture, not just missing code:

1. **The HTTP layer is a new decision, not an extension.** Everything you've built so far runs as a CLI binary (`main.rs` calling into `oms::tradelocker`). Wiring a web UI in means introducing a server (axum/actix are the usual choices) that owns the shared account state across requests — which is a different ownership problem than a one-shot CLI run. Worth treating as its own design question before writing any handler code.
2. **Balance monitoring is polling vs. push, again — this time across three brokers, not one.** You already flagged TradeLocker's WebSocket-vs-polling question as unresolved. For v1's aggregate balance view, you need an answer (even a "poll every N seconds" placeholder) for all three brokers, since OANDA and MT5 have different native mechanisms (OANDA: WebSocket streaming; MT5: whatever the home server terminal exposes over FFI or a local bridge).

---

## 5. Acceptance Criteria (v1)

- [ ] Web UI submits an order intent and it is received by a running Rust process (not hardcoded in `main.rs`)
- [ ] Order is dispatched concurrently (not sequentially) to every account in `config.toml`
- [ ] Each account's lot size is calculated from its own balance and the requested risk %
- [ ] A single failed account does not block or fail the others (per-account error isolation — already the pattern in `ensure_all_fresh` / `get_all_account_info`, needs to hold under concurrency too)
- [ ] At least one non-TradeLocker broker (OANDA or MT5) successfully places a live order through the same dispatch path
- [ ] The web UI displays a single aggregate floating balance figure across all accounts
- [ ] An open position can be modified (at minimum: SL/TP) from the web UI without touching each broker terminal manually

---

## 6. What This Description Deliberately Leaves Out

Per "solve the maze backwards" — these are real, correct pieces of the eventual system, but none of them are prerequisites for the v1 end target, so they're named here only to be excluded, not planned:

- Database schema design beyond what's needed to hold config
- The behavioral reflection gate
- Power BI / execution analysis
- Additional broker platforms (StoneX/FIX, DXTrade)
- Parrallax