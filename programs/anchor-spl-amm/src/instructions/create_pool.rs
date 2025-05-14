use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount},
};
use fixed::types::I64F64;

use crate::{
    constants::{AUTHORITY_SEED, LIQUIDITY_SEED},
    state::{Amm, Pool},
    models::volatility::VolatilityTracker,
};

// 分为两部分的指令实现
pub fn create_pool(ctx: Context<CreatePool>, initial_price: u64) -> Result<()> {
    // 首先初始化池
    let pool = &mut ctx.accounts.pool;
    pool.amm = ctx.accounts.amm.key();
    pool.mint_a = ctx.accounts.mint_a.key();
    pool.mint_b = ctx.accounts.mint_b.key();
    
    // 设置初始价格
    pool.initial_price = initial_price;
    
    // 初始化波动率追踪器
    pool.volatility_tracker = VolatilityTracker::default();
    
    // 如果开启了集中流动性，计算价格范围
    if ctx.accounts.amm.concentrated_liquidity_config.enabled {
        let current_price = I64F64::from_num(initial_price);
        let range_percentage = I64F64::from_num(ctx.accounts.amm.concentrated_liquidity_config.range_percentage) / I64F64::from_num(100);
        
        // 计算下限和上限价格
        let _lower_price = current_price * (I64F64::from_num(1) - range_percentage);
        let _upper_price = current_price * (I64F64::from_num(1) + range_percentage);
        
        // 未来可以将这些价格存储在池中，用于集中流动性范围的验证
    }

    Ok(())
}

// 分割成两个更小的上下文结构体以减少堆栈使用
#[derive(Accounts)]
#[instruction(initial_price: u64)]
pub struct CreatePool<'info> {
    #[account(
        seeds = [
            amm.id.as_ref()
        ],
        bump,
    )]
    pub amm: Box<Account<'info, Amm>>,

    #[account(
        init,
        payer = payer,
        space = Pool::LEN,
        seeds = [
            amm.key().as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
        ],
        bump,
    )]
    pub pool: Box<Account<'info, Pool>>,

    /// CHECK: Read only authority
    #[account(
        seeds = [
            amm.key().as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            AUTHORITY_SEED,
        ],
        bump,
    )]
    pub pool_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = payer,
        seeds = [
            amm.key().as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            LIQUIDITY_SEED,
        ],
        bump,
        mint::decimals = 6,
        mint::authority = pool_authority,
    )]
    pub mint_liquidity: Box<Account<'info, Mint>>,

    pub mint_a: Box<Account<'info, Mint>>,

    pub mint_b: Box<Account<'info, Mint>>,

    // 拆分账户减少同一时间验证的账户数量
    /// The liquidity pools
    pub token_accounts: TokenAccounts<'info>,

    /// The account paying for all rents
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Solana ecosystem accounts
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

// 单独的结构体持有池代币账户
#[derive(Accounts)]
pub struct TokenAccounts<'info> {
    #[account(
        init,
        payer = payer,
        associated_token::mint = mint_a,
        associated_token::authority = pool_authority,
    )]
    pub pool_account_a: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = payer,
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
    
    /// CHECK: Used for paying rent
    #[account(mut)]
    pub payer: AccountInfo<'info>,
    
    // 必须添加这些程序账户以实现init约束
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}