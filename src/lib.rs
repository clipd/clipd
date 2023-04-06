use std::ffi::OsString;

use anyhow::{Context, Result};

mod data;
mod error;
pub mod fmt;
mod os;
mod util;

pub use data::*;
pub use error::*;
use os::*;
use sys_locale::get_locale;
use sysinfo::{RefreshKind, System, SystemExt};
pub use util::*;

pub const SERVICE_NAME: &'static str = "clipd";

pub fn run(args: Args) -> Result<()> {
    if args.version {
        println!("{}", format_version());
        std::process::exit(0)
    }

    let _guard = init_log(&args)?;
    for line in format_version().lines() {
        log::debug!("======== {}", line);
    }

    let oal = OAL::init(&args)?;

    log::debug!("args: {:?}", args);

    let sub_cmd = args
        .sub
        .unwrap_or(SubCommand::Run(RunArgs { daemon: false }));

    if let SubCommand::Run(args) = &sub_cmd {
        if let Err(e) = oal.run_clipd(args.daemon) {
            log::error!("Error: {:?}", e);
            return Err(e);
        }
        return Ok(());
    }

    let controller = oal.service_controller()?;
    match sub_cmd {
        SubCommand::Install => {
            controller.install(vec![OsString::from("run"), OsString::from("--daemon")])?
        }
        SubCommand::Run(_) => panic!(),
        SubCommand::Start => controller.start(vec![])?,
        SubCommand::Pause => controller.pause()?,
        SubCommand::Resume => controller.resume()?,
        SubCommand::Stop => controller.stop()?,
        SubCommand::Restart => controller.restart(vec![])?,
        SubCommand::Status => controller.status()?,
        SubCommand::Uninstall(_) => controller.uninstall()?,
    }
    Ok(())
}

fn init_log(args: &Args) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let level = if cfg!(debug_assertions) {
        log::Level::Trace
    } else {
        log::Level::Warn
    };
    let log_file_name = {
        let daemon = if let Some(SubCommand::Run(args)) = &args.sub {
            args.daemon
        } else {
            false
        };

        let exe = std::env::current_exe()?;
        let name = exe
            .file_name()
            .unwrap()
            .to_str()
            .unwrap_or(SERVICE_NAME)
            .replace(std::env::consts::EXE_SUFFIX, "");
        if daemon {
            format!("{}.s.log", name)
        } else {
            format!("{}.log", name)
        }
    };
    Ok(logger::start_tracing(level, &log_file_name).context("init logger")?)
}

fn format_version() -> String {
    use data::constant::*;
    use std::env::consts::*;
    let refresh_kind = RefreshKind::new();
    let sys = System::new_with_specifics(refresh_kind);
    let mut str = format!("{} revision {} • {}", VERSION, GIT_COMMIT_ID, BUILD_TIME);

    str.push('\n');

    str.push_str(&format!(
        "{} ({} {}) {}",
        sys.long_os_version().unwrap_or_default(),
        ARCH,
        get_locale().unwrap_or_default(),
        sys.kernel_version().unwrap_or_default(),
    ));
    str
}
