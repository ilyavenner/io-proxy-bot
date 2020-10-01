use std::{
    fmt::Write as FmtWrite,
    io::{BufRead, BufReader, Read, Write},
    process::ChildStdin,
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use carapax::{
    methods::SendMessage,
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

const FILLING_DURATION: Duration = Duration::from_secs(1);

pub async fn stream_server_output<R>(
    mut reader: BufReader<R>,
    api: Api,
    master_chat_id: Integer,
) -> Result<(), Error>
where
    R: Read,
{
    loop {
        let text = collect_server_output(&mut reader)?;
        let message = SendMessage::new(master_chat_id, text);

        api.execute(message).await.context(ExecuteError)?;
    }
}

fn collect_server_output<R>(reader: &mut BufReader<R>) -> Result<String, Error>
where
    R: Read,
{
    let mut text = String::from("@\n");
    let beginning_instant = Instant::now();

    for line in reader.lines() {
        let line = line.context(IoError)?;
        writeln!(text, "{}", line).unwrap();

        if Instant::now().duration_since(beginning_instant) >= FILLING_DURATION {
            break;
        }
    }

    Ok(text)
}
