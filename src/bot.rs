use std::{fmt::Write as FmtWrite, sync::Arc, time::Duration};

use async_trait::async_trait;
use carapax::{
    methods::SendMessage,
    types::{Integer, Message, MessageData, MessageKind, SupergroupChat},
    Api, Handler,
};
use snafu::ResultExt;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::ChildStdin,
    sync::Mutex,
    time,
};

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
            .await
            .context(IoError)?;

        Ok(())
    }
}

const PAUSE_DURATION: Duration = Duration::from_secs(2);
const MESSAGE_LENGTH: usize = 4096;

pub async fn stream_server_output<R>(
    mut reader: BufReader<R>,
    api: Api,
    master_chat_id: Integer,
) -> Result<(), Error>
where
    R: AsyncRead + Unpin,
{
    loop {
        let text = collect_server_output(&mut reader).await?;

        match text {
            None => continue,
            Some(text) => {
                let text_chars = text.chars().collect::<Vec<char>>();
                let messages = text_chars
                    .chunks(MESSAGE_LENGTH)
                    .map(|chunk| chunk.iter().collect::<String>());

                for message in messages {
                    let message = SendMessage::new(master_chat_id, message);
                    api.execute(message).await.context(ExecuteError)?;

                    time::delay_for(PAUSE_DURATION).await;
                }
            }
        }
    }
}

async fn collect_server_output<R>(reader: &mut BufReader<R>) -> Result<Option<String>, Error>
where
    R: AsyncRead + Unpin,
{
    let mut text = String::new();
    let timeout = time::timeout(PAUSE_DURATION, async {
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(_) => write!(text, "{}", line).unwrap(),
                e @ Err(_) => break e,
            }
        }
    });

    match timeout.await {
        Ok(result) => result.map(|_| None).context(IoError),
        Err(_) => Ok(if text.is_empty() { None } else { Some(text) }),
    }
}
