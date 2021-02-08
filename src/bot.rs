use std::sync::Arc;

use async_trait::async_trait;
use carapax::{
    types::{Integer, Message, MessageData, MessageKind, SupergroupChat},
    Api, Handler,
};
use snafu::ResultExt;
use tokio::{io::AsyncWriteExt, process::ChildStdin, sync::Mutex};

use crate::error::*;
use carapax::methods::SendMessage;

/// A bot context.
#[derive(Clone)]
pub struct Context {
    /// A [carapax::Api].
    pub api: Api,

    /// A Minecraft Bedrock server `stdin`.
    pub server_stdin: Arc<Mutex<ChildStdin>>,

    /// A target chat ID.
    pub master_chat_id: Integer,
}

impl Context {
    pub fn new(api: Api, server_stdin: ChildStdin, master_chat_id: Integer) -> Self {
        Self {
            api,
            server_stdin: Arc::new(Mutex::new(server_stdin)),
            master_chat_id,
        }
    }
}

pub struct SuperGroupMessageHandler;

#[async_trait]
impl Handler<Context> for SuperGroupMessageHandler {
    type Input = Message;
    type Output = Result<(), Error>;

    async fn handle(&mut self, context: &Context, message: Self::Input) -> Self::Output {
        process_message(context, message).await
    }
}

const COMMENT_PATTERN: &str = "#";

async fn process_message(context: &Context, message: Message) -> Result<(), Error> {
    let message_text = extract_master_chat_message_text(context, message);

    if let Some(text) = message_text {
        let text_lines = text
            .split("\n")
            .filter(|line| !line.starts_with(COMMENT_PATTERN));

        let mut server_stdin = context.server_stdin.lock().await;

        for line in text_lines {
            server_stdin
                .write_all(format!("{}\n", line).as_bytes())
                .await
                .context(IoError)?;
        }
    }

    Ok(())
}

fn extract_master_chat_message_text(context: &Context, message: Message) -> Option<String> {
    match message {
        Message {
            kind:
                MessageKind::Supergroup {
                    chat: SupergroupChat { id, .. },
                    ..
                },
            data: MessageData::Text(text),
            ..
        } if id == context.master_chat_id => Some(text.data),
        _ => None,
    }
}

pub async fn send_message_to_master_chat(context: &Context, text: &str) -> Result<(), Error> {
    println!("{}", text);

    let message = SendMessage::new(context.master_chat_id, text);
    context
        .api
        .execute(message)
        .await
        .map(|_| ())
        .context(ExecuteError)
}
