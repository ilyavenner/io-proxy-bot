use std::{
    io::{BufRead, BufReader},
    process::{ChildStdout, Command, Stdio},
    thread::sleep,
};

use carapax::{
    longpoll::LongPoll,
    methods::SendMessage,
    types::{Integer, ParseMode},
    Api, Config, Dispatcher,
};
use snafu::ResultExt;
use structopt::StructOpt;
use tokio::time::Duration;

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
    let result = run(StructOpt::from_args()).await;

    if let Err(e) = result {
        log::error!("{}", e);
    }
}

async fn run(opt: Opt) -> Result<(), Error> {
    let Opt {
        token,
        master_chat_id,
        path_to_binary,
    } = opt;

    let server_binary = Command::new(path_to_binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context(IoError)?;

    let server_stdin = server_binary.stdin.expect("server `stdin` should exist");
    let server_stdout = server_binary.stdout.expect("server `stdout` should exist");

    let config = Config::new(token);
    let api = Api::new(config).expect("failed to create API");
    let context = Context::new(api.clone(), server_stdin);

    let cloned_api = api.clone();
    let server_stdout_reader = tokio::spawn(async move {
        let api = cloned_api;
        let stdout_reader = BufReader::new(server_stdout);
        read_server_stdout(master_chat_id, api, stdout_reader).await
    });

    let mut dispatcher = Dispatcher::new(context);
    dispatcher.add_handler(MessageHandler);

    log::info!("Running the bot...");
    LongPoll::new(api, dispatcher).run().await;

    server_stdout_reader
        .await
        .expect("cannot join `server_stdout_reader`")?;

    Ok(())
}

async fn read_server_stdout(
    master_chat_id: Integer,
    api: Api,
    stdout_reader: BufReader<ChildStdout>,
) -> Result<(), Error> {
    for line in stdout_reader.lines() {
        sleep(Duration::from_millis(500));

        let line = line.context(IoError)?;
        let text = format!("```{}```", line);
        let message = SendMessage::new(master_chat_id, text).parse_mode(ParseMode::Markdown);

        if let Err(e) = api.execute(message).await {
            if let carapax::ExecuteError::Response(_) = e {
                let text = format!("{}", line);
                let message = SendMessage::new(master_chat_id, text);

                api.execute(message).await.context(ExecuteError)?;
            }
        }
    }

    Ok(())
}
