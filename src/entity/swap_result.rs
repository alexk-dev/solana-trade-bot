pub struct SwapResult {
    pub source_token: String,
    pub target_token: String,
    pub amount_in: f64,
    pub amount_out: f64,
    pub signature: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}
