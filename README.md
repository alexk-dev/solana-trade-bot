# Solana Wallet Bot for Telegram

A powerful Telegram bot for Solana blockchain interactions, offering wallet management, token swaps, and crypto trading capabilities.

![Solana](https://img.shields.io/badge/Solana-black?style=for-the-badge&logo=solana)
![Telegram](https://img.shields.io/badge/Telegram-blue?style=for-the-badge&logo=telegram)
![Rust](https://img.shields.io/badge/Rust-orange?style=for-the-badge&logo=rust)

## Overview

Solana Wallet Bot is your all-in-one Telegram trading companion for the Solana ecosystem. Similar to popular bots like BONKBot, TrojanOnSolana, and BananaGun, but built with superior architecture and security in mind. This feature-rich bot enables users to create and manage Solana wallets, check balances, perform token swaps via Jupiter's powerful DEX aggregator, and execute trades—all directly from Telegram chats. Whether you're a casual trader or a DeFi power user, this bot provides a comprehensive solution for managing your Solana assets without ever leaving Telegram.

## Key Features

- **Wallet Management**: Create and manage Solana wallets securely
- **Balance Checking**: View SOL and SPL token balances with USD equivalents
- **QR Code Generation**: Generate QR codes for wallet addresses
- **Token Transfers**: Send SOL and SPL tokens to any Solana address
- **Token Swaps**: Swap between tokens using Jupiter DEX aggregator
- **Price Checking**: Get real-time token prices

## Commands

- `/start` - Start working with the bot
- `/create_wallet` - Create a new Solana wallet
- `/address` - Show your wallet address and QR code
- `/balance` - Check your wallet balance and token holdings
- `/send` - Send funds to another address
- `/swap <amount> <source_token> <target_token> [<slippage>%]` - Swap tokens via Jupiter DEX
- `/price <token_symbol>` - Get current token price
- `/help` - Show help message with command list

## Architecture

The project follows Clean Architecture principles with the VIPER pattern:

- **V**iew: Telegram message interfaces
- **I**nteractor: Business logic implementation
- **P**resenter: Transforms data between View and Interactor
- **E**ntity: Domain models
- **R**outer: Handles command routing and workflow

## Technical Stack

- **Language**: Rust
- **Telegram API**: Teloxide
- **Blockchain**: Solana (solana-sdk, solana-client)
- **Database**: PostgreSQL with SQLx
- **DEX Integration**: Jupiter Swap API

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
git clone https://github.com/alexk-dev/solana-wallet-bot.git
cd solana-wallet-bot
```

2. Set up the database:
```bash
sqlx database create
sqlx migrate run
```

3. Build and run:
```bash
cargo build --release
./target/release/solana-wallet-bot
```

## Docker Deployment

```bash
docker build -t solana-wallet-bot .
docker run -d --env-file .env --name solana-wallet-bot solana-wallet-bot
```

## License

This project is licensed under the Server Side Public License (SSPL) - see the [LICENSE](LICENSE) file for details.

## Disclaimer

This bot is provided as-is without any guarantees. Users are responsible for managing their own keys and funds. Always verify transactions before confirming them.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the project
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Acknowledgements

- [Solana](https://solana.com/)
- [Jupiter](https://jup.ag/)
- [Teloxide](https://github.com/teloxide/teloxide)
- The Rust community

---

Built with ❤️ for the Solana ecosystem.