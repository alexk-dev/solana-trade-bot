use crate::entity::LimitOrderType;

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
        trade_type: String,
    },
    AwaitingTradeAmount {
        trade_type: String,
        token_address: String,
        token_symbol: String,
        price_in_sol: f64,
        price_in_usdc: f64,
    },
    AwaitingTradeConfirmation {
        trade_type: String,
        token_address: String,
        token_symbol: String,
        amount: f64,
        price_in_sol: f64,
        total_sol: f64,
    },
    AwaitingPriceTokenAddress,
    AwaitingLimitOrderType,
    AwaitingLimitOrderTokenAddress {
        order_type: LimitOrderType,
    },
    AwaitingLimitOrderPriceAndAmount {
        order_type: LimitOrderType,
        token_address: String,
        token_symbol: String,
        current_price_in_sol: f64,
        current_price_in_usdc: f64,
    },
    AwaitingLimitOrderConfirmation {
        order_type: LimitOrderType,
        token_address: String,
        token_symbol: String,
        price_in_sol: f64,
        amount: f64,
        total_sol: f64,
    },
}
