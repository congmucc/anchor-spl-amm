use anchor_lang::prelude::*;

#[error_code]
pub enum TutorialError {
    #[msg("Invalid fee value")]
    InvalidFee,

    #[msg("Invalid mint for the pool")]
    InvalidMint,

    #[msg("Depositing too little liquidity")]
    DepositTooSmall,

    #[msg("Output is below the minimum expected")]
    OutputTooSmall,

    #[msg("Invariant does not hold")]
    InvariantViolated,
    
    #[msg("The price impact exceeds the maximum allowed slippage")]
    ExcessiveSlippage,
    
    #[msg("Volatility is too high")]
    ExcessiveVolatility,
    
    #[msg("Invalid price configuration")]
    InvalidPriceConfig,
    
    #[msg("Price impact is too high")]
    PriceImpactTooHigh,
    
    #[msg("Trade is not beneficial to the user")]
    TradeNotBeneficial,
}