use std::{io::Write, process::ChildStdin, sync::Arc};

use async_trait::async_trait;
use carapax::{
    types::{Message, MessageData},
    Api, Handler,
};
use tokio::sync::Mutex;

/// A bot context.
pub struct Context {
    /// A [carapax::Api].
    pub api: Api,

    /// A Minecraft Bedrock server `stdin`.
    pub server_stdin: Arc<Mutex<ChildStdin>>,
}

impl Context {
    pub fn new(api: Api, server_stdin: ChildStdin) -> Self {
        Self {
            api,
            server_stdin: Arc::new(Mutex::new(server_stdin)),
        }
    }
}

pub struct MessageHandler;

#[async_trait]
impl Handler<Context> for MessageHandler {
    type Input = Message;
    type Output = ();

    async fn handle(&mut self, context: &Context, message: Self::Input) -> Self::Output {
        let text: String = match message {
            Message {
                data: MessageData::Text(text),
                ..
            } => text.data,
            _ => return,
        };

        let mut server_stdin = context.server_stdin.lock().await;
        server_stdin
            .write_all(format!("{}\n", text).as_bytes())
            .unwrap();
    }
}
