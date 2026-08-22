# hw-oms-order_placement


---

## rest-oms-dispatcher-questionnaire date: 2026-07-08 context: Handwritten planning doc — REST/tokio track (post FIX pivot)

# Multi-Account Order Dispatch — Handwritten Planning Doc

You already have a working single-account `place_new_order`. Everything below exists to get you from "one account, hardcoded" to "N accounts, risk-sized, concurrent." No code is written for you — write pseudocode or real signatures in the blank blocks.

---

## 1. RELATIONAL ARCHITECTURE MAP

Fill in the blank column from what you already have in `tradelocker.rs` / `main.rs`. Where it's still empty, that's a section below you haven't built yet.

|Component / Layer|Struct / Trait|Driven by (fn)|Concurrency model|Status|
|---|---|---|---|---|
|Config ingestion|`TradeLockerConfig`|`load_config()`|none (sync)|done|
|Auth / token state|`TLAccountState`|`ensure_fresh_token()`|______________|open thread|
|Instrument resolution|`InstrumentInfo` / `Routes`|`find_instrument_id()`|n/a|mislabeled — see §2|
|Contract metadata|______________|______________|n/a|missing|
|Risk engine|`TradeSetup` + `RiskCalculator`|`calculate_pct_to_unit_size()`|pure math|partial — see §3|
|Order payload|`NewOrder`|(constructed manually today)|n/a|needs per-account builder|
|Execution|`NewOrder` impl `OrderExecutor`|`place_new_order()`|single-account only|needs fan-out — see §4|
|Dispatcher|______________|______________|______________|not started|
|Result aggregation|______________|______________|______________|not started|

---

## 2. NEW ORDER — FIELD BY FIELD INPUT MAP

This is the actual pipeline: a human picks a symbol from a dropdown, a direction, a stop/take, and a `TradeSetup`. Everything else in `NewOrder` has to be derived. Fill in the "Function responsible" column yourself — some already exist, some don't.

|`NewOrder` field|Source|Function responsible|Status|
|---|---|---|---|
|`qty`|derived (risk calc, §3)|`calculate_pct_to_lot_size()`|doesn't exist yet|
|`route_id`|derived (lookup)|______________|`find_instrument_id` returns this today, but is misnamed|
|`side`|direct user input|n/a (maps `OrderSide` enum)|done|
|`stop_loss`|direct user input|n/a|done|
|`stop_loss_type`|constant `"absolute"`|n/a (ADR already decided this)|done|
|`take_profit`|direct user input|n/a|done|
|`take_profit_type`|constant `"absolute"`|n/a|done|
|`tradable_instrument_id`|derived (lookup)|______________|exists as a field on `InstrumentInfo` already — not yet returned by any function|
|`kind` (`type`)|constant `"market"`|n/a|done|
|`validity`|constant `"IOC"`|n/a|done|

Notice the pattern: **6 of 10 fields are already constants or direct input** — the only real engineering left is `qty`, `route_id`, and `tradable_instrument_id`, plus the symbol-normalization step that feeds the lookup (§2b) and the lot-size math that feeds `qty` (§3).

**Handwritten challenge:** Given the table above, write two function signatures (not bodies) — one that takes a clean instrument name and an account, and returns _both_ `route_id` and `tradable_instrument_id` together. What return type avoids calling the same `.find()` over the instrument list twice?

---

## 2a. INSTRUMENT & CONTRACT METADATA RESOLUTION

**What's true right now:** `find_instrument_id(&self, target_name: &str) -> Result<i64, ...>` finds an `InstrumentInfo` by name, then returns `route.id` — that's a **route id**, not the `tradable_instrument_id` field that already sits directly on `InstrumentInfo`.

**Confirmed from the docs:** the order endpoint's mandatory fields are exactly `qty`, `routeId`, `side`, `validity`, `type`, `tradableInstrumentId` — so your struct shape is already correct, you just need both IDs resolved from one lookup instead of one.

**Questions to answer on paper:**

