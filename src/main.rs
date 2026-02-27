mod commands;
mod convert;
mod error;
mod neo4j_client;

use agcli::{AgentCli, ExecutionContext};

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: agcli::Jemalloc = agcli::Jemalloc;

#[tokio::main]
async fn main() {
    // Panic hook: output JSON error envelope on panic
    std::panic::set_hook(Box::new(|info| {
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| {
                info.payload()
                    .downcast_ref::<String>()
                    .map(|s| s.as_str())
            })
            .unwrap_or("unknown panic");
        eprintln!(
            r#"{{"ok":false,"error":{{"message":"Internal error: {msg}","code":"PANIC","retryable":false}},"fix":"Report this bug"}}"#,
        );
    }));

    let cli = AgentCli::new("lowmain", "Agent-native Neo4j CLI")
        .version(env!("CARGO_PKG_VERSION"))
        .command(commands::ping::register())
        .command(commands::query::register())
        .command(commands::schema::register())
        .command(commands::nodes::register())
        .command(commands::rels::register());

    let mut ctx = ExecutionContext::default();
    let run = cli.run_env_with_context(&mut ctx).await;
    println!("{}", run.to_json());
    std::process::exit(run.exit_code());
}
