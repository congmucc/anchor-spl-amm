use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};
use fixed::types::I64F64;

use crate::{
    constants::AUTHORITY_SEED,
    errors::*,
    state::{Amm, Pool},
    models::fee_strategy::{FeeCalculator, FeeStrategy},
    models::price_impact::PriceImpactCalculator,
    models::volatility::VolatilityTracker,
};

// 将指令拆分为两部分
pub fn swap_exact_tokens_for_tokens(
    ctx: Context<SwapExactTokensForTokens>,
    swap_a: bool, // true if swapping A for B, false if swapping B for A 
    input_amount: u64,
    min_output_amount: u64,
) -> Result<()> {
    // 调用处理函数
    swap_exact_tokens_for_tokens_process(ctx, swap_a, input_amount, min_output_amount)
}

// 处理交换逻辑
fn swap_exact_tokens_for_tokens_process(
    ctx: Context<SwapExactTokensForTokens>,
    swap_a: bool,
    input_amount: u64,
    min_output_amount: u64,
) -> Result<()> {
    // 1. Prevent depositing assets the depositor does not own
    let input = if swap_a && input_amount > ctx.accounts.trader_token_accounts.trader_account_a.amount {
        ctx.accounts.trader_token_accounts.trader_account_a.amount
    } else if !swap_a && input_amount > ctx.accounts.trader_token_accounts.trader_account_b.amount {
        ctx.accounts.trader_token_accounts.trader_account_b.amount
    } else {
        input_amount
    };

    // 2. Apply trading fee, used to compute the output
    let amm = &ctx.accounts.amm;
    
    // 使用动态费用计算器获取当前适用的费率
    let fee_rate_bps = if amm.fee_config.strategy != FeeStrategy::Fixed {
        // 获取当前波动率，用于调整费用
        let volatility = ctx.accounts.pool.volatility_tracker.get_volatility().to_num::<u16>();
        
        // 基于当前市场状况计算动态费率
        FeeCalculator::get_fee_rate_bps(
            &amm.fee_config, 
            input,
            if swap_a { ctx.accounts.pool_token_accounts.pool_account_a.amount } else { ctx.accounts.pool_token_accounts.pool_account_b.amount },
            if swap_a { ctx.accounts.pool_token_accounts.pool_account_b.amount } else { ctx.accounts.pool_token_accounts.pool_account_a.amount },
            Some(volatility)
        )
    } else {
        amm.fee // 使用默认固定费率
    };
    
    // 应用计算得到的费率
    let taxed_input = input - input * fee_rate_bps as u64 / 10000;
    
    // 3. Compute the output amount and check price impact
    let pool_a = &ctx.accounts.pool_token_accounts.pool_account_a;
    let pool_b = &ctx.accounts.pool_token_accounts.pool_account_b;
    
    // 计算价格影响（滑点）
    let price_impact = if swap_a {
        PriceImpactCalculator::calculate_price_impact(
            &amm.price_impact_config,
            input,
            0, // 暂时设为0，后面会计算实际输出
            pool_a.amount, 
            pool_b.amount
        )
    } else {
        PriceImpactCalculator::calculate_price_impact(
            &amm.price_impact_config,
            input,
            0, // 暂时设为0，后面会计算实际输出
            pool_b.amount, 
            pool_a.amount
        )
    };
    
    // 检查价格影响是否在可接受范围内
    if !PriceImpactCalculator::is_price_impact_acceptable(
        &amm.price_impact_config,
        price_impact
    ) {
        return err!(TutorialError::PriceImpactTooHigh);
    }
    
    // 计算输出金额
    let output = if swap_a {
        I64F64::from_num(taxed_input)
            .checked_mul(I64F64::from_num(pool_b.amount))
            .unwrap()
            .checked_div(
                I64F64::from_num(pool_a.amount)
                .checked_add(I64F64::from_num(taxed_input))
                .unwrap(),
            )
            .unwrap()
    } else {
        I64F64::from_num(taxed_input)
            .checked_mul(I64F64::from_num(pool_a.amount))
            .unwrap()
            .checked_div(
                I64F64::from_num(pool_b.amount)
                .checked_add(I64F64::from_num(taxed_input))
                .unwrap(),
            )
            .unwrap()
    }
    .to_num::<u64>();

    // 应用滑点调整，确保输出不低于用户设定的最小值
    let adjusted_output = PriceImpactCalculator::adjust_output_for_slippage(
        &amm.price_impact_config,
        output, 
        price_impact
    );

    // 4. Slip point protection
    if adjusted_output < min_output_amount {
        return err!(TutorialError::OutputTooSmall);
    }
    
    // 检查交易是否对用户有利
    if !PriceImpactCalculator::is_trade_beneficial(
        I64F64::from_num(input),
        I64F64::from_num(adjusted_output),
        I64F64::from_num(fee_rate_bps) / I64F64::from_num(10000)
    ) {
        return err!(TutorialError::TradeNotBeneficial);
    }

    // 5. Compute the invariant before the trade
    let invariant = pool_a.amount * pool_b.amount;

    // 6. Swap the tokens
    let authority_bump = ctx.bumps.pool_authority;
    let authority_seeds = &[
        &ctx.accounts.pool.amm.to_bytes(),
        &ctx.accounts.mint_a.key().to_bytes(),
        &ctx.accounts.mint_b.key().to_bytes(),
        AUTHORITY_SEED,
        &[authority_bump],
    ];
    let signer_seeds = &[&authority_seeds[..]];
    if swap_a {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.trader_token_accounts.trader_account_a.to_account_info(),
                    to: ctx.accounts.pool_token_accounts.pool_account_a.to_account_info(),
                    authority: ctx.accounts.trader.to_account_info(),
                },
                
            ), input,
        )?;
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_token_accounts.pool_account_b.to_account_info(),
                    to: ctx.accounts.trader_token_accounts.trader_account_b.to_account_info(),
                    authority: ctx.accounts.pool_authority.to_account_info(),
                },
                signer_seeds,
            ),
            adjusted_output,
        )?;
    } else {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.trader_token_accounts.trader_account_b.to_account_info(),
                    to: ctx.accounts.pool_token_accounts.pool_account_b.to_account_info(),
                    authority: ctx.accounts.trader.to_account_info(),
                },
            ),
            input,
        )?;
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_token_accounts.pool_account_a.to_account_info(),
                    to: ctx.accounts.trader_token_accounts.trader_account_a.to_account_info(),
                    authority: ctx.accounts.pool_authority.to_account_info(),
                },
                signer_seeds,
            ),
            adjusted_output,
        )?;
    }

    msg!(
        "Traded {} tokens ({} after fees) for {} (Price impact: {} bps)",
        input,
        taxed_input,
        adjusted_output,
        price_impact
    );

    // 7. Verify the invariant still holds
    // We tolerate if the new invariant is higher because it means a rounding error for LPs
    ctx.accounts.pool_token_accounts.pool_account_a.reload()?;
    ctx.accounts.pool_token_accounts.pool_account_b.reload()?;
    if invariant > ctx.accounts.pool_token_accounts.pool_account_a.amount * ctx.accounts.pool_token_accounts.pool_account_b.amount {
        return err!(TutorialError::InvariantViolated);
    }
    
    // 8. 更新波动率追踪器
    let current_price = if swap_a {
        I64F64::from_num(ctx.accounts.pool_token_accounts.pool_account_a.amount) / I64F64::from_num(ctx.accounts.pool_token_accounts.pool_account_b.amount)
    } else {
        I64F64::from_num(ctx.accounts.pool_token_accounts.pool_account_b.amount) / I64F64::from_num(ctx.accounts.pool_token_accounts.pool_account_a.amount)
    };
    
    // 更新价格样本和计算波动率
    let mut pool = &mut ctx.accounts.pool;
    pool.volatility_tracker.update_price_sample(
        current_price,
        Clock::get()?.unix_timestamp,
        &ctx.accounts.amm.volatility_config
    );
    
    Ok(())
}