1. Does `NewOrder` need one function that returns both IDs, or two separate lookups? What does that change about the return type — a tuple, or a small struct?
2. `Routes` currently only deserializes `id` and `kind`. The per-instrument contract size field is called `lotSize` (confirmed — see §3, it's _not_ a fixed 100,000, it varies per instrument and per broker). Where does `lotSize` actually live in the `/trade/accounts/{accountId}/instruments` response you already fetch — on the top-level `InstrumentInfo`, or nested inside each `Routes` entry? Check your own raw JSON (log the response body once) rather than guessing.
    - `lotSize` location: ______________________
    - minimum lot step/increment key (if any): ______________________
3. Is there a separate `/trade/instruments/{tradableInstrumentId}` singular endpoint that returns _more_ detail than the list endpoint already gives you, or is everything you need already sitting in the array you fetch today? Don't add a new API call if the field's already in `self.instruments`.

**Handwritten challenge:** Rename/split `find_instrument_id` into a lookup that returns instrument id + route id + lot size together, and write the struct field(s) you'd add to `InstrumentInfo` or `Routes` to hold `lotSize` — field name, type, which struct.

**Reference:** [TradeLocker instruments endpoint](https://public-api.tradelocker.com/reference/getinstruments), [serde field renaming](https://serde.rs/field-attrs.html)

---

## 2b. SYMBOL NORMALIZATION (dropdown value → broker's actual instrument name)

**The problem you're describing:** your user-facing dropdown shows a clean symbol ("GBPUSD"), but the same instrument may be listed by different brokers/routes with suffixes (e.g. `GBPUSD.qrt`, `GBPUSDraw`) or with no suffix at all. There's no single industry term for this — most people just call it a **symbol alias map** or **instrument alias table**: a lookup that translates one canonical symbol into whatever string each broker actually uses.

**Questions to answer on paper:**

1. Is the suffix pattern _predictable per broker_ (e.g. "this broker always appends `.qrt`"), or does it vary per instrument within the same broker? This determines whether you need a simple string-strip/suffix-match, or a hardcoded per-broker alias table.
2. Where should that mapping live — a `HashMap<String, String>` per `TLAccountState` built once at startup from the fetched `instruments` list (e.g. by stripping known suffixes and matching the base), or a static config table you maintain by hand per broker? Which one survives a broker adding a new instrument without you touching code?
3. Should the match be exact-after-strip, or does it need fuzzy matching (`starts_with`, `contains`)? What happens if two instruments in the same account both start with "GBPUSD" (e.g. a spot pair and a CFD variant) — how do you disambiguate?

**Handwritten challenge:** Write the function signature only — `fn resolve_canonical_symbol(&self, dropdown_symbol: &str) -> Result<&InstrumentInfo, ____>` — and pseudocode the matching strategy you picked from Q3. No implementation, just the decision tree as comments.

**Reference:** [Rust `str` methods: `strip_suffix`, `starts_with`](https://doc.rust-lang.org/std/primitive.str.html)

---

## 3. RISK ENGINE — PRICE DELTA → LOT SIZE

**What's true right now:** `calculate_pct_to_unit_size(&self, balance: Decimal) -> Decimal` only computes `balance * risk_percentage()` — that's **cash-at-risk**, not a lot size. Your own ADR names the next function `calculate_pct_to_lot_size()` and never wrote it.

**Resolved — the units-vs-lots confusion you ran into:** TradeLocker's docs are inconsistent in casual language, but the actual API is not ambiguous. The order endpoint's `qty` field means **number of lots** (confirmed from the close-position docs, which describe it as "the number of lots you want to close"). Separately, `lotSize` is a per-instrument field describing how many raw units one lot equals — TradeLocker's own materials confirm the "1 lot = 100,000 units" convention is a **forex-only default**, not universal; other instrument types define their own contract size. So the real conversion chain has one more step than "cash at risk over price delta":

```
cash_at_risk (money)
  → ÷ price_delta               = raw unit exposure the position needs
  → ÷ lotSize (per instrument)  = qty in lots
  → round to broker's minimum lot step
```

**Questions to answer on paper:**

1. Where does `lotSize` come from for a given trade — is it a fixed property of the instrument (pulled once from §2a), or can it differ by route/account for the same symbol? Check this against your own fetched data before assuming it's constant.
2. For a market order, where does "entry price" (used in the price-delta calc) come from? You don't have a `/trade/quotes` call yet — is that a blocking prerequisite to this function, or can the function accept entry price as a plain parameter and let the caller fetch it separately?
3. Rounding to lot step: `rust_decimal::Decimal` — which method rounds _down_ to the nearest multiple of an arbitrary step (not just N decimal places)? Look at `Decimal::round_dp` vs a manual `(qty / step).floor() * step` — which is actually correct here, and why does naive division-then-floor risk a subtle bug if `step` isn't a power of ten?
4. Should `calculate_pct_to_lot_size` take `lotSize` as one of its parameters (pure function, caller resolves it first), or should it reach into `TLAccountState`/ `InstrumentInfo` itself? Which keeps it testable without a live account?

**Handwritten challenge:** Write the full signature (not body) for `calculate_pct_to_lot_size`, matching the return-`Result` discipline the ADR already established, and including every input from the conversion chain above. What's your `Err` variant when `price_delta == 0` — and what's your `Err` variant when `lotSize` is somehow zero or missing?

**Reference:** [rust_decimal docs](https://docs.rs/rust_decimal/latest/rust_decimal/), [Rust Book: Error Handling](https://doc.rust-lang.org/stable/book/ch09-00-error-handling.html)

---

## 4. CONCURRENT DISPATCHER — FOR LOOP → `tokio::spawn` FAN-OUT

This is the core of what you asked about. Your `main.rs` today iterates `accounts.iter_mut()` **sequentially** with `.await` on each account, one at a time — functionally correct, but if one account's request hangs, everything behind it stalls. That's the exact problem the ADR/OMS description says a real dispatcher must avoid.

**Questions to answer on paper:**

1. `accounts: HashMap<String, TLAccountState>` — if you spawn one `tokio::task` per account, what does each spawned future need to _own_ (not borrow) to satisfy `tokio::spawn`'s `'static` bound? Walk through why `&mut TLAccountState` from a `HashMap` iterator can't be moved into a spawned task.
2. Given that, is the right move to drain the `HashMap` into owned `TLAccountState` values before spawning (e.g. `HashMap::into_iter()`), or wrap the whole map in `Arc<Mutex<...>>` and lock per-account inside each task? Write one sentence for each option on what it costs you (mutability, lock contention, code shape).
3. `ensure_fresh_token(&mut self, ...)` — if pre-flight token refresh happens once in the main task _before_ spawning, do the spawned workers still need `&mut self`, or can `place_new_order` take `&TLAccountState`? Decide this before you touch the dispatcher — it determines whether your fan-out loop needs mutable access at all.
4. Collecting results: `futures::future::join_all(handles).await` vs `tokio::task::JoinSet`. Which one lets you react to the _first_ completed account instead of waiting for all N in whatever order they were spawned? Given you have up to 15 accounts and care about one broker hanging, which fits better?
5. What does a single "task result" need to carry back to the caller? At minimum: account name, and a `Result<serde_json::Value, Box<dyn Error>>` — do you want anything else (elapsed time? the `NewOrder` that was sent, for the audit trail)?

**Handwritten challenge:** Write the function signature only — `pub async fn dispatch_orders(accounts: HashMap<String, TLAccountState>, setup: TradeSetup, client: Client) -> Vec<____>` — fill in the blank return type based on your answer to Q5. Then write the `tokio::spawn` loop as pseudocode (real Rust keywords, no bodies): what gets moved into the closure, what gets pushed into what collection, what you `.await` at the end.

**Reference:** [Tokio tasks](https://tokio.rs/tokio/tutorial/spawning), [JoinSet docs](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html), [Rust Book: Fearless Concurrency](https://doc.rust-lang.org/stable/book/ch16-00-concurrency.html)

---

## 5. PER-ACCOUNT ORDER PAYLOAD ASSEMBLY

**Questions to answer on paper:**

1. Inside each spawned task, what's the exact sequence before you can construct a `NewOrder`? List it in order (token check → instrument/route lookup → entry price → lot size → build struct → send). Where does §2's metadata and §3's math each plug in?
2. `TradeSetup` currently only supplies a risk percentage. Does the symbol/direction/ stop_loss/take_profit for a given trade idea belong on `TradeSetup`, or on a new struct that represents "one trade intent" independent of which setup triggered it? (Compare this to how the OMS description separates `OrderIntent` from account config.)

**Handwritten challenge:** Sketch the struct you're missing — call it whatever you want (`TradeIntent`? `OrderRequest`?) — that holds the human-supplied trade idea _before_ it's fanned out per-account. List fields only.

---

## 6. ERROR AGGREGATION & PARTIAL FAILURE

Per the OMS description: one broker failing mid-broadcast must not invalidate fills on the others, and must be logged relationally against its account ID.

**Questions to answer on paper:**

1. Your current per-account `for` loops in `main.rs` already do this informally with `if let Err(e) = ... { println!(...) }`. What's the minimum change needed to turn those printed errors into something you could later insert into a `child_executions` row instead of just logging to stdout?
2. Should a single failed account abort the whole `dispatch_orders` call (returning `Result<Vec<_>, _>`), or should the function itself never fail — always returning `Vec<Result<_, _>>` — pushing the decision of "is this bad enough to stop" up to the caller? Which matches the "no invalidating parallel fills" mandate?

---

## 7. ON THE HORIZON (don't solve yet, just note)

- `/trade/quotes` call for live entry price — blocks §3 Q2 above
- Wiring `dispatch_orders` output into a `child_executions` Postgres insert
- Deciding whether `ensure_fresh_token`'s pre-flight-vs-per-worker answer (§4 Q3) should also change how `main.rs`'s existing sequential fetch loops work

---

### Before you leave the desk

Bring back: the two JSON key names from §2 Q2, and a one-sentence answer to §4 Q2 (owned-drain vs `Arc<Mutex<_>>`). Those two decisions gate almost everything else here.