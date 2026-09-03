use clap::{Parser, Subcommand};
use std::path::PathBuf;
use usbwatch::{
    default_session, inspect_device, list_devices, load_session, parse_hex_u16, replay_session,
};

#[derive(Debug, Parser)]
#[command(
    name = "usbwatch",
    version,
    about = "Inspect USB descriptors and recorded mock sessions"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    List {
        #[arg(
            long,
            help = "Recorded session JSON; default is the built-in mock device"
        )]
        fixture: Option<PathBuf>,
    },
    Inspect {
        #[arg(long, value_parser = parse_hex_u16)]
        vid: u16,
        #[arg(long, value_parser = parse_hex_u16)]
        pid: u16,
        #[arg(long)]
        fixture: Option<PathBuf>,
    },
    Replay {
        fixture: PathBuf,
        #[arg(long, default_value_t = usbwatch::MAX_EVENTS)]
        max_events: usize,
    },
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::List { fixture } => {
            let session = fixture
                .as_deref()
                .map(load_session)
                .transpose()
                .map_err(|error| error.to_string())?
                .unwrap_or_else(default_session);
            for record in list_devices(&session).map_err(|error| error.to_string())? {
                println!(
                    "{}",
                    serde_json::to_string(&record).map_err(|error| error.to_string())?
                );
            }
        }
        Command::Inspect { vid, pid, fixture } => {
            let session = fixture
                .as_deref()
                .map(load_session)
                .transpose()
                .map_err(|error| error.to_string())?
                .unwrap_or_else(default_session);
            let device = session
                .devices
                .iter()
                .find(|device| device.vid == vid && device.pid == pid)
                .ok_or_else(|| format!("no mock device {:04x}:{:04x}", vid, pid))?;
            let inspection = inspect_device(device).map_err(|error| error.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&inspection).map_err(|error| error.to_string())?
            );
        }
        Command::Replay {
            fixture,
            max_events,
        } => {
            let session = load_session(&fixture).map_err(|error| error.to_string())?;
            for record in replay_session(&session, max_events).map_err(|error| error.to_string())? {
                println!(
                    "{}",
                    serde_json::to_string(&record).map_err(|error| error.to_string())?
                );
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("usbwatch: {error}");
        std::process::exit(2);
    }
}
