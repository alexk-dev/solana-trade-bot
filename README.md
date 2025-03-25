# Solana Wallet Bot for Telegram

A powerful Telegram bot for Solana blockchain interactions, offering wallet management, token swaps, and crypto trading capabilities.

![Solana](https://img.shields.io/badge/Solana-black?style=for-the-badge&logo=solana)
![Telegram](https://img.shields.io/badge/Telegram-blue?style=for-the-badge&logo=telegram)
![Rust](https://img.shields.io/badge/Rust-orange?style=for-the-badge&logo=rust)

## Overview

Solana Wallet Bot is your all-in-one Telegram trading companion for the Solana ecosystem. Similar to popular bots like BONKBot, TrojanOnSolana, and BananaGun, but built with superior architecture and security in mind. This feature-rich bot enables users to create and manage Solana wallets, check balances, perform token swaps via Jupiter's powerful DEX aggregator, and execute trades—all directly from Telegram chats. Whether you're a casual trader or a DeFi power user, this bot provides a comprehensive solution for managing your Solana assets without ever leaving Telegram.

## Project Status

🚀 **Beta** - Core functionality is complete and working, but some features may be refined based on user feedback.

## Key Features

- **Wallet Management**: Create and manage Solana wallets
- **Balance Checking**: View SOL and SPL token balances with USD equivalents
- **Token Transfers**: Send SOL and SPL tokens to any Solana address
- **Token Swaps**: Swap between tokens using Jupiter DEX aggregator
- **Price Checking**: Get real-time token prices
- **Limit Orders**: Create buy/sell limit orders that execute automatically when price conditions are met
- **Token Watchlist**: Track prices of your favorite tokens
- **Trade Management**: Buy and sell tokens with a simple interface

## Commands

- `/start` - Start working with the bot
- `/create_wallet` - Create a new Solana wallet
- `/menu` - Main menu (UI)
- `/help` - Show help message with command list

## Architecture

The project follows Clean Architecture principles with the VIPER pattern:

- **V**iew: Telegram message interfaces
- **I**nteractor: Business logic implementation
- **P**resenter: Transforms data between View and Interactor
- **E**ntity: Domain models
- **R**outer: Handles command routing and workflow

### Component Interaction

```
User (Telegram) → Router → Presenter → Interactor → Services (Solana/Jupiter)
                     ↑         ↓                         ↓
                    View ← Presenter                  Database
```

The bot uses a dependency injection container to manage service instances and their dependencies, making the code modular and testable.

## Security Considerations

- **Private Keys**: Private keys are stored plain (non encrypted) in the database
- **No External API Keys**: The bot doesn't require users to provide external API keys
- **Confirmations**: All financial transactions require explicit user confirmation
- **Sandboxed Operations**: Each user's operations are isolated
- **Transparent Transactions**: Transaction signatures are provided for all blockchain operations

## Technical Stack

- **Language**: Rust
- **Telegram API**: Teloxide
- **Blockchain**: Solana (solana-sdk, solana-client)
- **Database**: PostgreSQL with SQLx
- **DEX Integration**: Jupiter Swap API
- **Background Services**: Tokio for asynchronous processing

## Installation

### Prerequisites

- Rust 1.80+
- PostgreSQL
- Docker (optional, for containerized deployment)

### Environment Variables

Create a `.env` file with:

```
TELEGRAM_BOT_TOKEN=your_telegram_bot_token
DATABASE_URL=postgres://username:password@localhost/dbname
SOLANA_RPC_URL=your_solana_rpc_url
```

### Setup

1. Clone the repository:
```bash
git clone https://github.com/alexk-dev/solana-trade-bot.git
cd solana-trade-bot
```

2. Set up the database:
```bash
sqlx database create
sqlx migrate run
```

3. Build and run:
```bash
cargo build --release
./target/release/solana-trade-bot
```

## Docker Deployment

```bash
docker build -t solana-trade-bot .
docker run -d --env-file .env --name solana-trade-bot solana-trade-bot
```

## Development Setup

For developers who want to contribute:

1. Set up a development environment:
```bash
cargo install sqlx-cli
cargo install cargo-watch
```

2. Create a test database:
```bash
sqlx database create --database-url postgres://username:password@localhost/solana_bot_test
sqlx migrate run --database-url postgres://username:password@localhost/solana_bot_test
```

3. Run tests:
```bash
cargo test
```

4. Run with hot reloading during development:
```bash
cargo watch -x run
```

## Troubleshooting

### Common Issues

- **RPC Rate Limits**: If you encounter errors related to Solana RPC calls, you might be hitting rate limits. Consider using a paid RPC provider.
- **Database Connection**: Ensure your PostgreSQL service is running and the DATABASE_URL is correctly formatted.
- **Token Not Found**: When using custom tokens, make sure you're using the mint address and not the token account address.
- **Insufficient Funds**: For trades and swaps, ensure you have enough SOL to cover both the transaction and network fees.

### Logs

Check the application logs for detailed error messages:
```bash
RUST_LOG=info ./target/release/solana-trade-bot
```

For more verbose logging:
```bash
RUST_LOG=debug ./target/release/solana-trade-bot
```

## License
Distributed under the business friendly [MIT license](https://opensource.org/licenses/MIT).

## Disclaimer

This bot is provided as-is without any guarantees. Users are responsible for managing their own keys and funds. Always verify transactions before confirming them. The bot is not responsible for any financial losses resulting from market volatility, user error, or software bugs.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the project
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Contribution Guidelines

- Follow the existing code style
- Add tests for new features
- Update documentation for any changes
- Ensure all tests pass before submitting a PR

## Acknowledgements

- [Solana](https://solana.com/)
- [Jupiter](https://jup.ag/)
- [Teloxide](https://github.com/teloxide/teloxide)
- The Rust community

---

Built with ❤️ for the Solana ecosystem.