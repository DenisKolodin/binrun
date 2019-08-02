#![feature(async_await)]

mod settings;

use colored::*;
use failure::Error;
use futures::compat::{Future01CompatExt, Stream01CompatExt};
use futures::StreamExt;
use futures_legacy::prelude::*;
use std::io::BufReader;
use std::process::{Command, Stdio};
use tokio_process::{Child, ChildStdout, CommandExt};

#[runtime::main(runtime_tokio::Tokio)]
async fn main() -> Result<(), Error> {
    env_logger::try_init()?;
    let settings = settings::Settings::parse()?;
    let mut ctrl_c = tokio_signal::ctrl_c().flatten_stream().compat();
    for (name, bin) in settings.bins {
        runtime::spawn(async move {
            log::info!("Starting '{}': {}", name, bin.path);
            let mut cmd = Command::new(bin.path);
            cmd.stderr(Stdio::piped());
            match cmd.spawn_async() {
                Ok(mut child) => {
                    log::debug!("Started: '{}'", name);
                    if let Some(stderr) = child.stderr().take() {
                        let mut lines = tokio_io::io::lines(BufReader::new(stderr)).compat();
                        runtime::spawn(child.compat());
                        while let Some(line) = lines.next().await {
                            match line {
                                Ok(line) => {
                                    println!("{} | {}", name.green(), line);
                                }
                                Err(err) => {
                                    log::warn!("Can't read line from stderr of '{}': {}", name, err);
                                }
                            }
                        }
                    } else {
                        log::warn!("Can't get a stderr stream of '{}'", name);
                    }
                }
                Err(err) => {
                    log::error!("Can't start '{}': {}", name, err);
                }
            }
        });
    }
    ctrl_c.next().await;
    Ok(())
}
