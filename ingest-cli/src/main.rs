mod cli;
mod run;

use clap::Parser;
use cli::{Cli, Commands};
use run::{RunConfig, run_all_sources, run_url};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let config = RunConfig::default();

    match cli.command {
        Commands::Run(args) => {
            let section = args.section.unwrap_or_default();

            if let Some(url) = args.url {
                if let Err(e) = run_url(&url, &section, &config).await {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
                println!("ingested: {url} -> section {section}");
            } else if args.all_sources {
                if let Err(e) = run_all_sources(&section, &config).await {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            } else {
                eprintln!("error: either --url or --all-sources is required");
                std::process::exit(1);
            }
        }
    }
}
