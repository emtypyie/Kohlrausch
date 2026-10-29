use crate::config;
use clap::{Parser, Subcommand};

pub struct LaunchOptions {
    pub open_config: bool,
}

#[derive(Debug, Parser)]
#[command(
    name = "kohlrausch",
    version,
    about = "Kohlrausch Konfig configuration and device diagnostics"
)]
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
    Status {
        /// Include board metadata checks for hardware testing
        #[arg(long)]
        details: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ProfileCommand {
    List,
}

fn attach_parent_console() {
    #[cfg(windows)]
    if std::env::args_os().len() > 1 {
        use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
        use windows_sys::Win32::Storage::FileSystem::{
            CreateFileW, FILE_GENERIC_WRITE, FILE_SHARE_WRITE, OPEN_EXISTING,
        };
        use windows_sys::Win32::System::Console::{
            AttachConsole, GetStdHandle, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE,
            STD_OUTPUT_HANDLE,
        };

        unsafe {
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
            let console_output: Vec<u16> = "CONOUT$\0".encode_utf16().collect();
            for stream in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
                let handle = GetStdHandle(stream);
                if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                    let handle = CreateFileW(
                        console_output.as_ptr(),
                        FILE_GENERIC_WRITE,
                        FILE_SHARE_WRITE,
                        std::ptr::null(),
                        OPEN_EXISTING,
                        0,
                        std::ptr::null_mut(),
                    );
                    if !handle.is_null() && handle != INVALID_HANDLE_VALUE {
                        let _ = SetStdHandle(stream, handle);
                    }
                }
            }
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

fn device_diagnostics() -> &'static str {
    "\
Hardware metadata:
  Per-key matrix row/column identities: pending verification
  Confirmed pins: columns GP0-GP2; rows GP3-GP5; active-low LYR GP10
  Confirmed diode direction: columns-to-rows
  OLED controller, resolution, and I2C address: pending verification
OLED support: disabled in this firmware build
Firmware updates through the desktop app: unavailable
"
}

fn device_status(details: bool) -> Result<String, String> {
    let ports = crate::commands::serial_ports()?;
    let device = crate::commands::detect_kohlrausch_device();
    let port_status = if ports.is_empty() {
        "No serial ports detected".to_owned()
    } else {
        format!(
            "{} serial port(s) detected: {}",
            ports.len(),
            ports.join(", ")
        )
    };
    let identification = match device {
        Some(device) => format!(
            "Kohlrausch firmware: identified on {} (version {})",
            device.port, device.firmware_version
        ),
        None => "Kohlrausch firmware: no handshake detected".to_owned(),
    };
    let mut status = format!(
        "Device connection: No persistent session\n{identification}\nSerial ports: {port_status}\n"
    );
    if details {
        status.push_str(device_diagnostics());
    }
    Ok(status)
}

fn run_cli(command: Command) {
    match command {
        Command::Device {
            command: DeviceCommand::Status { details },
        } => match device_status(details) {
            Ok(status) => write_cli_output(&status, false),
            Err(error) => {
                write_cli_output(&format!("Device status error: {error}\n"), true);
                std::process::exit(1);
            }
        },
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

    #[test]
    fn device_status_diagnostics_option_parses() {
        let cli = Cli::try_parse_from(["kohlrausch", "device", "status", "--details"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Device {
                command: DeviceCommand::Status { details: true }
            })
        ));
    }

    #[test]
    fn device_diagnostics_keep_hardware_facts_cli_only() {
        let report = device_diagnostics();
        assert!(report.contains("Per-key matrix row/column identities: pending verification"));
        assert!(report.contains("Confirmed diode direction: columns-to-rows"));
        assert!(report.contains("OLED support: disabled in this firmware build"));
        assert!(
            report.contains("OLED controller, resolution, and I2C address: pending verification")
        );
        assert!(report.contains("Firmware updates through the desktop app: unavailable"));
    }
}
