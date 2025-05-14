use anchor_lang::prelude::*;
use fixed::types::I64F64;
use std::f64;

/// 最大价格样本数
pub const MAX_SAMPLES: usize = 24;

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
    /// 波动率计算的样本窗口大小
    pub window_size: u8,
    /// 最小样本数量
    pub min_samples: u8,
    /// 历史样本的衰减因子（放大1000倍）
    pub decay_lambda: i64,
    /// 无常损失补偿系数（放大1000倍）
    pub compensation_factor: i64,
    /// 补偿周期（秒）
    pub compensation_period: i64,
}

impl Default for VolatilityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            protection_factor: 500, // 默认0.5
            decay_factor: 950, // 默认0.95 (每个周期衰减5%)
            min_volatility_threshold: 100, // 默认0.1 (10%)
            window_size: 24,
            min_samples: 2,
            decay_lambda: 950,
            compensation_factor: 1000,
            compensation_period: 86400,
        }
    }
}

impl VolatilityConfig {
    // 计算结构体的大小：2个u8(2) + 3个i64(24)
    pub const LEN: usize = 2 + 3 * 8;
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
    /// 价格历史样本（使用i128存储I64F64值）
    pub price_samples: [i128; MAX_SAMPLES],
    /// 对应时间戳
    pub timestamps: [i64; MAX_SAMPLES],
    /// 当前样本索引
    pub current_index: u8,
    /// 计算出的波动率（使用i128存储I64F64值）
    pub volatility_raw: i128,
    /// 最后更新时间
    pub last_updated: i64,
    /// 最后补偿时间
    pub last_compensated: i64,
}

impl VolatilityTracker {
    /// 计算结构体的大小：MAX_SAMPLES个i128(16*24) + MAX_SAMPLES个i64(8*24) + u8(1) + i128(16) + 2个i64(16)
    pub const LEN: usize = MAX_SAMPLES * 16 + MAX_SAMPLES * 8 + 1 + 16 + 16;
    
    /// 添加新的价格样本并更新波动率
    pub fn update_price_sample(&mut self, current_price: I64F64, timestamp: i64, config: &VolatilityConfig) {
        if !config.enabled {
            return;
        }
        
        // 存储前一个价格来计算收益率
        let prev_index = if self.current_index == 0 {
            MAX_SAMPLES - 1
        } else {
            (self.current_index - 1) as usize
        };
        
        // 如果已经有样本，计算对数收益率并更新波动率
        if self.timestamps[prev_index] > 0 {
            // 计算对数收益率
            let prev_price = I64F64::from_bits(self.price_samples[prev_index]);
            // 更新当前波动率计算
            self.calculate_volatility(config);
        }
        
        // 存储新的价格样本
        self.price_samples[self.current_index as usize] = current_price.to_bits();
        self.timestamps[self.current_index as usize] = timestamp;
        
        // 更新索引
        self.current_index = ((self.current_index as usize + 1) % MAX_SAMPLES) as u8;
        self.last_updated = timestamp;
    }
    
    /// 获取当前波动率
    pub fn get_volatility(&self) -> I64F64 {
        I64F64::from_bits(self.volatility_raw)
    }
    
    /// 内部方法：计算波动率
    fn calculate_volatility(&mut self, config: &VolatilityConfig) {
        let mut sum_squared_returns = I64F64::from_num(0);
        let mut valid_samples = 0;
        
        for i in 0..config.window_size as usize {
            let idx = (self.current_index as usize + MAX_SAMPLES - 1 - i) % MAX_SAMPLES;
            let prev_idx = (idx + MAX_SAMPLES - 1) % MAX_SAMPLES;
            
            // 确保有两个有效的连续样本
            if self.timestamps[idx] > 0 && self.timestamps[prev_idx] > 0 {
                let price = I64F64::from_bits(self.price_samples[idx]);
                let prev_price = I64F64::from_bits(self.price_samples[prev_idx]);
                
                // 计算对数收益率
                let price_f64 = price.to_num::<f64>();
                let prev_price_f64 = prev_price.to_num::<f64>();
                
                if price_f64 > 0.0 && prev_price_f64 > 0.0 {
                    let log_return = I64F64::from_num(f64::ln(price_f64 / prev_price_f64));
                    
                    // 应用时间衰减
                    let decay = I64F64::from_num(config.decay_lambda) / I64F64::from_num(1000);
                    // 使用乘法代替powi
                    let mut weight = I64F64::from_num(1);
                    for _ in 0..i {
                        weight = weight * decay;
                    }
                    
                    // 累加加权平方收益率
                    sum_squared_returns += log_return * log_return * weight;
                    valid_samples += 1;
                }
            }
        }
        
        // 只有当有足够的样本时才更新波动率
        if valid_samples >= config.min_samples {
            // 计算年化波动率
            let avg_squared_return = sum_squared_returns / I64F64::from_num(valid_samples);
            let volatility = avg_squared_return.sqrt() * I64F64::from_num(365 * 24); // 假设每小时一个样本，年化
            
            // 存储计算结果
            self.volatility_raw = volatility.to_bits();
        }
    }
    
    /// 根据当前波动率计算非永久性损失补偿
    pub fn calculate_il_compensation(
        &self,
        initial_price: I64F64, 
        current_price: I64F64,
        liquidity_value: u64, 
        config: &VolatilityConfig,
        current_timestamp: i64,
    ) -> u64 {
        if !config.enabled || current_timestamp - self.last_compensated < config.compensation_period {
            return 0;
        }
        
        // 计算价格比率
        let price_ratio = current_price / initial_price;
        
        // 使用无常损失公式: 2√P/(1+P) - 1
        let sqrt_ratio = price_ratio.sqrt();
        let numerator = I64F64::from_num(2) * sqrt_ratio;
        let denominator = I64F64::from_num(1) + price_ratio;
        let il_percentage = (numerator / denominator) - I64F64::from_num(1);
        
        // 将百分比转换为正值
        let il_percentage_abs = il_percentage.abs();
        
        // 应用补偿因子（从配置）
        let compensation_factor = I64F64::from_num(config.compensation_factor) / I64F64::from_num(1000);
        
        // 计算补偿金额
        let compensation_amount = il_percentage_abs * compensation_factor * I64F64::from_num(liquidity_value);
        
        compensation_amount.floor().to_num::<u64>()
    }
    
    /// 估算LP头寸的非永久性损失
    pub fn estimate_impermanent_loss(
        initial_price: I64F64,
        current_price: I64F64
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