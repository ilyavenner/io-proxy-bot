use std::{io::Write, process::ChildStdin, sync::Arc};

use async_trait::async_trait;
use carapax::{
    types::{Integer, Message, MessageData, MessageKind, SupergroupChat},
    Api, Handler,
};
use snafu::ResultExt;
use tokio::sync::Mutex;

use crate::error::*;

/// A bot context.
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

pub struct MessageHandler;

#[async_trait]
impl Handler<Context> for MessageHandler {
    type Input = Message;
    type Output = Result<(), Error>;

    async fn handle(&mut self, context: &Context, message: Self::Input) -> Self::Output {
        let text: String = match message {
            Message {
                kind:
                    MessageKind::Supergroup {
                        chat: SupergroupChat { id, .. },
                        ..
                    },
                data: MessageData::Text(text),
                ..
            } if id == context.master_chat_id => text.data,
            _ => return Ok(()),
        };

        let mut server_stdin = context.server_stdin.lock().await;
        server_stdin
            .write_all(format!("{}\n", text).as_bytes())
            .context(IoError)?; // BUG: broken pipe

        Ok(())
    }
}
