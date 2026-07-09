use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "ingest-cli",
    about = "One-shot document ingest tool for Spontini"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run the ingest pipeline for one or more sources
    Run(RunArgs),
}

#[derive(clap::Args, Debug)]
pub struct RunArgs {
    /// URL to scrape and ingest (mutually exclusive with --all-sources)
    #[arg(long, conflicts_with = "all_sources")]
    pub url: Option<String>,

    /// Section name to assign to ingested documents
    #[arg(long)]
    pub section: Option<String>,

    /// Ingest all configured scrape sources for the given section
    #[arg(long)]
    pub all_sources: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_run_with_url_and_section() {
        let cli = Cli::try_parse_from([
            "ingest-cli",
            "run",
            "--url",
            "https://example.com",
            "--section",
            "news",
        ]);
        assert!(cli.is_ok(), "expected OK, got: {cli:?}");
        let cli = cli.unwrap();
        match cli.command {
            Commands::Run(args) => {
                assert_eq!(args.url.as_deref(), Some("https://example.com"));
                assert_eq!(args.section.as_deref(), Some("news"));
                assert!(!args.all_sources);
            }
        }
    }

    #[test]
    fn should_parse_run_with_all_sources_and_section() {
        let cli = Cli::try_parse_from(["ingest-cli", "run", "--all-sources", "--section", "sport"]);
        assert!(cli.is_ok(), "expected OK, got: {cli:?}");
        let cli = cli.unwrap();
        match cli.command {
            Commands::Run(args) => {
                assert!(args.url.is_none());
                assert_eq!(args.section.as_deref(), Some("sport"));
                assert!(args.all_sources);
            }
        }
    }

    #[test]
    fn should_reject_url_and_all_sources_together() {
        let cli = Cli::try_parse_from([
            "ingest-cli",
            "run",
            "--url",
            "https://example.com",
            "--all-sources",
            "--section",
            "news",
        ]);
        assert!(cli.is_err(), "expected conflict error, got: {cli:?}");
    }

    #[test]
    fn should_accept_run_without_section() {
        let cli = Cli::try_parse_from(["ingest-cli", "run", "--url", "https://example.com"]);
        assert!(
            cli.is_ok(),
            "expected OK, section is optional at parse level; got: {cli:?}"
        );
        let cli = cli.unwrap();
        match cli.command {
            Commands::Run(args) => {
                assert_eq!(args.url.as_deref(), Some("https://example.com"));
                assert!(args.section.is_none());
            }
        }
    }

    #[test]
    fn should_accept_run_with_section_only() {
        let cli = Cli::try_parse_from(["ingest-cli", "run", "--section", "news"]);
        assert!(
            cli.is_ok(),
            "should accept section only (validation in main); got: {cli:?}"
        );
        let cli = cli.unwrap();
        match cli.command {
            Commands::Run(args) => {
                assert!(args.url.is_none());
                assert_eq!(args.section.as_deref(), Some("news"));
                assert!(!args.all_sources);
            }
        }
    }

    #[test]
    fn should_show_help_when_no_args() {
        let cli = Cli::try_parse_from(["ingest-cli"]);
        assert!(cli.is_err(), "expected error for no subcommand");
    }
}
