use anchor_lang::prelude::*;
use fixed::types::I64F64;

/// 价格影响配置
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub struct PriceImpactConfig {
    /// 是否启用高级价格影响保护
    pub enabled: bool,
    /// 最大允许滑点（基点，10000 = 100%）
    pub max_slippage_bps: u16,
    /// 动态滑点调整系数（放大1000倍）
    pub dynamic_adjustment_factor: u16,
}

impl PriceImpactConfig {
    // 计算结构体的大小：bool(1) + 2个u16(4)
    pub const LEN: usize = 1 + 2 * 2;
}

impl Default for PriceImpactConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_slippage_bps: 50, // 默认0.5%最大滑点
            dynamic_adjustment_factor: 1000, // 默认1.0
        }
    }
}

/// 价格影响计算器
pub struct PriceImpactCalculator;

impl PriceImpactCalculator {
    /// 计算交易的价格影响
    pub fn calculate_price_impact(
        config: &PriceImpactConfig,
        input_amount: u64,
        output_amount: u64,
        reserve_in: u64,
        reserve_out: u64,
    ) -> I64F64 {
        // 计算交易前后的价格变化
        let price_before = I64F64::from_num(reserve_out) / I64F64::from_num(reserve_in);
        let price_after = I64F64::from_num(reserve_out - output_amount) / I64F64::from_num(reserve_in + input_amount);
        
        // 计算价格影响百分比
        let price_impact = I64F64::from_num(1) - (price_after / price_before);
        
        price_impact
    }
    
    /// 检查交易是否超过最大允许的价格影响
    pub fn is_price_impact_acceptable(
        config: &PriceImpactConfig,
        price_impact: I64F64,
    ) -> bool {
        if !config.enabled {
            return true; // 如果未启用高级价格影响保护，默认接受任何价格影响
        }
        
        // 将价格影响转换为基点值进行比较
        let impact_bps = price_impact * I64F64::from_num(10000);
        let max_slippage = I64F64::from_num(config.max_slippage_bps);
        
        impact_bps <= max_slippage
    }
    
    /// 根据价格影响动态调整输出金额
    pub fn adjust_output_for_slippage(
        config: &PriceImpactConfig,
        output_amount: u64,
        price_impact: I64F64,
    ) -> u64 {
        if !config.enabled {
            return output_amount; // 如果未启用，不调整输出
        }
        
        // 根据价格影响计算调整系数
        let adjustment_factor = I64F64::from_num(1) - 
            (price_impact * I64F64::from_num(config.dynamic_adjustment_factor) / I64F64::from_num(1000));
        
        // 确保调整系数不会低于某个阈值（例如0.9）
        let min_adjustment = I64F64::from_num(0.9);
        let final_adjustment = if adjustment_factor < min_adjustment {
            min_adjustment
        } else {
            adjustment_factor
        };
        
        // 计算调整后的输出金额
        (I64F64::from_num(output_amount) * final_adjustment).to_num::<u64>()
    }
    
    /// 检查交易是否有利
    pub fn is_trade_beneficial(
        input_value: I64F64,
        output_value: I64F64,
        fee_percentage: I64F64,
    ) -> bool {
        // 计算交易成本（包括费用）
        let cost = input_value * (I64F64::from_num(1) + fee_percentage);
        
        // 如果输出价值大于输入价值加费用，则交易有利
        output_value > cost
    }
} 