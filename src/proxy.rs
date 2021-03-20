/*
 * Copyright (c) 2021 Ilya Venner
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use snafu::ResultExt;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    time,
};

use crate::{
    bot::{self, Context},
    error::*,
};

/// A max length of the Telegram message in characters.
const TELEGRAM_MESSAGE_LENGTH: usize = 4096;

/// Streams executable output (`reader` parameter) into [master chat](Context::master_chat_id) with
/// [pause duration](Context::pause_duration) between messages.
///
/// If length of the collected executable output more than [TELEGRAM_MESSAGE_LENGTH], text will
/// separate to some messages.
pub async fn stream_executable_output<R>(
    context: &Context,
    mut reader: BufReader<R>,
) -> Result<(), Error>
where
    R: AsyncRead + Unpin,
{
    loop {
        let executable_output = collect_data_from_executable_stream(&context, &mut reader).await?;

        if let Some(text) = executable_output {
            log::debug!("collected `{}` character(s)", text.chars().count());

            let text_chars = text.chars().collect::<Vec<char>>();
            let messages = text_chars
                .chunks(TELEGRAM_MESSAGE_LENGTH)
                .map(|chunks| chunks.iter().collect::<String>());

            log::debug!(
                "the original message separated to `{}` chunk(s)",
                messages.len()
            );

            for message in messages {
                bot::send_message_to_master_chat(&context, &message).await?;
                time::sleep(context.pause_duration).await;
            }
        }
    }
}

/// Collects data from the executable every [pause duration](Context::pause_duration).
///
/// Returns non-empty string.
async fn collect_data_from_executable_stream<R>(
    context: &Context,
    reader: &mut BufReader<R>,
) -> Result<Option<String>, Error>
where
    R: AsyncRead + Unpin,
{
    let mut text = String::new();
    let timeout = time::timeout(context.pause_duration, async {
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(_) => {
                    if context
                        .filter_dictionary
                        .iter()
                        .any(|pattern| line.contains(pattern))
                    {
                        log::debug!("filtered `{}`", line.trim());
                    } else {
                        text.push_str(&line);
                    }
                }

                e @ Err(_) => break e,
            }
        }
    });

    match timeout.await {
        Ok(result) => result.map(|_| None).context(TimeoutCompletion),
        Err(_) => Ok(if text.is_empty() { None } else { Some(text) }),
    }
}
