# Telegram Downloader Rust

Effortlessly download Telegram media with this simple Rust-powered bot.

## Configuration:

- Copy the `.env.example` file to `.env`.
- Open the `.env` file and fill in the required variables

## Download a Release:

- Go to the [Releases page](https://github.com/rohitsangwan01/telegram_downloader_rust/releases) and download the appropriate archive for your operating system.
- Extract the archive contents to a directory of your choice.

## Build from Source (requires Rust):

- Make sure you have Rust installed on your system.
- Clone this repository: `git clone https://github.com/rohitsangwan01/telegram_downloader_rust.git`
- Navigate to the project directory: `cd telegram_downloader_rust`
- Build the bot: `cargo build --release`

## Running the Bot:

- From a Release:
  - Open a terminal and navigate to the directory where you extracted the release files. place `.env` file in the same directory.
  - Run the bot: `./telegram_bot` (or the appropriate executable name for your OS)
- From Source:
  - Run the bot: `cargo run`
- With pm2:

  - Make sure you have [pm2](https://pm2.keymetrics.io/) installed globally on your system. You can install it using `npm install -g pm2`. Run the bot using the following command, replacing `./target/release/telegram_bot` with the actual path to your bot's binary if it's different:

  ```bash
  pm2 start ./target/release/telegram_bot --name telegram_bot
  ```

## Using the Bot

1. Start a chat with your bot in Telegram.
2. Forward any media file (photos, videos, documents) to the bot.
3. The bot will download the media to your configured `DOWNLOAD_DIRECTORY`

## Permissions

If you encounter issues downloading to the specified directory, make sure you have the necessary permissions:

```sh
sudo chown -R $USER <DOWNLOAD_DIRECTORY>
```
