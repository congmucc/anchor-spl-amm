use anchor_lang::prelude::*;
use fixed::types::I64F64;

/// 费用策略枚举
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum FeeStrategy {
    /// 固定费用 - 始终使用相同的手续费率
    Fixed,
    /// 动态费用 - 根据池子深度和交易量调整费用
    Dynamic,
    /// 分层费用 - 根据交易量分层收费
    Tiered,
    /// 按波动率调整 - 高波动率时提高费用
    VolatilityAdjusted,
}

impl Default for FeeStrategy {
    fn default() -> Self {
        FeeStrategy::Fixed
    }
}

/// 费用配置
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct FeeConfig {
    /// 当前使用的费用策略
    pub strategy: FeeStrategy,
    /// 最低费用（基点 - 10000 = 100%）
    pub min_fee_bps: u16,
    /// 最高费用（基点 - 10000 = 100%）
    pub max_fee_bps: u16,
    /// 当前基础费用（基点 - 10000 = 100%）
    pub base_fee_bps: u16,
    /// 费用调整系数（放大1000倍）
    pub adjustment_factor: u16,
}

impl Default for FeeConfig {
    fn default() -> Self {
        Self {
            strategy: FeeStrategy::Fixed,
            min_fee_bps: 10,    // 最低0.1%
            max_fee_bps: 100,   // 最高1%
            base_fee_bps: 30,   // 基础费率0.3%
            adjustment_factor: 1000, // 调整系数1.0
        }
    }
}

impl FeeConfig {
    // 计算结构体的大小：枚举(1) + 4个u16(8)
    pub const LEN: usize = 1 + 4 * 2;
}

/// 费用计算器
pub struct FeeCalculator;

impl FeeCalculator {
    /// 根据当前策略计算交易费用
    pub fn calculate_fee(
        config: &FeeConfig, 
        input_amount: u64,
        reserve_in: u64,
        reserve_out: u64,
        volatility: Option<u16>,
    ) -> u64 {
        // 获取基点费率
        let fee_bps = Self::get_fee_rate_bps(config, input_amount, reserve_in, reserve_out, volatility);
        
        // 计算费用金额
        (I64F64::from_num(input_amount) * I64F64::from_num(fee_bps) / I64F64::from_num(10000)).to_num::<u64>()
    }
    
    /// 获取按策略计算的费率（基点）
    pub fn get_fee_rate_bps(
        config: &FeeConfig, 
        input_amount: u64,
        reserve_in: u64,
        reserve_out: u64,
        volatility: Option<u16>,
    ) -> u16 {
        match config.strategy {
            FeeStrategy::Fixed => config.base_fee_bps,
            FeeStrategy::Dynamic => Self::calculate_dynamic_fee_bps(config, input_amount, reserve_in),
            FeeStrategy::Tiered => Self::calculate_tiered_fee_bps(config, input_amount),
            FeeStrategy::VolatilityAdjusted => Self::calculate_volatility_adjusted_fee_bps(
                config, 
                volatility.unwrap_or(0)
            ),
        }
    }
    
    /// 计算动态费用（基于池子深度和交易量）
    fn calculate_dynamic_fee_bps(
        config: &FeeConfig, 
        input_amount: u64,
        reserve: u64,
    ) -> u16 {
        // 计算交易量占池子的比例
        let ratio = if reserve == 0 {
            I64F64::from_num(1) // 防止除以0
        } else {
            I64F64::from_num(input_amount) / I64F64::from_num(reserve)
        };
        
        // 用二次曲线调整费率：base_fee + adjustment * (ratio)^2
        let adjustment = I64F64::from_num(config.adjustment_factor) / I64F64::from_num(1000);
        let base_fee = I64F64::from_num(config.base_fee_bps);
        let fee_adjustment = adjustment * ratio * ratio;
        
        // 计算最终费率，确保在min和max之间
        let calculated_fee = base_fee + fee_adjustment * I64F64::from_num(10000);
        let fee_bps = calculated_fee.to_num::<u16>();
        
        fee_bps.clamp(config.min_fee_bps, config.max_fee_bps)
    }
    
    /// 计算分层费用（基于交易量大小）
    fn calculate_tiered_fee_bps(config: &FeeConfig, input_amount: u64) -> u16 {
        // 定义几个交易量分层阈值
        let tier1 = 1_000 * 10u64.pow(6); // 1,000 tokens (假设6位小数)
        let tier2 = 10_000 * 10u64.pow(6); // 10,000 tokens
        let tier3 = 100_000 * 10u64.pow(6); // 100,000 tokens
        
        // 根据交易量确定费率
        let tier_fee = if input_amount < tier1 {
            config.max_fee_bps // 小额交易，使用最高费率
        } else if input_amount < tier2 {
            // 线性插值第一层和第二层之间
            let mid_fee = (config.max_fee_bps + config.base_fee_bps) / 2;
            mid_fee
        } else if input_amount < tier3 {
            config.base_fee_bps // 中等交易，使用基础费率
        } else {
            config.min_fee_bps // 大额交易，使用最低费率
        };
        
        tier_fee
    }
    
    /// 计算基于波动率的费用
    fn calculate_volatility_adjusted_fee_bps(config: &FeeConfig, volatility: u16) -> u16 {
        // 波动率门槛
        let low_threshold = 50; // 波动率低于5%
        let high_threshold = 200; // 波动率高于20%
        
        // 根据波动率调整费率
        let fee_bps = if volatility < low_threshold {
            config.min_fee_bps // 低波动率，使用最低费率
        } else if volatility > high_threshold {
            config.max_fee_bps // 高波动率，使用最高费率
        } else {
            // 线性插值波动率与费率
            let volatility_range = high_threshold - low_threshold;
            let fee_range = config.max_fee_bps - config.min_fee_bps;
            let vol_position = volatility - low_threshold;
            
            config.min_fee_bps + (vol_position * fee_range) / volatility_range
        };
        
        fee_bps
    }
} 