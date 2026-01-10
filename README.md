# TScrapingBot · [![Rust CI CD](https://github.com/ImMau14/tscrapingbot-rs/actions/workflows/rust-ci.yaml/badge.svg)](https://github.com/ImMau14/tscrapingbot-rs/actions/workflows/rust-ci.yaml)

My Telegram bot that analyzes web pages with AI, but now written in Rust. Use it on Telegram: [@TScrapingBot](https://t.me/TScrapingBot).

---

<img alt="image" src="https://github.com/user-attachments/assets/4a6d1d08-a3c8-4f54-ab0e-63bc8e913cc0" />

---

## Table of Contents

- [Technologies](#technologies)
- [Build Steps](#build)
- [Deployment Steps](#deployment-steps)
- [Code Quality](#code-quality)
- [Downloads](#downloads)
- [Why the bot changed](#why-the-bot-changed)
- [Contributing](#contributing)
- [Code of Conduct](#code-of-conduct)
- [License](#license)

---

## Technologies

TScrapingbot uses various technologies from the Rust ecosystem, but its main ones are:

- **[Teloxide](https://github.com/teloxide/teloxide)** - "An elegant Telegram bots framework for Rust".
- **[SQLx](https://github.com/launchbadge/sqlx)** - "The Rust SQL Toolkit".
- **[GroqAI](https://github.com/hiddenpath/groqai-rust-sdk)** - "A modern, type-safe Rust SDK for the Groq API".
- **[reqwest](https://github.com/seanmonstar/reqwest)** - "An easy and powerful Rust HTTP Client".
- **[Kuchiki](https://github.com/kuchiki-rs/kuchiki)** - "HTML/XML tree manipulation library for Rust".
- **[Axum](https://github.com/tokio-rs/axum)** - "HTTP routing and request-handling library for Rust".

Outside the Rust ecosystem, the bot uses a PostgreSQL database hosted on [Supabase](https://supabase.com).

---

## Build Steps

### Requirements

- **Rust** 1.90.0 or higher.

### Steps

1. **Clone the repository and enter it:**

   ```bash
   git clone https://github.com/ImMau14/tscrapingbot-rs.git
   cd tscrapingbot-rs
   ```

2. **Compile the binary.:**

   ```bash
   cargo build --release
   ```

   The binary is usually saved in the `target/release/` directory (within the repository).

---

## Deployment Steps

1. **Configure the environment variables**:

   To deploy the bot, either on a host or locally, its environment variables must be configured.

   |     Variable     | Description                                                               | Data Type |
   | :--------------: | ------------------------------------------------------------------------- | :-------: |
   |  `DATABASE_URL`  | Connection string for the database                                        |  string   |
   | `SCRAPEDO_TOKEN` | API token for the Scrapedo service                                        |  string   |
   | `TELOXIDE_TOKEN` | Bot token issued by Telexide                                              |  string   |
   |  `GROQ_API_KEY`  | API key for the Groq language‑model service                               |  string   |
   |      `PORT`      | TCP port the bot listens on (used when running locally or in a container) |  integer  |
   |    `HOSTING`     | Flag indicating whether the bot is running in production                  |  boolean  |
   |  `WEBHOOK_URL`   | Full HTTPS URL that Telegram will POST updates to                         |  string   |

   There is a template for the environment variables in [.env.template](.env.template).

2. **Run the project**:

   Use a compiled binary, or run the project using the `cargo run` command.

   ```bash
   # You can use the cargo run command (with the --release flag if you built with --release)...
   cargo run --release

   # or by running a compiled binary
   ./tscrapingbot-rs
   ```

---

## Code Quality

This project includes comprehensive code quality tools:

- **[Clippy](https://github.com/rust-lang/rust-clippy)** - "A collection of lints to catch common mistakes and improve your Rust code".
- **[rustfmt](https://github.com/rust-lang/rustfmt)** - "A tool for formatting Rust code according to style guidelines".
- **[SQLx-CLI](https://github.com/launchbadge/sqlx)** - "Command-line utility for SQLx, the Rust SQL toolkit".

---

## Downloads

You can get the latest release from this repository's [releases page](https://github.com/ImMau14/tscrapingbot-rs/releases). The binaries are automatically compiled through GitHub Actions and are available for the following platforms: Windows (32-bit and 64-bit) and Linux (64-bit only).

---

## Why the bot changed

I decided to implement this change abruptly in order to optimize the bot’s overall performance: speed up its responses, improve error tolerance, and increase the efficiency of SQL queries, without forgetting the errors that compile‑time typing prevents. I also want to learn more about Rust.

I switched from using Gemini to Groq models because, as of the date I’m writing this, the free limit for Gemini calls has been reduced to 20 calls per day.

The original bot repository still exists and will remain that way, as it might still be useful to someone: [ImMau14/TScrapingbot](https://github.com/ImMau14/TScrapingBot).

---

## Contributing

Please refer to the [CONTRIBUTING.md](CONTRIBUTING.md) file for guidelines on how to report issues, submit pull requests, and get started with development.

---

## Code of Conduct

By participating in this project, you agree to abide by the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
