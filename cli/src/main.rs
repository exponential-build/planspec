use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
#[cfg(feature = "server")]
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
    /// Apply resources from a file or directory
    #[command(group(
        clap::ArgGroup::new("input")
            .required(true)
            .args(["file", "directory"]),
    ))]
    Apply {
        /// File containing resources to apply
        #[arg(short, long)]
        file: Option<String>,

        /// Directory containing YAML files to apply
        #[arg(short, long)]
        directory: Option<String>,

        /// Recursively process subdirectories (only with -d)
        #[arg(short = 'R', long, requires = "directory")]
        recursive: bool,

        /// Dry run mode - validate without applying
        #[arg(long)]
        dry_run: bool,
    },

    /// Get one or more resources
    Get {
        /// Resource type (goals, plans, capabilities, bindings, executions, all)
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

    /// Delete a resource or resources from a file/directory
    #[command(group(
        clap::ArgGroup::new("target")
            .required(true)
            .args(["resource", "file", "directory"]),
    ))]
    Delete {
        /// Resource type (goal, plan, capability, binding, execution, namespace)
        resource: Option<String>,

        /// Resource name (required when using resource type)
        #[arg(requires = "resource")]
        name: Option<String>,

        /// File containing resources to delete
        #[arg(short, long)]
        file: Option<String>,

        /// Directory containing YAML files with resources to delete
        #[arg(short, long)]
        directory: Option<String>,

        /// Recursively process subdirectories (only with -d)
        #[arg(short = 'R', long, requires = "directory")]
        recursive: bool,
    },

    /// Create a resource (currently only namespace is supported)
    Create {
        /// Resource type (namespace)
        resource: String,

        /// Resource name
        name: String,
    },

    /// Validate resources against JSON schema (offline)
    #[command(group(
        clap::ArgGroup::new("input")
            .required(true)
            .args(["file", "directory"]),
    ))]
    Validate {
        /// File containing resources to validate
        #[arg(short, long)]
        file: Option<String>,

        /// Directory containing YAML files to validate
        #[arg(short, long)]
        directory: Option<String>,

        /// Recursively process subdirectories (only with -d)
        #[arg(short = 'R', long, requires = "directory")]
        recursive: bool,
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

    /// Edit a resource in your editor (like kubectl edit)
    Edit {
        /// Resource type (goal, plan, capability, binding, execution)
        resource: String,

        /// Resource name
        name: String,

        /// Editor to use (defaults to $EDITOR, $VISUAL, or vi)
        #[arg(long, env = "EDITOR")]
        editor: Option<String>,
    },

    /// Start the PlanSpec API server
    #[cfg(feature = "server")]
    Serve {
        /// Host address to bind to
        #[arg(long, default_value = "127.0.0.1", env = "PLANSPEC_HOST")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value = "8080", env = "PORT")]
        port: u16,

        /// Path to SQLite database file (defaults to platform data directory)
        #[arg(long, env = "PLANSPEC_DB")]
        db: Option<String>,

        /// Plan reconciliation interval in seconds
        #[arg(long, default_value = "5")]
        reconcile_interval: u64,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
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
        Commands::Apply {
            file,
            directory,
            recursive,
            dry_run,
        } => commands::apply::run(&config, file, directory, recursive, dry_run, &cli.output).await,
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
        Commands::Delete {
            resource,
            name,
            file,
            directory,
            recursive,
        } => commands::delete::run(&config, resource, name, file, directory, recursive).await,
        Commands::Create { resource, name } => {
            commands::create::run(&config, &resource, &name).await
        }
        Commands::Validate {
            file,
            directory,
            recursive,
        } => commands::validate::run(file, directory, recursive, &cli.output),
        Commands::Watch { resource, selector } => {
            commands::watch::run(&config, &resource, selector, &cli.output).await
        }
        Commands::Diff { file } => commands::diff::run(&config, &file, &cli.output).await,
        Commands::Graph { name, format } => {
            commands::graph::run(&config, &name, &format, &cli.output).await
        }
        Commands::Edit {
            resource,
            name,
            editor,
        } => commands::edit::run(&config, &resource, &name, editor, &cli.output).await,
        #[cfg(feature = "server")]
        Commands::Serve {
            host,
            port,
            db,
            reconcile_interval,
        } => {
            // Initialize tracing for serve command
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| "planspec_server=info,tower_http=info".into()),
                )
                .with(tracing_subscriber::fmt::layer())
                .try_init()
                .ok();

            let db_path = db.unwrap_or_else(|| {
                commands::serve::default_db_path()
                    .to_string_lossy()
                    .to_string()
            });
            commands::serve::run(&host, port, &db_path, reconcile_interval).await
        }
        Commands::Completions { shell } => {
            commands::completions::run(shell, &mut Cli::command());
            Ok(())
        }
    }
}
