use anchor_lang::prelude::*;
use fixed::types::I64F64;

use crate::models::{
    concentrated_liquidity::ConcentratedLiquidityConfig,
    price_impact::PriceImpactConfig,
    volatility::{VolatilityConfig, VolatilityTracker},
    fee_strategy::{FeeStrategy, FeeConfig},
};

#[account]
#[derive(Default)]
pub struct Amm {
    /// The primary key of the AMM
    pub id: Pubkey,

    /// Account that has admin authority over the AMM
    pub admin: Pubkey,

    /// The LP fee taken on each trade, in basis points
    pub fee: u16,
    
    /// 动态费用配置
    pub fee_config: FeeConfig,
    
    /// 价格影响保护配置
    pub price_impact_config: PriceImpactConfig,
    
    /// 波动率配置
    pub volatility_config: VolatilityConfig,
    
    /// 集中流动性配置
    pub concentrated_liquidity_config: ConcentratedLiquidityConfig,
}

impl Amm {
    // 8字节discriminator + id + admin + fee + fee_config + price_impact_config + volatility_config + concentrated_liquidity_config
    pub const LEN: usize = 8 + 32 + 32 + 2 + 9 + 5 + 26 + 17;
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
    
    /// 波动率追踪器
    pub volatility_tracker: VolatilityTracker,
}

impl Pool {
    // 8字节discriminator + amm + mint_a + mint_b + initial_price + volatility_tracker
    pub const LEN: usize = 8 + 32 + 32 + 32 + 8 + 
        (24 * 16 + 24 * 8 + 1 + 16 + 16); // VolatilityTracker的大小
}

impl Default for Pool {
    fn default() -> Self {
        Self {
            amm: Pubkey::default(),
            mint_a: Pubkey::default(),
            mint_b: Pubkey::default(),
            initial_price: 0,
            volatility_tracker: VolatilityTracker::default(),
        }
    }
}