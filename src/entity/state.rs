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
    AwaitingSwapDetails,
}
