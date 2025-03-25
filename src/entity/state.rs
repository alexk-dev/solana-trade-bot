use crate::entity::OrderType;

#[derive(Clone, Default, Debug)]
pub enum State {
    #[default]
    Start,
    AwaitingRecipientAddress,
    AwaitingAmount {
        recipient: String,
    },
    AwaitingConfirmation {
        recipient: String,
        amount: f64,
        token: String,
    },
    AwaitingTokenAddress {
        trade_type: OrderType,
    },
    AwaitingTradeAmount {
        trade_type: OrderType,
        token_address: String,
        token_symbol: String,
        price_in_sol: f64,
        price_in_usdc: f64,
    },
    AwaitingTradeConfirmation {
        trade_type: OrderType,
        token_address: String,
        token_symbol: String,
        amount: f64,
        price_in_sol: f64,
        total_sol: f64,
    },
    AwaitingPriceTokenAddress,
    AwaitingLimitOrderType,
    AwaitingLimitOrderTokenAddress {
        order_type: OrderType,
    },
    AwaitingLimitOrderPriceAndAmount {
        order_type: OrderType,
        token_address: String,
        token_symbol: String,
        current_price_in_sol: f64,
        current_price_in_usdc: f64,
    },
    AwaitingLimitOrderConfirmation {
        order_type: OrderType,
        token_address: String,
        token_symbol: String,
        price_in_sol: f64,
        amount: f64,
        total_sol: f64,
    },
    AwaitingSlippageInput,
    AwaitingWatchlistTokenAddress,
    AwaitingWithdrawTokenSelection,
    AwaitingWithdrawRecipientAddress {
        token_address: String,
        token_symbol: String,
        amount: f64,
        price_in_sol: f64,
        price_in_usdc: f64,
    },
    AwaitingWithdrawAmount {
        token_address: String,
        token_symbol: String,
        recipient: String,
        balance: f64,
        price_in_sol: f64,
        price_in_usdc: f64,
    },
    AwaitingWithdrawConfirmation {
        token_address: String,
        token_symbol: String,
        recipient: String,
        amount: f64,
        price_in_sol: f64,
        total_sol: f64,
        total_usdc: f64,
    },
}
