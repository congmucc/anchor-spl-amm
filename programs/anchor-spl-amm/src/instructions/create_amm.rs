use anchor_lang::prelude::*;

use crate::{
    errors::*,
    state::Amm,
    models::{
        concentrated_liquidity::ConcentratedLiquidityConfig,
        price_impact::PriceImpactConfig,
        volatility::VolatilityConfig,
        fee_strategy::{FeeConfig, FeeStrategy},
    },
};

pub fn create_amm(ctx: Context<CreateAmm>, id: Pubkey, fee: u16) -> Result<()> {
    let amm = &mut ctx.accounts.amm;
    amm.id = id;
    amm.admin = ctx.accounts.admin.key();
    amm.fee = fee;
    
    // 初始化默认配置
    amm.fee_config = FeeConfig {
        strategy: FeeStrategy::Fixed, // 默认使用固定费率
        min_fee_bps: fee / 2,         // 最低费率为设定的一半
        max_fee_bps: fee * 2,         // 最高费率为设定的两倍
        base_fee_bps: fee,            // 基础费率即为设定值
        adjustment_factor: 500,       // 默认调整因子0.5
    };
    
    amm.price_impact_config = PriceImpactConfig::default();
    amm.volatility_config = VolatilityConfig::default();
    amm.concentrated_liquidity_config = ConcentratedLiquidityConfig::default();
    
    Ok(())
}

#[derive(Accounts)]
#[instruction(id: Pubkey, fee: u16)]
pub struct CreateAmm<'info> {
    #[account(
        init,
        payer = payer,
        space = Amm::LEN,
        seeds = [
            id.as_ref()
        ],
        bump,
        constraint = fee < 10000 @ TutorialError::InvalidFee,
    )]
    pub amm: Account<'info, Amm>,

    /// The admin of the AMM
    /// CHECK: Read only, delegatable creation
    pub admin: AccountInfo<'info>,

    /// The account paying for all rents
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Solana ecosystem accounts
    pub system_program: Program<'info, System>,
}