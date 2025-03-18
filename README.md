# Solana Telegram Wallet Bot

Телеграм-бот на Rust для управления Solana-кошельком и выполнения операций через Raydium DEX.

## Возможности

- Регистрация пользователей по Telegram ID
- Создание Solana-кошелька с генерацией мнемонической фразы
- Отображение адреса кошелька с QR-кодом для удобного пополнения
- Проверка баланса SOL и SPL-токенов
- Отправка SOL и SPL-токенов на другие адреса
- Обмен токенов через Raydium DEX (token swap)
- Получение информации о ценах на токены

## Технический стек

- **Язык**: Rust
- **Фреймворк для Telegram**: Teloxide
- **Блокчейн**: Solana SDK
- **База данных**: PostgreSQL
- **Работа с токенами**: SPL Token
- **DEX**: Raydium

## Архитектура

Проект имеет модульную структуру:

```
src/
├── main.rs                   # Основная точка входа
├── commands.rs               # Обработчики команд Telegram
├── db.rs                     # Взаимодействие с базой данных
├── model.rs                  # Модели данных
├── initialize_db.rs          # Инициализация базы данных
├── raydium.rs                # Интеграция с Raydium DEX
├── utils.rs                  # Вспомогательные функции
└── solana/                   # Взаимодействие с Solana
    ├── mod.rs                # Реэкспорт основных функций
    ├── client.rs             # Клиент Solana RPC
    ├── wallet.rs             # Работа с кошельками
    ├── utils.rs              # Утилиты для работы с Solana
    └── tokens/               # Работа с токенами
        ├── mod.rs            # Модуль токенов 
        ├── constants.rs      # Константы токенов
        ├── native.rs         # Работа с нативным SOL
        ├── spl.rs            # Работа с SPL-токенами
        └── transaction.rs    # Утилиты для транзакций
```

## Установка и настройка

### Требования

- Rust (последняя стабильная версия)
- PostgreSQL
- Доступ к Solana RPC (публичный или приватный)

### Шаги установки

1. Клонируйте репозиторий
   ```
   git clone https://github.com/username/solana-telegram-wallet.git
   cd solana-telegram-wallet
   ```

2. Создайте файл `.env` на основе `.env.example`
   ```
   cp .env.example .env
   ```

3. Отредактируйте `.env` и укажите:
    - Telegram Bot Token (получите от @BotFather)
    - Строку подключения к PostgreSQL
    - URL Solana RPC

4. Соберите и запустите бот
   ```
   cargo build --release
   ./target/release/solana_telegram_wallet
   ```

## Команды бота

- `/start` - Начать работу с ботом / зарегистрироваться
- `/create_wallet` - Создать новый Solana-кошелек
- `/address` - Показать адрес вашего кошелька и QR-код
- `/balance` - Проверить баланс вашего кошелька
- `/send` - Отправить SOL или токены на другой адрес
- `/swap <сумма> <исходный_токен> <целевой_токен> [<проскальзывание>%]` - Обменять токены
- `/price <символ_токена>` - Получить цену токена
- `/help` - Показать справку по командам

## Развертывание

Для запуска в продакшен-среде рекомендуется использовать Docker:

```
docker-compose up -d
```

## Безопасность

Обратите внимание:
- Приватные ключи хранятся в открытом виде в базе данных. В продакшен-версии следует использовать шифрование.
- Бот предназначен для образовательных целей и демонстрации возможностей Solana SDK.

## Лицензия

MIT License

## Контакты

По вопросам и предложениям обращайтесь [email@example.com]