//! Owns the one OS thread every *read-only* COM call in `firewall::netfw`
//! runs on. `INetFwPolicy2`/`INetFwRule` are apartment-threaded COM
//! objects; Tauri's async command handlers can run on any worker thread in
//! the runtime's pool, so calling COM directly from a command would be a
//! threading violation the moment two commands land on different threads.
//! Routing every call through a single dedicated thread sidesteps that
//! entirely.
//!
//! Only reads live here - checking firewall/profile status and a rule's
//! current enabled state never requires administrator rights on Windows.
//! Anything that *changes* firewall state goes through
//! `firewall::elevated_ops` instead, since that always does require
//! elevation - see that module and `elevation.rs`.

use std::sync::mpsc as std_mpsc;

use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

use crate::error::AppResult;
use crate::store::models::FirewallStatus;

use super::netfw;

pub enum Command {
    Status(tokio::sync::oneshot::Sender<AppResult<FirewallStatus>>),
    GetEnabled {
        name: String,
        resp: tokio::sync::oneshot::Sender<AppResult<Option<bool>>>,
    },
}

#[derive(Clone)]
pub struct Handle {
    tx: std_mpsc::Sender<Command>,
}

impl Handle {
    /// Spawns the worker thread and returns a cheap, cloneable handle to it.
    /// The thread runs for the life of the process; it exits cleanly when
    /// every `Handle` (and so every `Sender`) is dropped and `recv()` ends.
    pub fn spawn() -> Self {
        let (tx, rx) = std_mpsc::channel::<Command>();
        std::thread::Builder::new()
            .name("thomsen-shield-firewall".into())
            .spawn(move || run(rx))
            .expect("failed to spawn the firewall worker thread");
        Self { tx }
    }

    pub async fn status(&self) -> AppResult<FirewallStatus> {
        let (resp, rx) = tokio::sync::oneshot::channel();
        self.send(Command::Status(resp));
        recv(rx).await
    }

    pub async fn get_enabled(&self, name: String) -> AppResult<Option<bool>> {
        let (resp, rx) = tokio::sync::oneshot::channel();
        self.send(Command::GetEnabled { name, resp });
        recv(rx).await
    }

    fn send(&self, cmd: Command) {
        // The worker thread never exits while the app is running, so this
        // only fails during shutdown races - dropping `cmd` (and so the
        // response sender) is the right behavior there, the caller's
        // `recv().await` below simply observes a closed channel.
        let _ = self.tx.send(cmd);
    }
}

async fn recv<T>(rx: tokio::sync::oneshot::Receiver<AppResult<T>>) -> AppResult<T> {
    match rx.await {
        Ok(result) => result,
        Err(_) => Err(crate::error::AppError::new("The firewall worker stopped unexpectedly.")),
    }
}

fn run(rx: std_mpsc::Receiver<Command>) {
    if let Err(e) = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.ok() {
        // Every subsequent COM call on this thread would fail too; report
        // that once per request rather than panicking the whole app.
        let msg = format!("Couldn't read Windows Firewall state: {e}");
        for cmd in rx {
            reply_init_failed(cmd, &msg);
        }
        return;
    }

    for cmd in rx {
        match cmd {
            Command::Status(resp) => {
                let _ = resp.send(netfw::status());
            }
            Command::GetEnabled { name, resp } => {
                let _ = resp.send(netfw::get_enabled(&name));
            }
        }
    }

    unsafe { CoUninitialize() };
}

fn reply_init_failed(cmd: Command, msg: &str) {
    let err = || crate::error::AppError::new(msg.to_string());
    match cmd {
        Command::Status(resp) => {
            let _ = resp.send(Err(err()));
        }
        Command::GetEnabled { resp, .. } => {
            let _ = resp.send(Err(err()));
        }
    }
}
