use crate::rune_api;
use futures::future::join_all;
use rune::{
    Diagnostics, Source, Sources, Vm,
    termcolor::{ColorChoice, StandardStream},
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{sync::mpsc, task::LocalSet};

/// A simple structure to hold the results from a single VU iteration.
#[derive(Debug)]
pub struct Metric {
    pub request_duration: Duration,
}

/// Configuration for the test run, provided by the CLI layer.
pub struct TestConfig {
    pub script: String,
    pub vus: u32,
    pub duration: Duration,
}

pub async fn run_test(config: TestConfig) -> anyhow::Result<()> {
    // 1. Create the channel.
    let (tx, mut rx) = mpsc::channel::<Metric>(1000);

    // 2. Spawn the aggregator as an independent background task.
    let aggregator_handle = tokio::spawn(async move {
        let mut total_requests = 0u64;
        let mut total_duration = Duration::ZERO;
        let mut max_duration = Duration::ZERO;

        while let Some(metric) = rx.recv().await {
            total_requests += 1;
            total_duration += metric.request_duration;
            if metric.request_duration > max_duration {
                max_duration = metric.request_duration;
            }
        }

        println!("\n--- Test Finished ---");
        println!("Total Requests: {}", total_requests);
        if total_requests > 0 {
            let avg = total_duration / total_requests as u32;
            println!("Average Duration: {:?}", avg);
            println!("Max Duration:     {:?}", max_duration);
        }
        println!("---------------------\n");
    });

    // 3. Create the LocalSet to run the VUs.
    let local = LocalSet::new();

    // 4. Run the LocalSet. All Rune setup and VU spawning happens inside.
    local
        .run_until(async move {
            // --- RUNE SETUP MOVED INSIDE ---
            let ctx = Arc::new(rune_api::our_tool().expect("Failed to build rune context"));
            let runtime = Arc::new(ctx.runtime().expect("Failed to get rune runtime"));

            let mut sources = Sources::new();
            sources
                .insert(Source::new("script", &config.script).expect("Failed to load script"))
                .unwrap();

            let mut diagnostics = Diagnostics::new();
            let unit = Arc::new(
                rune::prepare(&mut sources)
                    .with_context(&ctx)
                    .with_diagnostics(&mut diagnostics)
                    .build()
                    .expect("Failed to build rune sources"),
            );

            if !diagnostics.is_empty() {
                let mut writer = StandardStream::stderr(ColorChoice::Always);
                diagnostics.emit(&mut writer, &sources).unwrap();
                // Optionally panic or exit here if there are build errors.
            }
            // --- END OF MOVED SETUP ---

            let mut handles = Vec::new();

            for i in 0..config.vus {
                let unit = unit.clone();
                let runtime = runtime.clone();
                let tx = tx.clone();
                let duration = config.duration;

                let handle = tokio::task::spawn_local(async move {
                    println!("Spawning VU {}", i + 1);
                    let mut vm = Vm::new(runtime, unit);
                    let test_end = Instant::now() + duration;

                    while Instant::now() < test_end {
                        let start = Instant::now();
                        if let Err(e) = vm.async_call(["main"], ()).await {
                            eprintln!("VU {} script error: {}", i + 1, e);
                        }
                        if tx
                            .send(Metric {
                                request_duration: start.elapsed(),
                            })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    println!("VU {} finished.", i + 1);
                });
                handles.push(handle);
            }

            // This drop is critical. It ensures the only `tx` clones left
            // are the ones inside the VUs.
            drop(tx);

            // Wait for all VUs to finish.
            join_all(handles).await;
        })
        .await;

    // 5. Wait for the aggregator to finish.
    // It will finish naturally because the channel is now guaranteed to close.
    aggregator_handle.await?;

    Ok(())
}