#[derive(Accounts)]
pub struct SwapExactTokensForTokens<'info> {
    #[account(
        seeds = [
            amm.id.as_ref()
        ],
        bump,
    )]
    pub amm: Box<Account<'info, Amm>>,

    #[account(
        mut,
        seeds = [
            pool.amm.as_ref(),
            pool.mint_a.key().as_ref(),
            pool.mint_b.key().as_ref(),
        ],
        bump,
        has_one = amm,
        has_one = mint_a,
        has_one = mint_b,
    )]
    pub pool: Box<Account<'info, Pool>>,

    /// CHECK: Read only authority
    #[account(
        seeds = [
            pool.amm.as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            AUTHORITY_SEED,
        ],
        bump,
    )]
    pub pool_authority: AccountInfo<'info>,

    /// The account doing the swap
    pub trader: Signer<'info>,

    pub mint_a: Box<Account<'info, Mint>>,

    pub mint_b: Box<Account<'info, Mint>>,

    // 分离池账户和交易者账户到单独的结构体中
    pub pool_token_accounts: PoolTokenAccounts<'info>,
    
    pub trader_token_accounts: TraderTokenAccounts<'info>,

    /// Solana ecosystem accounts
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

// 池代币账户
#[derive(Accounts)]
pub struct PoolTokenAccounts<'info> {
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = pool_authority,
    )]
    pub pool_account_a: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = pool_authority,
    )]
    pub pool_account_b: Box<Account<'info, TokenAccount>>,
    
    /// CHECK: Used in constraints
    pub mint_a: AccountInfo<'info>,
    
    /// CHECK: Used in constraints
    pub mint_b: AccountInfo<'info>,
    
    /// CHECK: Used in constraints
    pub pool_authority: AccountInfo<'info>,
}

// 交易者代币账户
#[derive(Accounts)]
pub struct TraderTokenAccounts<'info> {
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_a,
        associated_token::authority = trader,
    )]
    pub trader_account_a: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_b,
        associated_token::authority = trader,
    )]
    pub trader_account_b: Box<Account<'info, TokenAccount>>,
    
    /// CHECK: Used in constraints
    pub mint_a: AccountInfo<'info>,
    
    /// CHECK: Used in constraints
    pub mint_b: AccountInfo<'info>,
    
    /// CHECK: Used in constraints
    pub trader: AccountInfo<'info>,
    
    /// The account paying for all rents
    #[account(mut)]
    pub payer: Signer<'info>,
    
    // 必须添加这些程序账户以实现init_if_needed约束
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}