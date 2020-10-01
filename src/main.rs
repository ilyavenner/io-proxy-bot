use std::{
    io::{BufRead, BufReader, Read},
    process::{Command, Stdio},
};

use carapax::{longpoll::LongPoll, methods::SendMessage, types::Integer, Api, Config, Dispatcher};
use snafu::ResultExt;
use structopt::StructOpt;

use crate::{
    bot::{Context, MessageHandler},
    error::*,
    init::{setup_logger, Opt},
};

mod bot;
mod error;
mod init;

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

    let cloned_api = api.clone();
    let server_stdout_reader = tokio::spawn(async move {
        let api = cloned_api;
        let stdout_reader = BufReader::new(server_stdout);
        read_server_output(api, master_chat_id, stdout_reader).await
    });

    let cloned_api = api.clone();
    let server_stderr_reader = tokio::spawn(async move {
        let api = cloned_api;
        let stderr_reader = BufReader::new(server_stderr);
        read_server_output(api, master_chat_id, stderr_reader).await
    });

    let mut dispatcher = Dispatcher::new(context);
    dispatcher.add_handler(MessageHandler);

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

async fn read_server_output<R>(
    api: Api,
    master_chat_id: Integer,
    reader: BufReader<R>,
) -> Result<(), Error>
where
    R: Read,
{
    for line in reader.lines() {
        let line = line.context(IoError)?;
        let text = format!("@: {}", line);
        let message = SendMessage::new(master_chat_id, text);

        api.execute(message).await.context(ExecuteError)?;
    }

    Ok(())
}
