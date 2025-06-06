#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;

mod constants;
mod errors;
mod instructions;
mod models;
mod state;

use instructions::*;

declare_id!("5pCZ4MZ1BU4FSx7zWxCtAQ5vyhxWLikoZpLV6biPG8Rj");


#[program]
pub mod anchor_spl_amm {
    
    use super::*;
    pub fn create_amm(ctx: Context<CreateAmm>, id: Pubkey, fee: u16) -> Result<()> {
        instructions::create_amm(ctx, id, fee)
    }

    pub fn create_pool(ctx: Context<CreatePool>, initial_price: u64) -> Result<()> {
        instructions::create_pool(ctx, initial_price)
    }

    pub fn deposit_liquidity(
        ctx: Context<DepositLiquidity>,
        amount_a: u64,
        amount_b: u64,
    ) -> Result<()> {
        instructions::deposit_liquidity(ctx, amount_a, amount_b)
    }

    pub fn withdraw_liquidity(ctx: Context<WithdrawLiquidity>, amount: u64) -> Result<()> {
        instructions::withdraw_liquidity(ctx, amount)
    }

    pub fn swap_exact_tokens_for_tokens(
        ctx: Context<SwapExactTokensForTokens>,
        swap_a: bool,
        input_amount: u64,
        min_output_amount: u64,
    ) -> Result<()> {
        instructions::swap_exact_tokens_for_tokens(ctx, swap_a, input_amount, min_output_amount)
    }
}