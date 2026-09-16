mod adapters;
mod board;
mod cards;
mod credentials;
mod domain;
mod event_loop;
mod fixture_board;
mod fixtures;
mod layout;
mod overlay;
mod providers;
mod theme;
mod tui;
mod windows;

use crate::adapters::live_registry;
use crate::credentials::Credentials;
use crate::domain::{Clock, SystemClock};
use crate::event_loop::run_crossterm;
use crate::fixtures::{FixtureSet, fixture_registry};
use crate::providers::Provider;
use crate::tui::App;
use clap::Parser;
use std::io;
use std::path::Path;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "token-balance", disable_version_flag = true)]
struct Cli {
    #[arg(long, value_enum)]
    fixture: Option<FixtureSet>,
    #[arg(long)]
    muse_on_demand: bool,
}

fn print_version() {
    let argv0 = std::env::args()
        .next()
        .unwrap_or_else(|| "token-balance".into());
    let name = Path::new(&argv0)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("token-balance");
    println!("{name} {}", env!("CARGO_PKG_VERSION"));
}

fn build_providers(cli: &Cli, clock: Arc<dyn Clock>) -> Vec<Arc<dyn Provider>> {
    match cli.fixture {
        Some(set) => fixture_registry(set, clock),
        None => live_registry(
            &Credentials::from_process(),
            cli.muse_on_demand
                || matches!(
                    std::env::var("TOKEN_BALANCE_MUSE_ON_DEMAND").as_deref(),
                    Ok("1") | Ok("true") | Ok("TRUE")
                ),
        ),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        print_version();
        return;
    }
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");
    if let Err(e) = rt.block_on(run()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

async fn run() -> io::Result<()> {
    let cli = Cli::parse();
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let providers = build_providers(&cli, Arc::clone(&clock));
    let (app, mut fetch_rx) = App::new(providers, clock, cli.fixture);
    let app = app.bootstrap(&mut fetch_rx).await;
    run_crossterm(app, fetch_rx).await
}
