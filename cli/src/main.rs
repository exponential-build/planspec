use anyhow::Result;
use clap::{Parser, Subcommand};

mod client;
mod commands;
mod output;

#[derive(Parser)]
#[command(name = "planspec")]
#[command(author = "Exponential Build, Inc.")]
#[command(version)]
#[command(about = "CLI for PlanSpec - declarative work orchestration")]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// API server URL
    #[arg(long, env = "PLANSPEC_SERVER", global = true)]
    server: Option<String>,

    /// Output format (json, yaml, table)
    #[arg(short, long, default_value = "table", global = true)]
    output: String,

    /// Namespace to use
    #[arg(short, long, env = "PLANSPEC_NAMESPACE", global = true)]
    namespace: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Apply resources from a file
    Apply {
        /// File containing resources to apply
        #[arg(short, long)]
        file: String,

        /// Dry run mode - validate without applying
        #[arg(long)]
        dry_run: bool,
    },

    /// Get one or more resources
    Get {
        /// Resource type (goals, plans, capabilities, bindings, executions)
        resource: String,

        /// Resource name (optional, lists all if not specified)
        name: Option<String>,

        /// Label selector
        #[arg(short, long)]
        selector: Option<String>,

        /// List resources from all namespaces
        #[arg(short = 'A', long)]
        all_namespaces: bool,

        /// Filter plans by series
        #[arg(long)]
        series: Option<String>,
    },

    /// Show detailed information about a resource
    Describe {
        /// Resource type
        resource: String,

        /// Resource name
        name: String,
    },

    /// Validate resources against JSON schema (offline)
    Validate {
        /// File containing resources to validate
        #[arg(short, long)]
        file: String,
    },

    /// Watch for changes to resources
    Watch {
        /// Resource type to watch
        resource: String,

        /// Label selector
        #[arg(short, long)]
        selector: Option<String>,
    },

    /// Show diff between file and server state
    Diff {
        /// File containing resources to compare
        #[arg(short, long)]
        file: String,
    },

    /// Visualize a plan's DAG
    Graph {
        /// Plan name
        name: String,

        /// Output format (text, dot, mermaid)
        #[arg(long, default_value = "text")]
        format: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let config = client::Config {
        server: cli.server,
        namespace: cli.namespace,
    };

    match cli.command {
        Commands::Apply { file, dry_run } => {
            commands::apply::run(&config, &file, dry_run, &cli.output).await
        }
        Commands::Get {
            resource,
            name,
            selector,
            all_namespaces,
            series,
        } => {
            commands::get::run(
                &config,
                &resource,
                name,
                selector,
                all_namespaces,
                series,
                &cli.output,
            )
            .await
        }
        Commands::Describe { resource, name } => {
            commands::describe::run(&config, &resource, &name, &cli.output).await
        }
        Commands::Validate { file } => commands::validate::run(&file, &cli.output),
        Commands::Watch { resource, selector } => {
            commands::watch::run(&config, &resource, selector, &cli.output).await
        }
        Commands::Diff { file } => commands::diff::run(&config, &file, &cli.output).await,
        Commands::Graph { name, format } => {
            commands::graph::run(&config, &name, &format, &cli.output).await
        }
    }
}
