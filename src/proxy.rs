use std::{fmt::Write, time::Duration};

use snafu::ResultExt;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    time,
};

use crate::{
    bot::{send_message_to_master_chat, Context},
    error::*,
};

const PAUSE_DURATION: Duration = Duration::from_secs(2);
const MESSAGE_LENGTH: usize = 4096;
const SKIPPING_PATTERN: &str = "Running AutoCompaction...";

pub async fn stream_server_output<R>(
    context: Context,
    mut reader: BufReader<R>,
) -> Result<(), Error>
where
    R: AsyncRead + Unpin,
{
    loop {
        let server_output = collect_server_output(&mut reader, PAUSE_DURATION).await?;

        if let Some(text) = server_output {
            let text_chars = text.chars().collect::<Vec<char>>();
            let messages = text_chars
                .chunks(MESSAGE_LENGTH)
                .map(|chunk| chunk.iter().collect::<String>());

            for message in messages {
                send_message_to_master_chat(&context, &message).await?;
                time::sleep(PAUSE_DURATION).await;
            }
        }
    }
}

async fn collect_server_output<R>(
    reader: &mut BufReader<R>,
    pause_duration: Duration,
) -> Result<Option<String>, Error>
where
    R: AsyncRead + Unpin,
{
    let mut text = String::new();
    let timeout = time::timeout(pause_duration, async {
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(_) => {
                    if line.contains(SKIPPING_PATTERN) {
                        continue;
                    }

                    write!(text, "{}", line).unwrap()
                }

                e @ Err(_) => break e,
            }
        }
    });

    match timeout.await {
        Ok(result) => result.map(|_| None).context(IoError),
        Err(_) => Ok(if text.is_empty() { None } else { Some(text) }),
    }
}
