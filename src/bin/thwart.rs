use clap::{Parser, Subcommand};
use std::process::ExitCode;

const DEFAULT_WORLD_PATH: &str = "authors/mx/test-world";

#[derive(Parser)]
#[command(name = "thwart")]
#[command(about = "A Hugo-style engine for interactive fiction")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, global = true, help = "Specify world directory")]
    world: Option<String>,
    
    #[arg(long, global = true, help = "Use custom configuration file")]
    config: Option<String>,
    
    #[arg(long, short = 'q', global = true, help = "Suppress non-essential output")]
    quiet: bool,
    
    #[arg(long, short = 'v', global = true, help = "Enable verbose output")]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Open the console in edit mode, to write a world
    Edit {
        /// World directory (defaults to --world, then the built-in default)
        #[arg(value_name = "WORLD")]
        world_dir: Option<String>,
    },
    /// Open the console in play mode, to read (play) a world
    Play {
        /// World directory, or an installed world as persona/world
        /// (defaults to --world, then the built-in default)
        #[arg(value_name = "WORLD")]
        world_dir: Option<String>,
    },
    /// Package a world and its persona into one .thwarter file to share
    Package {
        /// World directory (defaults to --world, then the built-in default)
        #[arg(value_name = "WORLD")]
        world_dir: Option<String>,
        #[arg(long, help = "Where to write the package (default: <persona>-<world>-<version>.thwarter here)")]
        output: Option<String>,
        #[arg(long, help = "Leave the author comments (<!-- -->) out of the packaged files")]
        strip_comments: bool,
        #[arg(long, help = "Don't ask before publishing the persona's contact details")]
        yes: bool,
    },
    /// Install a packaged world to play
    Install {
        /// A .thwarter file
        package: String,
        #[arg(long, help = "Replace an installed copy of the same world")]
        replace: bool,
    },
    /// List the installed worlds
    Installed,
    /// Validate world files
    Validate {
        /// Location to validate (optional)
        location: Option<String>,
        #[arg(long, help = "Enable strict validation rules")]
        strict: bool,
        #[arg(long, help = "Save validation report")]
        output: Option<String>,
        #[arg(long, help = "Report format", default_value = "text")]
        format: String,
        #[arg(long, help = "Attempt to auto-fix common issues")]
        fix: bool,
    },
    /// Display world state
    Show {
        /// A location, object or character, by id or link (optional; the
        /// world's summary without one)
        location: Option<String>,
        #[arg(long, help = "location, object or character, when the id names more than one")]
        kind: Option<String>,
        #[arg(long, help = "Show a template or built-in as written")]
        template: Option<String>,
        #[arg(long, help = "Show from specific player perspective")]
        player: Option<String>,
        #[arg(long, help = "Show the thing in one of its states (design/STATES.md)")]
        state: Option<String>,
        #[arg(long, help = "Show the thing in a chapter")]
        chapter: Option<String>,
        #[arg(long, help = "Output format", default_value = "text")]
        format: String,
    },
    /// Generate maps
    Map {
        /// Location to generate map from (optional)
        location: Option<String>,
        #[arg(long, help = "Generate from player perspective (fog-of-war)")]
        player: Option<String>,
        #[arg(long, help = "Limit map depth/radius")]
        depth: Option<usize>,
        #[arg(long, help = "Map rendering style", default_value = "ascii")]
        style: String,
        #[arg(long, help = "Save map to file")]
        output: Option<String>,
        #[arg(long, help = "Show connection types between locations")]
        connections: bool,
    },
    /// Find content
    Search {
        /// Search query
        query: String,
        /// Location to search in (optional)
        location: Option<String>,
        #[arg(long, help = "Limit search scope")]
        r#type: Option<String>,
        #[arg(long, help = "Enable case-sensitive matching")]
        case_sensitive: bool,
        #[arg(long, help = "Use regular expression matching")]
        regex: bool,
        #[arg(long, help = "Find broken location references")]
        missing_connections: bool,
    },
    /// Debugging tools
    Debug {
        /// Location to debug (optional)
        location: Option<String>,
        #[arg(long, help = "Validate all location/character references")]
        check_references: bool,
        #[arg(long, help = "Show memory usage statistics")]
        memory_usage: bool,
        #[arg(long, help = "Trace specific action execution")]
        trace_action: Option<String>,
        #[arg(long, help = "Enable performance profiling")]
        profile: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let world_path = cli.world.clone().unwrap_or_else(|| DEFAULT_WORLD_PATH.to_string());

    let result = match cli.command {
        Commands::Edit { world_dir } => {
            let path = world_dir.unwrap_or(world_path);
            thwarter::console::run(std::path::Path::new(&path), thwarter::console::Mode::Edit)
        },
        Commands::Play { world_dir } => {
            let path = world_dir.unwrap_or(world_path);
            // An installed world by name, when there's no such folder here.
            let path = match thwarter::settings::data_dir() {
                Some(data) if !std::path::Path::new(&path).exists() => {
                    thwarter::packaging::find_installed(&data, &path).unwrap_or_else(|| std::path::PathBuf::from(&path))
                }
                _ => std::path::PathBuf::from(&path),
            };
            thwarter::console::run(&path, thwarter::console::Mode::Play)
        },
        Commands::Package { world_dir, output, strip_comments, yes } => {
            thwarter::cli::commands::package_command(world_dir.unwrap_or(world_path), output, strip_comments, yes)
        },
        Commands::Install { package, replace } => thwarter::cli::commands::install_command(package, replace),
        Commands::Installed => thwarter::cli::commands::installed_command(),
        Commands::Validate { location, strict, output, format, fix } => {
            thwarter::cli::commands::validate_command(world_path, location, strict, output, format, fix)
        },
        Commands::Show { location, kind, template, player, state, chapter, format } => {
            thwarter::cli::commands::show_command(thwarter::cli::commands::ShowRequest {
                world_path,
                name: location,
                kind,
                template,
                player,
                state,
                chapter,
                format,
                verbose: cli.verbose,
            })
        },
        Commands::Map { location, player, depth, style, output, connections } => {
            thwarter::cli::commands::map_command(world_path, location, player, depth, style, output, connections)
        },
        Commands::Search { query, location, r#type, case_sensitive, regex, missing_connections } => {
            thwarter::cli::commands::search_command(query, location, r#type, case_sensitive, regex, missing_connections)
        },
        Commands::Debug { location, check_references, memory_usage, trace_action, profile } => {
            thwarter::cli::commands::debug_command(location, check_references, memory_usage, trace_action, profile)
        },
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        // The command already printed its own failure; just exit non-zero.
        Err(e) if e.is::<thwarter::cli::AlreadyReported>() => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("Error: {:?}", e);
            ExitCode::FAILURE
        }
    }
}
