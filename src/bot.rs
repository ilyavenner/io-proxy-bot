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

use std::{collections::HashSet, sync::Arc, time::Duration};

use async_trait::async_trait;
use carapax::{
    methods::SendMessage,
    types::{Integer, Message, MessageData},
    Api, Handler,
};
use snafu::ResultExt;
use tokio::{io::AsyncWriteExt, process::ChildStdin, sync::Mutex};

use crate::error::*;

/// A bot context.
pub struct Context {
    /// A telegram bot [API](carapax::Api).
    pub api: Api,

    /// A master chat ID from which the bot will process messages.
    pub master_chat_id: Integer,

    /// A `stdin` of the given executable.
    pub executable_stdin: Mutex<ChildStdin>,

    /// A pause duration between two sending messages.
    pub pause_duration: Duration,

    /// A strings which will ignored from proxying `stdout`.
    pub filter_dictionary: HashSet<String>,
}

/// A special symbol which from comments start (such strings will be ignored).
const COMMENT_PATTERN: char = '#';

/// A handler which proxy [executable stdin](Context::executable_stdin) only from messages in the
/// [master chat](Context::master_chat_id).
pub struct MessageHandler;

#[async_trait]
impl Handler<Arc<Context>> for MessageHandler {
    type Input = Message;
    type Output = Result<(), Error>;

    async fn handle(&mut self, context: &Arc<Context>, message: Self::Input) -> Self::Output {
        let message_text = extract_master_chat_message_text(context, message);

        if let Some(text) = message_text {
            let text_lines = text
                .lines()
                .filter(|line| !line.starts_with(COMMENT_PATTERN));

            let mut executable_stdin = context.executable_stdin.lock().await;

            for line in text_lines {
                executable_stdin
                    .write_all(format!("{}\n", line).as_bytes())
                    .await
                    .context(WriteExecutableStdIn)?;
            }
        }

        Ok(())
    }
}

/// Extracts text of the incoming message from the [master chat](Context::master_chat_id).
///
/// Returns non-empty string.
fn extract_master_chat_message_text(context: &Context, message: Message) -> Option<String> {
    if message.get_chat_id() == context.master_chat_id {
        match message.data {
            MessageData::Text(text) => Some(text.data),
            _ => None,
        }
    } else {
        None
    }
}

/// Sends the message to the [master chat](Context::master_chat_id).
pub async fn send_message_to_master_chat(context: &Context, text: &str) -> Result<Message, Error> {
    let message = SendMessage::new(context.master_chat_id, text);
    context.api.execute(message).await.context(SendMessage)
}
