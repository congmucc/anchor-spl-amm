use anchor_lang::prelude::*;
use fixed::types::I64F64;

/// 聚合流动性配置
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub struct ConcentratedLiquidityConfig {
    /// 是否启用聚合流动性
    pub enabled: bool,
    /// 聚合流动性范围（价格范围百分比，例如：10表示当前价格的±10%范围内）
    pub range_percentage: u16,
    /// 聚合流动性奖励系数（放大1000倍），用于计算提供聚合流动性的额外奖励
    pub reward_multiplier: u16,
    /// 最小范围宽度
    pub min_width: i64,
}

impl Default for ConcentratedLiquidityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            range_percentage: 10, // 默认范围为当前价格的±10%
            reward_multiplier: 1200, // 默认奖励系数为1.2（1200/1000）
            min_width: 0,
        }
    }
}

impl ConcentratedLiquidityConfig {
    // 计算结构体的大小：bool(1) + 2个i64(16)
    pub const LEN: usize = 1 + 16;
}

/// 聚合流动性价格计算
pub struct ConcentratedLiquidityPricing;

impl ConcentratedLiquidityPricing {
    /// 计算给定价格范围内的流动性价值
    pub fn calculate_concentrated_liquidity_value(
        config: &ConcentratedLiquidityConfig,
        current_price: I64F64,
        token_a_amount: u64,
        token_b_amount: u64,
    ) -> I64F64 {
        if !config.enabled {
            return I64F64::from_num(0);
        }

        // 计算流动性范围
        let range_percentage = I64F64::from_num(config.range_percentage as u64) / I64F64::from_num(100);
        let lower_price = current_price * (I64F64::from_num(1) - range_percentage);
        let upper_price = current_price * (I64F64::from_num(1) + range_percentage);

        // 计算聚合流动性值
        let token_a_value = I64F64::from_num(token_a_amount);
        let token_b_value = I64F64::from_num(token_b_amount) * current_price;
        let total_value = token_a_value + token_b_value;

        // 返回加权后的流动性值
        total_value * I64F64::from_num(config.reward_multiplier) / I64F64::from_num(1000)
    }

    /// 计算特定价格点的流动性深度
    pub fn calculate_liquidity_depth(
        config: &ConcentratedLiquidityConfig,
        current_price: I64F64,
        target_price: I64F64,
        token_a_reserve: u64,
        token_b_reserve: u64,
    ) -> I64F64 {
        if !config.enabled {
            // 如果未启用聚合流动性，使用恒定乘积公式
            return I64F64::from_num(token_a_reserve) * I64F64::from_num(token_b_reserve);
        }

        // 计算流动性范围
        let range_percentage = I64F64::from_num(config.range_percentage as u64) / I64F64::from_num(100);
        let lower_price = current_price * (I64F64::from_num(1) - range_percentage);
        let upper_price = current_price * (I64F64::from_num(1) + range_percentage);

        // 如果目标价格在范围内，提供更多流动性
        if target_price >= lower_price && target_price <= upper_price {
            let base_liquidity = I64F64::from_num(token_a_reserve) * I64F64::from_num(token_b_reserve);
            let boost_factor = I64F64::from_num(config.reward_multiplier) / I64F64::from_num(1000);
            return base_liquidity * boost_factor;
        }

        // 如果目标价格在范围外，使用恒定乘积公式
        I64F64::from_num(token_a_reserve) * I64F64::from_num(token_b_reserve)
    }
} 