## 1. RELATIONAL ARCHITECTURE MAPPING

| Component / Layer | Primary Struct / Trait | Driven By (Functions / Methods) | Concurrency / Standard Library Tool |
| :--- | :--- | :--- | :--- |
| **Ingestion & Validation** | `OrderIntent` (Enum) | `parse_raw_payload()` | `std::io::BufReader` & `std::io::Read` |
| **Risk Engine** | `RiskCalculator` (Struct) | `calculate_lot_size()` | Pure mathematical functions (Deterministic) |
| **Concurrency Dispatcher** | `ExecutionDispatcher` (Struct) | `dispatch_parallel()` | `std::thread::spawn` & `std::sync::mpsc::channel` |
| **FIX Protocol Adapter** | `FixAdapter` (Struct implementing `BrokerAdapter`) | `send_fix_order()` | `std::net::TcpStream` |
| **MT5 FFI Adapter** | `Mt5Adapter` (Struct implementing `BrokerAdapter`) | `call_terminal_ffi()` | `std::ffi::CString` & `std::os::raw` |
| **Persistence Layer** | `PostgresJournal` (Struct) | `execute_transactional_write()` | `std::ffi::CString` linking to `libpq` via FFI |
| **State Monitor** | `SystemState` (Struct) | `aggregate_pnl()` | `std::sync::Arc<std::sync::Mutex<T>>` |

---

## 2. DATA STRUCTURES (STRUCTS & ENUMS)

### Requirements
- Define the core domain models, execution payloads, and state representations.
- You must use strict memory-efficient types (e.g., fixed-size arrays for strings where possible, or explicit `String` where dynamic allocation is unavoidable).
- All floating-point values for currency and pricing must be handled via integer representations of micro-units (e.g., `u64` representing ten-millionths of a unit, $10^{-7}$, to prevent IEEE-754 float rounding errors) or explicitly documented scaling factors.

