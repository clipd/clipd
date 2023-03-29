use clap::Parser;

#[derive(Parser, Debug)]
#[command(author)]
pub struct Args {
    #[clap(subcommand)]
    pub sub: Option<SubCommand>,

    /// Print version information
    #[arg(short = 'V', long)]
    pub version: bool,
}

#[derive(clap::Subcommand, Debug)]
pub enum SubCommand {
    /// Install service
    Install,
    /// Run service
    Run(RunArgs),
    /// Start service
    Start,
    /// Pause service
    Pause,
    /// Resume service
    Resume,
    /// Stop service
    Stop,
    /// Restart service
    Restart,
    /// Query service status
    Status,
    /// Uninstall service
    Uninstall(UninstallArgs),
}

#[derive(clap::Parser, Debug)]
pub struct RunArgs {
    /// Run as daemon service
    #[clap(short = 'D', long)]
    pub daemon: bool,
}

#[cfg(target_os = "windows")]
#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct UninstallArgs {
    /// Timeout waiting for the service to be deleted from the database
    #[clap(short, long, default_value_t = 5)]
    pub timeout: u64,
}

#[cfg(not(target_os = "windows"))]
#[derive(clap::Parser, Debug)]
pub struct UninstallArgs;
