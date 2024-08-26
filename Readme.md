# Telegram Media Downloader

A telegram bot to download forwared media files

## Get Started

Make sure rust is installed in your system

configure `.env` file, checkout `.env.example` for variables

Run bot with command: `cargo run`

## To run on your server

Build with

```sh
cargo build --release
```

If using pm2, run with

```sh
pm2 start ./target/release/telegram_bot --name telegram_bot
```

If not able to download in given directory, Make sure you have proper permission,

```sh
sudo chown -R $USER DIRECTORY_PATH
```
