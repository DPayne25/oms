---
title: adr-001-tradelocker-multi-account-execution
date:  2026-07-03
---

# adr-001-tradelocker-multi-account-execution

> **Context Summary:** While managing multiple accounts that operate on the TradeLocker platform, a trader is required navigate to each account to execute the same trade idea independently. This presents many discrepancies in the position across the accounts, especially as the number of managed accounts increases. Differences in: 
> 
>- execution time
>- risk size
>- management of idea
>- human input or lack of input
>
> are all experienced scenarios without implementation of trade locker multi-account execution.

---

## 1. Status
-   **Current State:**  `Accepted` 

## 2. Context
*This section describes the factual forces at play—technological constraints, systemic requirements, and performance limitations. Keep this language strictly value-neutral and fact-based.

- **Goal:** Take a single trade input and concurrently execute the same trade across all trade locker accounts with risk proportional to the account balance.
- **System Priority:** Valid API response and appropriate risk replication.

## 3. Decision
*Direct, actionable response to the forces described above. State this section in full, active-voice sentences. `⁙`  acts as the input requested from the user.*

- **I will** ensure a valid response is received after a hard coded `place_new_order()` request on a demo account.
- **I will** create a `struct` called `NewOrder` with the following fields:
	- ⁙ `trade_setup: TradeSetup` (matches to risk percentage)
	- `qty: Decimal` ( that derived from percentage of `TradeSetup`)
	- `route_id: i64`
	- ⁙ `side: OrderSide`
	- ⁙ `stop_loss: Decimal`
	- `stop_loss_type: String` (const "absolute" should comment out and hard code in executed function. Alts not accepted.)
	- ⁙ `take_profit: Decimal` 
	- `take_profit_type: String` (const "absolute" should comment out and hard code in executed function. Alts not accepted)
	- `tradable_insturment_id: i64`
	- `type: String` (const "market")
	- `validity: String` (const "IOC").
- **I will** establish a `trait` called `OrderExecutor` .
- **I will** implement a method for `NewOrder` with the `OrderExecutor` trait, called `place_new_order() -> `  with the expected input variables:
	- `&self`
	- `account: &TLAccountState`.
- **I will** create an `enum` for the `trade_setup` input called `TradeSetup` with all the setups specified in [[TradingPlan-v9.0#Setups]].
- **I will** implement a method on `TradeSetup` with `risk_percentage()` (`-> Decimal`) to match the input `enum` to the established risk percentage defined in my [[TradingPlan-v9.0]]:
	- `TradeSetup::{enum} => {risk_pct}`.
- **I will** create a trait called `RiskCalculator` with the following functions:
	- `calculate_pct_to_unit_size()`
	- `calculate_pct_to_lot_size()`.
- **I will** `impl RiskCalculator for TradeSetup` and use the output of `calculate_pct_to_unit_size()` to store into `NewOrder::qty`.
- **I will** create a method called `find_instrument_id` on the `TLAccountState` struct that identifies the value stored in `self.instrument_info.as_ref(name)` and store the output in `NewOrder::tradable_instrument_id`
- **I will** create a method called `find_route_id` on the `TLAccountState` struct that identifies the value stored in `self.instrument_info.as_ref(routes.id)` and store the output in `NewOrder::route_id`
- I will test in main the for loop execution on all accounts with with a `TradeSetup` enum called `TestSetup`.

### Scope Boundaries (The Execution Guardrails)
| In Scope (Strictly Allowed)                                                                             | Out of Scope (The Filler)                                                                                          |
| ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Structs to store the user input values.                                                                 | Structs that include fields of all possible input of an API request. No need to state items that will not be used. |
| Functions that contribute to the expected formatting required by the API request.                       | Functions that generate pair dependent risk calculations or consider cross pair leverage reduction.                |
| Outputs that clearly states what was stored in a field and provide proof that a request was successful. | Db ingestion                                                                                                       | 

## 4. Alternatives Considered
*A breakdown of serious alternatives evaluated, weighing their trade-offs against our first-principles mandates.*

-   **Alternative A: `stop_loss`/`take_profit` variations**
    -   *Pros:* Encompasses the full scope of the `stopLossType` and `takeProfitType` parameters from the API reference
    -   *Cons:* 
	    - Requires parsing the established input to decide the appropriate `stopLossType`/`takeProfitType`.
	    - Requires an additional input of a less intuitive nature.
	    - Introduces a requirement of mapping logic to future platform specific APIs on stop loss and take profit parameters. (Translating every stop loss type and take profit type to a uniform of the input. I prefer to just standardize the price and the input.)
-   **Alternative B:** `RiskCalculator` functions
    -   *Pros:* `...unit_size()` and `...lot_size()` where chosen due to the assumption that other brokerages or platforms that will be implemented in the future may require different input formats for the qty/size of a new order.
    -   *Cons:* Will need to figure bidirectional conversions to ensure one payload that gets inserted into db when that phase is initiated, or just write more functions for risk variant and call the appropriate function for consistent risk variant ingestion in db ingest phase.

## 5. Consequences
*The resulting context after applying the decision. Record all outcomes honestly—positive, negative, and neutral.*

### **Positive:** [What becomes simpler, faster, or safer].


### **Negative:** [What technical debt, limitation, or trade-off is accepted].


### **Neutral:** [Structural changes that don't directly hurt or help but must be known].

## 6. Acceptance Criteria
*The binary win condition that proves this specific decision has been successfully executed.*

- [ ] Print value of `NewOrder::qty` and manually check for correct value.
- [ ] Print value of `NewOrder::tradable_instrument_id` and manually check for correct stored value.
- [ ] Print value of `NewOrder::route_id` and manually check for correct stored value.
- [ ] 200 Request Code with expected output:
	```json
	{
  "d": {
    "orderId": "123"
  },
  "s": "ok"
}
	```
- [ ] The Tradelocker Desktop application reflects the intended order