# IO Proxy Bot

> Версия [`README.ME` на русском](./README_ru.md).

This bot can redirect messages from a configured chat to the `stdin` of a proxying application.

## Installation

Follow a few simple steps:

```shell
# setup `rust` toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# clone this repository
git clone https://github.com/ilyavenner/io-proxy-bot 

# install executable
cd io-proxy-bot && cargo install --path=.
```

## Setting up the bot account

If you do not have a bot, create one at [BotFather](https://t.me/BotFather). If you want to use the
bot in group chats, enable this feature in the bot settings and do not forget to disable the private
mode (additional access to message of participants, not just commands).

To run the bot, you should specify the following required parameters:

* `token` - bot token from [BotFather](https://t.me/BotFather);
* `chat` - chat ID where the bot will work (other chats will be ignored);
* `executable` - path to the executable that will be proxying.

Example:

```shell
io-proxy-bot --token 'bot-token:88005553535' --chat '42424242' --executable '/bin/cat'
```

This example will create the simplest echo bot. Please note that the bot must have rights to send
messages in the specified chat. For direct dialogue with the bot, you have to write first. To use
this bot in a group simply add it to the group.

> __NOTE__
>
> You can get your account ID at [@chatid_echo_bot](https://t.me/chatid_echo_bot).
> To find out the group ID just add him to the chat.

### Setting up a pause between messages

If the application sends too much text, this may lead to exceeding Telegram limits. Using the
`--pause-duration` option, you can specify the interval between two messages (default 2 seconds).
For example to set 10 seconds interval, use the command below:

```shell
io-proxy-bot ... --pause-duration 10s
```

### Strings filtering

The `--filter-dictionary` option can be used to customize the strings that will be excluded from the
sending messages. Useful when you need to filter out unnecessary text. Note that strings are
searched by full match and case-sensitive.

```shell
io-proxy-bot ... --filter-dictionary 'Dumping data...' 'Reboot...'
```

## Bot using

This section describes some features of this bot.

### Comments

All messages which sent to the bot are forward to the `stdin` without any changes. Lines starting
with a `#` are perceived as comments and will be ignored by the bot.

### Scripts

If the proxying application executes commands line by line, relying on the information from the
above paragraph, you can organize your test in multiple lines. The example below is a script from
pseudo-commands with using comments:

```shell
# add a player to the whitelist
whitelist add somebody

# set the added player role to admin 
setrole somebody admin
```

## License

This program is free software: you can redistribute it and/or modify it under the terms of the GNU
Affero General Public License as published by the Free Software Foundation, either version 3 of the
License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero
General Public License for more details.
