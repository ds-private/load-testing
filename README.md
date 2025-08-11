Excellent. Let's begin Phase 1.

The primary goal of this phase is to build a Minimum Viable Product (MVP) that validates the most critical and uncertain part of the architecture: the integration between the high-concurrency **Tokio** engine and the **Rune** scripting environment. We will create a tool that can execute a simple load test defined in a Rune script and print a basic summary to the console.

Per your direction, we will use **Hyper** for the HTTP client from the start, which differs from the plan's suggestion to begin with `reqwest`. This allows us to build with fine-grained control from day one.

-----

### **1. Project Setup: The Cargo Workspace**

First, we will set up the `cargo workspace` as planned. This separates our concerns into focused, independent crates.

1.  Create the project directory:
    ```bash
    mkdir load_tester
    cd load_tester
    ```
2.  Create the workspace `Cargo.toml` file:
    ```bash
    touch Cargo.toml
    ```
    Add the following content to define the workspace members. For Phase 1, we only need `load_tester_cli`, `load_tester_core`, and `rune_api`:
    ```toml
    [workspace]
    members = [
        "crates/load_tester_cli",
        "crates/load_tester_core",
        "crates/rune_api",
    ]
    ```
3.  Create the individual crates:
    ```bash
    cargo new crates/load_tester_cli
    cargo new --lib crates/load_tester_core
    cargo new --lib crates/rune_api
    ```

-----

### **2. Phase 1 Implementation Steps**

Here is the step-by-step plan to build the MVP's features.

#### **Step 1: Build the Minimal CLI (`load_tester_cli`)**

  * **Crate:** `load_tester_cli`
  * **Library:** `clap`
  * **Goal:** Create a command-line interface that accepts a single argument: the path to a `.rn` script file.

In `crates/load_tester_cli/src/main.rs`, we will use **`clap`** with its "derive" feature to parse the arguments. The `main` function will be responsible for reading the script file content and passing it to the core engine to execute the test.

#### **Step 2: Implement the Core Engine (`load_tester_core`)**

  * **Crate:** `load_tester_core`
  * **Libraries:** `tokio`, `hyper`, `rune`
  * **Goal:** Orchestrate the test run based on the `constant-vus` executor logic.

The implementation will look like this:

1.  **Controller:** A main `run_test` function will serve as the controller. It will receive the script content from the CLI crate.
2.  **Rune Compilation:** It will use `rune::prepare` to compile the script into bytecode just once.
3.  **VU Spawning:** It will use **`tokio::spawn`** to create a fixed number of asynchronous tasks, representing our virtual users (VUs). This is the Rust equivalent of the goroutine-per-VU model.
4.  **Metrics Collection:** A **`tokio::sync::mpsc`** channel will be used for metrics. Each VU task will be a producer, sending its results (like request duration) to the channel. A single aggregator task will be the consumer.
5.  **Summary:** At the end of the test, the aggregator will calculate and print the minimal metrics to standard output: total requests (`http_reqs`) and request duration (`avg` and `max`).

#### **Step 3: Create the Rune Scripting API (`rune_api`)**

  * **Crate:** `rune_api`
  * **Libraries:** `rune`, `hyper`, `tokio`
  * **Goal:** Expose basic load-testing functions to the Rune scripts. This is the bridge between our Rust engine and the user's test logic.

We will create a Rust module that defines the functions available within Rune scripts:

  * **`http::get(url)`:** An `async` function exposed to Rune. Internally, this Rust function will use a shared **Hyper** client to execute an HTTP GET request. It will capture the response time.
  * **`sleep(seconds)`:** An `async` function that pauses the current VU's execution by calling **`tokio::time::sleep`**.
  * **Module Registration:** This entire API module will be registered with the `rune::Context` before the script is compiled, making the functions available to the user's script.

-----

### **3. Example Rune Script for Phase 1**

The user would write a script like the one below and run it with our tool (`./load_tester run ./scripts/simple_test.rn`).

```rust
// In scripts/simple_test.rn

// The http and sleep functions are provided by our Rust engine
use our_tool::{http, sleep};

pub async fn main() {
    // Make a request to a test API
    let res = http::get("https://api.test.com/items").await?;
    
    // Pause for 1 second to simulate user think time
    sleep(1.0).await?;
}
```

By completing these steps, we will have a functional, end-to-end skeleton of the application. This will prove the viability of the core architecture and provide a solid, de-risked foundation for building the more advanced features in Phase 2.