### Official Reference
- [[Rust Book: Structs](https://doc.rust-lang.org/stable/book/ch05-00-structs.html)](https://doc.rust-lang.org/stable/book/ch05-00-structs.html)
- [[Rust Book: Enums](https://doc.rust-lang.org/stable/book/ch06-00-enums.html)](https://doc.rust-lang.org/stable/book/ch06-00-enums.html)

### Handwritten Challenges

1. **`OrderDirection`**: Write an enum defining the binary state of a trade. It must have two variants: `Buy` and `Sell`. Add a helper method to this enum that returns its representation as a single byte (`b'B'` or `b'S'`) for binary protocol serialization.
   - *Challenge*: Write the complete enum definition and its `impl` block containing the serialization method by hand.

2. **`OrderIntent`**: Write an enum that encapsulates the two mutually exclusive inbound payloads: `Market` and `Pending`.
   - The `Market` variant must contain an anonymous struct with fields: `parent_id` (a 16-byte array for UUID), `symbol` (a fixed-size byte array `[u8; 8]`), `direction` (`OrderDirection`), `stop_loss` (`u64` micro-units), `take_profit` (`u64` micro-units), and `risk_percent` (`u32` basis points, where `100` equals 1.00%).
   - The `Pending` variant must contain all fields of the `Market` variant, plus an additional `entry_price` (`u64` micro-units) field.
   - *Challenge*: Hand-write this nested enum structure, ensuring no helper crates are used for UUIDs or strings.

3. **`AccountConfig`**: Write a struct to hold the state of an individual target account. It must contain fields for `account_id` (a fixed-size byte array `[u8; 16]`), `balance` (`u64` micro-units), and `platform` (an enum `BrokerPlatform` with variants: `Oanda`, `TradeLocker`, `StoneX`, `Mt5`).
   - *Challenge*: Write the `AccountConfig` struct definition and the `BrokerPlatform` enum by hand.

4. **`ChildExecution`**: Write a struct representing the child order sent to a specific broker. It must contain the `parent_id` (`[u8; 16]`), a unique `child_id` (`[u8; 16]`), the `account_id` (`[u8; 16]`), the calculated `lot_size` (`u64` representing micro-lots), and an execution status enum `ExecutionStatus` (variants: `Pending`, `Filled`, `Rejected`, `Closed`).
   - *Challenge*: Write the `ChildExecution` and `ExecutionStatus` structures.

---

## 3. ABSTRACTION & COUPLING (TRAITS)

### Requirements
- Establish a uniform interface for outbound broker adapters. This isolates the concurrency dispatcher from the underlying network protocols (FIX, REST, FFI).
- Implementations must return a standard `Result` type containing either a successful execution confirmation or a concrete error structure.

### Official Reference
- [[Rust Book: Traits](https://doc.rust-lang.org/stable/book/ch10-02-traits.html)](https://doc.rust-lang.org/stable/book/ch10-02-traits.html)

### Handwritten Challenges

1. **`BrokerAdapter` Trait**: Define a trait named `BrokerAdapter`. It must expose two methods:
   - `execute_order(&self, account: &AccountConfig, child: &ChildExecution) -> Result<ChildExecution, AdapterError>;`
   - `query_status(&self, child_id: &[u8; 16]) -> Result<ExecutionStatus, AdapterError>;`
   - *Challenge*: Write the complete trait definition by hand. Include the definition of a custom `AdapterError` struct containing an error code (`u32`) and a fixed-size error message buffer (`[u8; 64]`).

2. **Trait Implementation for `StoneX` (FIX)**: Write the skeleton of the `impl BrokerAdapter for FixAdapter` block.
   - The `FixAdapter` struct must contain a `std::net::TcpStream` wrapped in an option-like structure or initialized state.
   - *Challenge*: Write the entire `impl BrokerAdapter for FixAdapter` signature block, including empty placeholder bodies for both methods, ensuring the type signatures match the trait definition exactly.

>Good. Your reasoning is correct — `Option` models the "not yet connected or disconnected" state without panicking.
>
>## The Struct
>
>Two things you need to figure out before you write it:
>
>**1. What fields does it need?**
>
>The assignment tells you exactly one required field. But a real FIX session also needs to identify _who_ is connecting. From the TT manual, a FIX Logon (Tag 35=A) requires a `SenderCompID` and `TargetCompID` to establish the session.
>
>So ask yourself: should those identifiers live on the struct, or be passed in at call time? What makes more sense for a persistent TCP session?
>
>**2. What type wraps the stream?**
>
>You said `Option<TcpStream>`. That's it. Write it.
>
>→ [`std::net::TcpStream`](https://doc.rust-lang.org/std/net/struct.TcpStream.html)
>
>---
>
>## The Shape
>
>
>```
>FixAdapter
>├── stream: Option<TcpStream>        ← required by the assignment
>├── sender_comp_id: [u8; N]          ← who am I on the FIX session?
>└── target_comp_id: [u8; N]          ← who am I talking to?
>```
>
>Fixed-size byte arrays for the IDs — consistent with the memory discipline from Section 2. Pick a reasonable size for a FIX comp ID (max 64 chars in the spec).
>
>---
>
>Write the struct definition. No `impl` yet, just the `struct` block. What do you have?

---

## 4. BEHAVIOR AND CONTROL FLOW (FUNCTIONS & METHODS)

### Requirements
- Implement the deterministic risk calculation matrix without using floating-point types.
- Enforce strict error boundaries using `Result` to propagate failures up to the database journal.

### Official Reference
- [[Rust Book: Defining Methods](https://doc.rust-lang.org/stable/book/ch05-03-method-syntax.html)](https://doc.rust-lang.org/stable/book/ch05-03-method-syntax.html)
- [[Rust Book: Error Handling](https://doc.rust-lang.org/stable/book/ch09-00-error-handling.html)](https://doc.rust-lang.org/stable/book/ch09-00-error-handling.html)

### Handwritten Challenges

1. **`calculate_lot_size` Method**: Write a method associated with a `RiskCalculator` struct. The function signature must be:
   `pub fn calculate_lot_size(balance: u64, risk_basis_points: u32, entry_price: u64, stop_loss: u64, contract_unit_value: u64) -> Result<u64, RiskError>`
   - **Calculation Logic**:
     1. Calculate the absolute price delta: $\lvert \text{entry\_price} - \text{stop\_loss} \rvert$. If the delta is zero, return `Err(RiskError::InvalidStopLoss)`.
     2. Calculate the cash-at-risk: $(\text{balance} \times \text{risk\_basis\_points}) / 10000$.
     3. Calculate the raw lot size: $\text{cash\_at\_risk} / (\text{price\_delta} \times \text{contract\_unit\_value})$.
     4. If any overflow or division-by-zero risk occurs, you must catch it using safe integer methods (e.g., `checked_sub`, `checked_mul`, `checked_div`).
   - *Challenge*: Write the complete body of `calculate_lot_size` by hand, using only standard library integer methods and explicit error propagation.

2. **FFI Transaction Journaling**: Write a function `journal_transaction` that executes an SQL command using raw FFI calls to the PostgreSQL client library (`libpq`).
   - The function must accept raw pointers to a database connection and a SQL command string.
   - You must handle the conversion of Rust strings to null-terminated C-strings using `std::ffi::CString`.
   - *Challenge*: Write the function signature and body:
     `unsafe fn journal_transaction(conn: *mut std::os::raw::c_void, query: &str) -> Result<(), JournalError>`
     Ensure you explicitly handle null pointer checks and error status checks returned by the foreign functions.

---

## 5. MEMORY MANAGEMENT, OWNERSHIP, AND LIFETIMES

### Requirements
- Maximize performance by passing read-only configuration pools and order payloads by reference.
- Prevent data duplication by enforcing lifetime annotations where structures hold references to shared configurations.

### Official Reference
- [[Rust Book: Understanding Ownership](https://doc.rust-lang.org/stable/book/ch04-00-understanding-ownership.html)](https://doc.rust-lang.org/stable/book/ch04-00-understanding-ownership.html)
- [[Rust Book: Validating References with Lifetimes](https://doc.rust-lang.org/stable/book/ch10-03-lifetime-syntax.html)](https://doc.rust-lang.org/stable/book/ch10-03-lifetime-syntax.html)

### Handwritten Challenges

1. **`ExecutionTask` Lifetime Mapping**: Create a struct named `ExecutionTask` that binds a reference to an `AccountConfig` and a reference to an `OrderIntent` together for processing. Because these references are borrowed from the main coordinator memory, you must use explicit lifetimes.
   - *Challenge*: Write the struct definition:
     `pub struct ExecutionTask<'a, 'b> { ... }`
     Ensure that the `AccountConfig` reference uses lifetime `'a` and the `OrderIntent` reference uses lifetime `'b`. Explain in a comment above your code how the compiler uses these lifetimes to prevent a use-after-free bug if the target account configuration is dropped.

2. **Borrowing vs. Cloning in Fan-Out**: Write a function signature for a dispatcher that iterates over a slice of `AccountConfig` references and spawns execution tasks.
   - *Challenge*: Write the function signature of `prepare_dispatch` which accepts `accounts: &[AccountConfig]` and `intent: &OrderIntent`, returning a vector of `ExecutionTask` structs mapping to those lifetimes. Do not clone any inner fields of the structures.

---

## 6. CONCURRENCY, PARALLELISM, AND STATE DISPATCH

### Requirements
- Implement a concurrent dispatch loop using only the Rust Standard Library.
- Spin up an isolated thread for each target account execution.
- Use multi-producer, single-consumer channels (`std::sync::mpsc`) to aggregate execution results back to a single monitoring thread.
- Protect the consolidated P&L and active position counts using a thread-safe mutex wrapper.

### Official Reference
- [[Rust Book: Fearless Concurrency](https://doc.rust-lang.org/stable/book/ch16-00-concurrency.html)](https://doc.rust-lang.org/stable/book/ch16-00-concurrency.html)
- [[Rust Standard Library: sync](https://doc.rust-lang.org/std/sync/)](https://doc.rust-lang.org/std/sync/)

### Handwritten Challenges

1. **The Parallel Dispatch Loop**: You must write the orchestration logic for the concurrent execution engine.
   - Given a vector of `AccountConfig` configurations and an validated `OrderIntent`, spawn a new OS thread for each account using `std::thread::spawn`.
   - Before spawning, instantiate a channel using `std::sync::mpsc::channel()`.
   - Pass the sender half (`Sender<ChildExecution>`) into each spawned thread.
   - Inside each thread, execute the risk calculations and call the respective `BrokerAdapter` implementation.
   - Send the resulting `ChildExecution` structure back through the channel.
   - *Challenge*: Write the complete `dispatch_and_collect` function by hand:
     `pub fn dispatch_and_collect(accounts: Vec<AccountConfig>, intent: OrderIntent) -> Vec<ChildExecution>`
     You must write the manual thread spawning loop, the moves of the channel senders, and the collection loop that blocks on the receiver until all threads have completed their work.

2. **Thread-Safe State Synchronization**: Write a monitoring loop that updates a global state struct `OmsState` as child executions are reported.
   - The `OmsState` struct holds a `total_allocated_volume: u64` and `active_trade_count: u32`.
   - This state is wrapped in a `std::sync::Arc<std::sync::Mutex<OmsState>>`.
   - *Challenge*: Write the handler block that receives a `ChildExecution` from a channel, locks the mutex safely, handles potential poison errors, and increments the state values safely. Use explicit scope blocks or manual drops to release the lock as quickly as possible.