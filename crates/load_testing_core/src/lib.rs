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
/// This is sent from each virtual user back to the main aggregator task.
#[derive(Debug)]
pub struct Metric {
    /// The total time taken for a single script iteration to complete.
    pub request_duration: Duration,
}

/// Configuration for the test run, provided by the CLI layer.
pub struct TestConfig {
    /// The raw source code of the Rune script to be executed.
    pub script: String,
    /// The number of concurrent virtual users to simulate.
    pub vus: u32,
    /// The total duration for which the test should run.
    pub duration: Duration,
}

pub async fn run_test(config: TestConfig) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel::<Metric>(1000);

    let ctx = Arc::new(rune_api::our_tool()?);
    let runtime = Arc::new(ctx.runtime()?);

    let mut sources = Sources::new();
    sources.insert(Source::new("script", &config.script)?)?;

    let mut diagnostics = Diagnostics::new();
    let build_res = rune::prepare(&mut sources)
        .with_context(&ctx)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    };

    let unit = Arc::new(build_res?);
    let local = LocalSet::new();

    let vus_runner = {
        let unit = unit.clone();
        let runtime = runtime.clone();
        let tx = tx.clone();

        local.run_until(async move {
            for i in 0..config.vus {
                let unit = unit.clone();
                let runtime = runtime.clone();
                let tx = tx.clone();
                let duration = config.duration;

                tokio::task::spawn_local(async move {
                    println!("Spawning VU {}", i + 1);
                    let mut vm = Vm::new(runtime, unit);
                    let test_end = Instant::now() + duration;

                    while Instant::now() < test_end {
                        let start = Instant::now();

                        match vm.async_call(["main"], ()).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("VU {} script error: {}", i + 1, e),
                        }

                        let metric = Metric {
                            request_duration: start.elapsed(),
                        };

                        if tx.send(metric).await.is_err() {
                            break;
                        }
                    }
                    println!("VU {} finished.", i + 1);
                });
            }
            drop(tx);
        })
    };

    let aggregator = async move {
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
    };

    tokio::join!(vus_runner, aggregator);
    Ok(())
}
