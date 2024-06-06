use crate::connect_to_coordinator;
use communication_layer_request_reply::TcpRequestReplyConnection;
use dora_core::topics::{ControlRequest, ControlRequestReply};
use eyre::{bail, Context};
use std::{
    collections::BTreeSet,
    io::{IsTerminal,Write},
    net::SocketAddr,
};
use termcolor::{Color, ColorChoice, ColorSpec, WriteColor};

pub fn check_environment(coordinator_addr: SocketAddr) -> eyre::Result<()> {
    let mut error_occured = false;

    let color_choice = if std::io::stdout().is_terminal() {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    };
    let mut stdout = termcolor::StandardStream::stdout(color_choice);

    // check whether coordinator is running
    write!(stdout, "Dora Coordinator: ")?;
    let mut session = match connect_to_coordinator(coordinator_addr) {
        Ok(session) => {
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
            writeln!(stdout, "ok")?;
            Some(session)
        }
        Err(_) => {
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
            writeln!(stdout, "not running")?;
            None
        }
    };

    let _ = stdout.reset();

    // check whether daemon is running
    write!(stdout, "Dora Daemon: ")?;

    if let Some(session) = session.as_deref_mut() {
        match daemon_running(session) {
            Ok(daemon_running) => {
                if daemon_running.is_empty() {
                    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
                    writeln!(stdout, "not running")?;

                    error_occured = true;
                } else {
                    writeln!(stdout)?;
                    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)));
                    for item in &daemon_running {
                        writeln!(stdout, " - {} ok", item)?;
                    }
                }
            }
            Err(_) => {
                let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
                writeln!(stdout, "not running")?;
                error_occured = true;
            }
        }
        let _ = stdout.reset();
        writeln!(stdout)?;
        if error_occured {
            bail!("Environment check failed.");
        }
    }
    Ok(())
}

pub fn daemon_running(session: &mut TcpRequestReplyConnection) -> Result<BTreeSet<String>, eyre::ErrReport> {
    let reply_raw = session
        .request(&serde_json::to_vec(&ControlRequest::ConnectedMachines).unwrap())
        .wrap_err("failed to send DaemonConnected message")?;

    let reply = serde_json::from_slice(&reply_raw).wrap_err("failed to parse reply")?;
    let running = match reply {
        ControlRequestReply::ConnectedMachines(running) => running,
        other => bail!("unexpected reply to daemon connection check: {other:?}"),
    };

    Ok(running)
}
