use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct Amm {
    /// The primary key of the AMM
    pub id: Pubkey,

    /// Account that has admin authority over the AMM
    pub admin: Pubkey,

    /// The LP fee taken on each trade, in basis points
    pub fee: u16,
}

impl Amm {
    pub const LEN: usize = 8 + 32 + 32 + 2;
}

#[account]
pub struct Pool {
    /// Primary key of the AMM
    pub amm: Pubkey,

    /// Mint of token A
    pub mint_a: Pubkey,

    /// Mint of token B
    pub mint_b: Pubkey,
    
    /// 初始价格，用于价格参考
    pub initial_price: u64,
}

impl Pool {
    pub const LEN: usize = 8 + 32 + 32 + 32 + 8;
}

impl Default for Pool {
    fn default() -> Self {
        Self {
            amm: Pubkey::default(),
            mint_a: Pubkey::default(),
            mint_b: Pubkey::default(),
            initial_price: 0,
        }
    }
}