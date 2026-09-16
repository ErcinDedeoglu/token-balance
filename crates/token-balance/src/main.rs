mod accounts;
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

use crate::adapters::open_registry;
use crate::credentials::Credentials;
use crate::domain::{Clock, SystemClock};
use crate::event_loop::run_crossterm;
use crate::fixtures::FixtureSet;
use crate::tui::App;
use clap::{Parser, Subcommand};
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
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Write a commented accounts.toml template
    Init,
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

fn muse_on_demand(cli: &Cli) -> bool {
    cli.muse_on_demand
        || matches!(
            std::env::var("TOKEN_BALANCE_MUSE_ON_DEMAND").as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE")
        )
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
    if matches!(cli.command, Some(Cmd::Init)) {
        let path = crate::accounts::write_init(&Credentials::from_process())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        println!("wrote {}", path.display());
        return Ok(());
    }
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let creds = Credentials::from_process();
    let muse = muse_on_demand(&cli);
    let providers = match open_registry(cli.fixture, &creds, muse, Arc::clone(&clock)) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let (app, mut fetch_rx) = App::new(providers, clock, cli.fixture);
    let app = if cli.fixture.is_none() {
        app.with_live(creds, muse)
    } else {
        app
    };
    let app = app.bootstrap(&mut fetch_rx).await;
    run_crossterm(app, fetch_rx).await
}
