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

use std::{collections::HashSet, io, path::PathBuf, sync::Arc, time::Duration};

use snafu::{ResultExt, Snafu};
use teloxide::types::ParseMode;
use teloxide::{
    adaptors::{AutoSend, DefaultParseMode},
    prelude::*,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::ChildStdin,
    sync::oneshot,
    sync::Mutex,
    time,
};

const INITIALIZATION_MESSAGE: &str = concat!(
    "You are using `",
    std::env!("CARGO_PKG_NAME"),
    "`, version `",
    std::env!("CARGO_PKG_VERSION"),
    "`.\n\nThis program is free software: you can redistribute it and/or modify it under the \
        terms of the \
        [GNU Affero General Public License](https://www.gnu.org/licenses/agpl-3.0.html) \
        as published by the Free Software Foundation, either version 3 of the License, or (at your \
        option) any later version.\n\nThe original source \
        code is available in [this repository](https://github.com/ilyavenner/io-proxy-bot/)."
);

/// A max length of the Telegram message in characters.
const TELEGRAM_MESSAGE_LENGTH: usize = 4096;

/// A special symbol which from comments start (such strings will be ignored).
const COMMENT_PATTERN: char = '#';

/// A service context type.
type Cx = UpdateWithCx<Arc<DefaultParseMode<AutoSend<Bot>>>, Message>;

/// A bot service.
pub struct Service {
    /// A master chat ID from which the bot will process messages.
    master_chat_id: i64,

    /// A `stdin` of the given executable.
    executable_stdin: Mutex<ChildStdin>,

    /// A `stdout` data receiver.
    executable_stdout: oneshot::Receiver<String>,

    /// A `stderr` data receiver.
    executable_stderr: oneshot::Receiver<String>,

    /// A pause duration between two sending messages.
    pause_duration: Duration,

    /// A strings which will ignored from proxying `stdout`.
    filter_dictionary: HashSet<String>,
}

impl Service {
    pub fn new(
        master_chat_id: i64,
        executable_stdin: Mutex<ChildStdin>,
        executable_stdout: oneshot::Receiver<String>,
        executable_stderr: oneshot::Receiver<String>,
        pause_duration: Duration,
        filter_dictionary: HashSet<String>,
    ) -> Self {
        Self {
            master_chat_id,
            executable_stdin,
            executable_stdout,
            executable_stderr,
            pause_duration,
            filter_dictionary,
        }
    }

    pub async fn run<T>(self, token: T)
    where
        T: Into<String>,
    {
        let token = token.into();

        // NOTE: use deprecated parse mod because `teloxide` does not
        //       call `teloxide::utils::markdown::escape` itself
        let bot = Bot::new(token).auto_send().parse_mode(
            #[allow(deprecated)]
            ParseMode::Markdown,
        );

        let bot = Arc::new(bot);
        let service = Arc::new(self);

        let handler = move |cx| {
            let service = Arc::clone(&service);

            async move {
                service.send_initialization_message(&cx).await?;
                service.message_handler(cx).await
            }
        };

        teloxide::repl(bot, handler).await;
    }

    async fn message_handler(&self, cx: Cx) -> Result<(), Error> {
        let message = &cx.update;
        let message_text = self.extract_master_chat_message_text(message);

        if let Some(text) = message_text {
            let text_lines = text
                .lines()
                .filter(|line| !line.starts_with(COMMENT_PATTERN));

            let mut executable_stdin = self.executable_stdin.lock().await;

            for line in text_lines {
                executable_stdin
                    .write_all(format!("{}\n", line).as_bytes())
                    .await
                    .context(WriteExecutableStdIn)?;
            }
        }

        Ok(())
    }

    /// Streams executable output (`reader` parameter) into [master chat](Context::master_chat_id) with
    /// [pause duration](Context::pause_duration) between messages.
    ///
    /// If length of the collected executable output more than [TELEGRAM_MESSAGE_LENGTH], text will
    /// separate to some messages.
    async fn stream_executable_output<R>(
        &self,
        cx: &Cx,
        mut reader: BufReader<R>,
    ) -> Result<(), Error>
    where
        R: AsyncRead + Unpin,
    {
        loop {
            let executable_output = self
                .collect_data_from_executable_stream(&mut reader)
                .await?;

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
                    cx.answer(&message).await.context(SendMessage)?;
                    time::sleep(self.pause_duration).await;
                }
            }
        }
    }

    /// Collects data from the executable every [pause duration](Context::pause_duration).
    ///
    /// Returns non-empty string.
    async fn collect_data_from_executable_stream<R>(
        &self,
        reader: &mut BufReader<R>,
    ) -> Result<Option<String>, Error>
    where
        R: AsyncRead + Unpin,
    {
        let mut text = String::new();
        let timeout = time::timeout(self.pause_duration, async {
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line).await {
                    Ok(_) => {
                        if self
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

    /// Sends the initialization message to the [master chat](Context::master_chat_id).
    async fn send_initialization_message(&self, cx: &Cx) -> Result<(), Error> {
        cx.requester
            .send_message(self.master_chat_id, INITIALIZATION_MESSAGE)
            .await
            .context(SendingInitializationMessage)?;

        Ok(())
    }

    /// Extracts text of the incoming message from the [master chat](Context::master_chat_id).
    ///
    /// Returns non-empty string.
    fn extract_master_chat_message_text<'m>(&self, message: &'m Message) -> Option<&'m str> {
        if let Some(user) = message.from() {
            if user.id == self.master_chat_id {
                return message.text();
            }
        }

        None
    }
}

/// A service error.
#[derive(Debug, Snafu)]
pub enum Error {
    /// Indicates that executable could not be run.
    #[snafu(display("Cannot spawn executable `{}`.", path_to_executable.display()))]
    CannotSpawnProcess {
        /// A path to executable.
        path_to_executable: PathBuf,

        /// Source error.
        source: io::Error,
    },

    /// Indicate the running executable have not `stdin` stream.
    #[snafu(display(
    "Cannot find `stdin` stream in the given executable: `{}`.", path_to_executable.display()
    ))]
    NoStdIn {
        /// A path to executable.
        path_to_executable: PathBuf,
    },

    /// Indicate the running executable have not `stdout` stream.
    #[snafu(display(
    "Cannot find `stdout` stream in the given executable: `{}`.", path_to_executable.display()
    ))]
    NoStdOut {
        /// A path to executable.
        path_to_executable: PathBuf,
    },

    /// Indicate the running executable have not `stderr` stream.
    #[snafu(display(
    "Cannot find `stderr` stream in the given executable: `{}`.", path_to_executable.display()
    ))]
    NoStdErr {
        /// A path to executable.
        path_to_executable: PathBuf,
    },

    /// Indicates that data cannot be written to the `stdin` stream.
    #[snafu(display("Cannot write to `stdin`."))]
    WriteExecutableStdIn {
        /// Source error.
        source: io::Error,
    },

    /// Indicates that bot cannot send the message.
    #[snafu(display("Cannot send text message."))]
    SendMessage {
        /// Source error.
        source: teloxide::RequestError,
    },

    /// Indicates that bot cannot send the initialization message.
    #[snafu(display("Cannot send the initialization message."))]
    SendingInitializationMessage {
        /// Source error.
        source: teloxide::RequestError,
    },

    /// Indicates that [timeout](tokio::time::timeout) future completed unexpectedly.
    #[snafu(display("An unexpected timeout future completion."))]
    TimeoutCompletion {
        /// Source error.
        source: io::Error,
    },
}
