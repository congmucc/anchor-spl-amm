use anchor_lang::prelude::*;
use fixed::types::I64F64;
use std::f64;

/// 波动率跟踪配置
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub struct VolatilityConfig {
    /// 是否启用波动率跟踪和保护
    pub enabled: bool,
    /// 保护力度系数（放大1000倍）
    pub protection_factor: u16,
    /// 波动率衰减系数（放大1000倍）
    pub decay_factor: u16,
    /// 最小波动率阈值（放大1000倍）
    pub min_volatility_threshold: u16,
}

impl Default for VolatilityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            protection_factor: 500, // 默认0.5
            decay_factor: 950, // 默认0.95 (每个周期衰减5%)
            min_volatility_threshold: 100, // 默认0.1 (10%)
        }
    }
}

/// 价格采样数据，用于跟踪历史价格
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default)]
pub struct PriceSample {
    /// 价格数据（放大1e9倍）
    pub price: u64,
    /// 时间戳（unix时间）
    pub timestamp: i64,
}

/// 波动率监测器
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default)]
pub struct VolatilityTracker {
    /// 历史价格样本
    pub price_samples: [PriceSample; 24], // 保存24个历史价格点
    /// 当前样本索引
    pub current_index: u8,
    /// 当前计算的波动率（放大1000倍）
    pub current_volatility: u16,
    /// 最后更新时间
    pub last_updated: i64,
}

impl VolatilityTracker {
    /// 添加新的价格样本并更新波动率
    pub fn update_price_sample(&mut self, current_price: I64F64, timestamp: i64, config: &VolatilityConfig) {
        if !config.enabled {
            return;
        }
        
        // 转换价格为整数表示（放大1e9）
        let price_integer = (current_price * I64F64::from_num(1_000_000_000)).to_num::<u64>();
        
        // 创建新的价格样本
        let new_sample = PriceSample {
            price: price_integer,
            timestamp,
        };
        
        // 更新样本数组
        self.price_samples[self.current_index as usize] = new_sample;
        self.current_index = (self.current_index + 1) % 24;
        
        // 计算新的波动率
        self.calculate_volatility(config);
        
        // 更新最后更新时间
        self.last_updated = timestamp;
    }
    
    /// 计算当前波动率
    fn calculate_volatility(&mut self, config: &VolatilityConfig) {
        // 计算日志收益率的标准差
        let mut valid_samples = 0;
        let mut sum = I64F64::from_num(0);
        let mut sum_squared = I64F64::from_num(0);
        
        // 找到有效的价格样本
        let valid_price_samples: Vec<I64F64> = self.price_samples.iter()
            .filter(|sample| sample.timestamp > 0) // 排除未初始化的样本
            .map(|sample| I64F64::from_num(sample.price))
            .collect();
        
        valid_samples = valid_price_samples.len();
        
        if valid_samples < 2 {
            // 不足以计算波动率
            return;
        }
        
        // 计算收益率
        let returns: Vec<I64F64> = (1..valid_samples).filter_map(|i| {
            let prev_price = valid_price_samples[i - 1];
            let curr_price = valid_price_samples[i];
            
            // 计算对数收益率 - 使用f64转换计算，因为I64F64没有ln方法
            if prev_price > I64F64::from_num(0) && curr_price > I64F64::from_num(0) {
                let prev_price_f64 = prev_price.to_num::<f64>();
                let curr_price_f64 = curr_price.to_num::<f64>();
                let log_return = I64F64::from_num(f64::ln(curr_price_f64 / prev_price_f64));
                sum = sum + log_return;
                Some(log_return)
            } else {
                None
            }
        }).collect();
        
        if returns.is_empty() {
            // 没有有效的收益率
            return;
        }
        
        let mean = sum / I64F64::from_num(returns.len());
        
        // 计算方差
        for ret in &returns {
            let deviation = *ret - mean;
            sum_squared = sum_squared + (deviation * deviation);
        }
        
        let variance = sum_squared / I64F64::from_num(returns.len());
        let volatility = variance.sqrt();
        
        // 应用衰减因子
        let decay = I64F64::from_num(config.decay_factor) / I64F64::from_num(1000);
        let old_volatility = I64F64::from_num(self.current_volatility) / I64F64::from_num(1000);
        let new_volatility = decay * old_volatility + (I64F64::from_num(1) - decay) * volatility;
        
        // 转换为u16存储（放大1000倍）
        self.current_volatility = (new_volatility * I64F64::from_num(1000)).to_num::<u16>();
    }
    
    /// 根据当前波动率计算非永久性损失补偿
    pub fn calculate_il_compensation(
        &self, 
        config: &VolatilityConfig, 
        estimated_loss: I64F64
    ) -> I64F64 {
        if !config.enabled || estimated_loss <= I64F64::from_num(0) {
            return I64F64::from_num(0);
        }
        
        // 获取当前波动率
        let volatility = I64F64::from_num(self.current_volatility) / I64F64::from_num(1000);
        let threshold = I64F64::from_num(config.min_volatility_threshold) / I64F64::from_num(1000);
        
        // 只有当波动率超过阈值时才提供保护
        if volatility < threshold {
            return I64F64::from_num(0);
        }
        
        // 计算补偿比例，与波动率成正比
        let protection_factor = I64F64::from_num(config.protection_factor) / I64F64::from_num(1000);
        let compensation_ratio = protection_factor * (volatility - threshold) / volatility;
        
        // 确保补偿比例不超过1
        let capped_ratio = if compensation_ratio > I64F64::from_num(1) {
            I64F64::from_num(1)
        } else {
            compensation_ratio
        };
        
        // 计算补偿金额
        estimated_loss * capped_ratio
    }
    
    /// 估算LP头寸的非永久性损失
    pub fn estimate_impermanent_loss(
        initial_price: I64F64,
        current_price: I64F64,
    ) -> I64F64 {
        if initial_price <= I64F64::from_num(0) || current_price <= I64F64::from_num(0) {
            return I64F64::from_num(0);
        }
        
        let price_ratio = current_price / initial_price;
        
        // 非永久性损失公式：2*sqrt(r)/(1+r) - 1
        // 其中r是价格比率
        let sqrt_ratio = price_ratio.sqrt();
        let denominator = I64F64::from_num(1) + price_ratio;
        
        let holding_value = I64F64::from_num(2) * sqrt_ratio / denominator;
        let impermanent_loss = holding_value - I64F64::from_num(1);
        
        // 返回损失的绝对值（正数）
        if impermanent_loss < I64F64::from_num(0) {
            impermanent_loss.abs()
        } else {
            I64F64::from_num(0) // 如果计算结果为正，表示没有损失
        }
    }
} 