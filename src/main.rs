use std::process::Stdio;

use carapax::{longpoll::LongPoll, Api, Config, Dispatcher};
use snafu::ResultExt;
use structopt::StructOpt;
use tokio::{io::BufReader, process::Command};

use crate::init::send_initialization_message;
use crate::{
    bot::{Context, SuperGroupMessageHandler},
    error::*,
    init::{setup_logger, Opt},
    proxy::stream_server_output,
};

pub mod bot;
pub mod error;
pub mod init;
pub mod proxy;

#[tokio::main]
async fn main() {
    setup_logger();
    let result = run().await;

    if let Err(e) = result {
        log::error!("{}", e);
    }
}

async fn run() -> Result<(), Error> {
    let Opt {
        token,
        master_chat_id,
        path_to_binary,
    } = StructOpt::from_args();

    let server_binary = Command::new(path_to_binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context(IoError)?;

    let server_stdin = server_binary.stdin.expect("server `stdin` should exist");
    let server_stdout = server_binary.stdout.expect("server `stdout` should exist");
    let server_stderr = server_binary.stderr.expect("server `stderr` should exist");

    let config = Config::new(token);
    let api = Api::new(config).expect("failed to create API");
    let context = Context::new(api.clone(), server_stdin, master_chat_id);

    send_initialization_message(&context).await?;

    let cloned_context = context.clone();
    let server_stdout_reader = tokio::spawn(async move {
        let stdout_reader = BufReader::new(server_stdout);
        stream_server_output(cloned_context, stdout_reader).await
    });

    let cloned_context = context.clone();
    let server_stderr_reader = tokio::spawn(async move {
        let stderr_reader = BufReader::new(server_stderr);
        stream_server_output(cloned_context, stderr_reader).await
    });

    let mut dispatcher = Dispatcher::new(context);
    dispatcher.add_handler(SuperGroupMessageHandler);

    log::info!("Running the bot...");
    LongPoll::new(api, dispatcher).run().await;

    server_stdout_reader
        .await
        .expect("cannot join `server_stdout_reader`")?;

    server_stderr_reader
        .await
        .expect("cannot join `server_stderr_reader`")?;

    Ok(())
}
