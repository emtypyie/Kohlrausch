use crate::config;
use clap::{Parser, Subcommand};

pub struct LaunchOptions {
    pub open_config: bool,
}

#[derive(Debug, Parser)]
#[command(name = "kohlrausch", version, about = "Kohlrausch Konfig macropad configuration")]
struct Cli {
    /// Open the Config Studio workspace
    #[arg(long, visible_alias = "c")]
    config: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Device {
        #[command(subcommand)]
        command: DeviceCommand,
    },
    Profiles {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    Flash {
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Debug, Subcommand)]
enum DeviceCommand {
    Status,
}

#[derive(Debug, Subcommand)]
enum ProfileCommand {
    List,
}

fn attach_parent_console() {
    #[cfg(windows)]
    if std::env::args_os().len() > 1 {
        use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};

        unsafe {
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

fn write_cli_output(text: &str, is_error: bool) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::Storage::FileSystem::WriteFile;
        use windows_sys::Win32::System::Console::{
            GetConsoleMode, GetStdHandle, WriteConsoleW, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
        };

        let handle = GetStdHandle(if is_error {
            STD_ERROR_HANDLE
        } else {
            STD_OUTPUT_HANDLE
        });
        if handle.is_null() || handle as isize == -1 {
            return;
        }

        let mut mode = 0;
        if GetConsoleMode(handle, &mut mode) != 0 {
            let wide: Vec<u16> = text.encode_utf16().collect();
            let mut written = 0;
            let _ = WriteConsoleW(
                handle,
                wide.as_ptr(),
                wide.len() as u32,
                &mut written,
                std::ptr::null(),
            );
        } else {
            let mut written = 0;
            let _ = WriteFile(
                handle,
                text.as_ptr(),
                text.len() as u32,
                &mut written,
                std::ptr::null_mut(),
            );
        }
    }
    #[cfg(not(windows))]
    if is_error {
        eprint!("{text}");
    } else {
        print!("{text}");
    }
}

fn device_status() -> String {
    let mut ports = serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|port| port.port_name)
        .collect::<Vec<_>>();
    ports.sort();
    if ports.is_empty() {
        "Device status: no serial ports detected; no compatible Kohlrausch device is connected.\n".to_owned()
    } else {
        format!(
            "Device status: {} serial port(s) detected ({}); no verified Kohlrausch handshake has occurred.\n",
            ports.len(),
            ports.join(", ")
        )
    }
}

fn run_cli(command: Command) {
    match command {
        Command::Device {
            command: DeviceCommand::Status,
        } => write_cli_output(&device_status(), false),
        Command::Profiles {
            command: ProfileCommand::List,
        } => match config::load_profiles() {
            Ok(profiles) => {
                for profile in profiles {
                    write_cli_output(
                        &format!(
                            "{} ({}) · {} assignments\n",
                            profile.name,
                            profile.profile_id,
                            profile.assignments.len()
                        ),
                        false,
                    );
                }
            }
            Err(error) => {
                write_cli_output(&format!("Profile storage error: {error}\n"), true);
                std::process::exit(1);
            }
        },
        // Flashing remains disabled until identification, target validation, confirmation,
        // progress/error handling and post-flash verification exist. Dry-run never writes.
        Command::Flash { dry_run: true } => write_cli_output(
            "No firmware operation initiated: flashing disabled (no verified device/target). Dry-run has no effect.\n",
            false,
        ),
        Command::Flash { dry_run: false } => {
            write_cli_output(
                "Firmware update unavailable: flashing disabled pending device identification, target validation, confirmation and verification.\n",
                true,
            );
            std::process::exit(2);
        }
    }
}

/// Parse the command line. Returns `None` when a CLI subcommand was handled and the GUI
/// must not start. Returns launch options when the graphical application should run.
pub fn dispatch() -> Option<LaunchOptions> {
    attach_parent_console();
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            write_cli_output(&error.to_string(), error.use_stderr());
            std::process::exit(error.exit_code());
        }
    };
    if let Some(command) = cli.command {
        run_cli(command);
        return None;
    }
    Some(LaunchOptions {
        open_config: cli.config,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_flag_is_supported() {
        let cli = Cli::try_parse_from(["kohlrausch", "--config"]).unwrap();
        assert!(cli.config);
    }

    #[test]
    fn short_configuration_alias_is_supported() {
        let cli = Cli::try_parse_from(["kohlrausch", "--c"]).unwrap();
        assert!(cli.config);
    }

    #[test]
    fn profile_list_subcommand_parses() {
        let cli = Cli::try_parse_from(["kohlrausch", "profiles", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Profiles {
                command: ProfileCommand::List
            })
        ));
    }
}
