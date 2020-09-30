use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    thread::sleep,
};

use carapax::{longpoll::LongPoll, methods::SendMessage, Api, Config, Dispatcher};
use structopt::StructOpt;
use tokio::time::Duration;

use crate::{
    bot::{Context, MessageHandler},
    init::{setup_logger, Opt},
};
use carapax::types::ParseMode;

mod bot;
mod init;

#[tokio::main]
async fn main() {
    setup_logger();

    let Opt {
        token,
        master_chat_id,
        path_to_binary,
    } = StructOpt::from_args();

    let server_binary = Command::new(path_to_binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let server_stdin = server_binary.stdin.expect("server `stdin` should exist");
    let server_stdout = server_binary.stdout.expect("server `stdout` should exist");

    let config = Config::new(token);
    let api = Api::new(config).expect("failed to create API");
    let context = Context::new(api.clone(), server_stdin);

    let cloned_api = api.clone();
    let server_stdout_reader = tokio::spawn(async move {
        let api = cloned_api;
        let stdout_reader = BufReader::new(server_stdout);

        for line in stdout_reader.lines() {
            sleep(Duration::from_millis(500));

            let message = SendMessage::new(
                master_chat_id,
                format!("```{}```", line.expect("cannot read string")),
            )
            .parse_mode(ParseMode::Markdown);

            api.execute(message).await.expect("cannot send message");
        }
    });

    let mut dispatcher = Dispatcher::new(context);
    dispatcher.add_handler(MessageHandler);

    log::info!("Running the bot...");
    LongPoll::new(api, dispatcher).run().await;

    server_stdout_reader
        .await
        .expect("cannot join `server_stdout_reader`");
}
