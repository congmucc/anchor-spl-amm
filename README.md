## Anchor SPL AMM - 高级自动做市商

这是一个基于Solana区块链的高级自动做市商(AMM)实现，使用Anchor框架开发。项目实现了传统AMM的核心功能，同时增加了多项创新的流动性管理、价格影响控制和费用策略功能。

### 功能亮点

- **集中流动性管理**: 允许LP在指定价格区间内提供流动性，提高资本效率
- **动态费用策略**: 支持固定、动态、分层和波动率调整四种费用计算方式
- **价格影响控制**: 精确计算和限制滑点，保护交易者免受过高价格影响
- **波动率追踪**: 记录分析价格波动，为流动性提供者提供无常损失补偿
- **模块化设计**: 清晰的代码结构便于扩展和维护

### 技术优化

- 针对Solana 4KB堆栈限制进行优化:
  - 使用结构分解减少同时验证的账户数量
  - 采用Box引用降低堆栈使用
  - 函数拆分减少复杂指令处理的堆栈压力

### File Tree

```rust
programs/anchor_spl_amm/src/
├── constants.rs
├── errors.rs
├── instructions
│   ├── create_amm.rs
│   ├── create_pool.rs
│   ├── deposit_liquidity.rs
│   ├── mod.rs
│   ├── swap_exact_tokens_for_tokens.rs
│   └── withdraw_liquidity.rs
├── lib.rs
├── models
│   ├── concentrated_liquidity.rs
│   ├── fee_strategy.rs
│   ├── mod.rs
│   ├── price_impact.rs
│   └── volatility.rs
└── state.rs
```

### How to use

- Build
`anchor build`

- Deploy
`anchor deploy`

- Test

**DEV**
> Change `Anchor.toml`
```rust
cluster = "https://api.devnet.solana.com"
```

```sh
anchor test
```

**Local**

1. Clone Metaplex Token Metadata Program

    在项目根目录新开一个目录`genesis`
    ```sh
    mkdir genesis
    cd genesis
    ```

    ```sh
    solana program dump -u m metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s metadata.so
    ```
    > 结果： `Wrote program to metadata.so`，并且会在本地生成相应的so文件


2. Initiate a Local Solana Validator

    ```sh
    solana-test-validator -r --bpf-program metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s metadata.so
    ```
    > **需要在`genesis`文件夹下**运行这段代码之后会开启一启动本地的 Solana 测试验证器，并会将 Metaplex Token Metadata 程序部署到 Localnet。
    > 
    > 此时可以在 [Explorer | Solana](https://explorer.solana.com/?cluster=custom)进行查看带有Metaplex部署好的Solana区块链，默认端口`8899`


    此时需要修改一下`Anchor.toml`，如下：
    ```rust
    [[test.genesis]]
    address = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
    program = "genesis/metadata.so"
    ```
    > 注意，这里`program`是一个路径，以根目录为基础。


3. Test
    > 项目目录新开一个终端

    ```sh
    anchor test --provider.cluster http://localhost:8899 --skip-local-validator
    ```
    > 这个是包含了build和deploy的。
    ```sh
    anchor test --provider.cluster http://localhost:8899 --skip-build --skip-deploy
    ```
    > 这个是只测试的。 前提是部署完毕之后

### 核心逻辑与高级功能

#### 核心AMM公式

- **流动性存入公式**
  ```rust
  let liquidity = I64F64::from_num(amount_a)
      .checked_mul(I64F64::from_num(amount_b))
      .unwrap()
      .sqrt()
      .to_num::<u64>();
  ```
  > 流动性计算: `liquidity = sqrt(amount_a * amount_b)`

- **交换公式**
  ```rust
  let output = I64F64::from_num(taxed_input)
      .checked_mul(I64F64::from_num(pool_b.amount))
      .unwrap()
      .checked_div(
          I64F64::from_num(pool_a.amount)
          .checked_add(I64F64::from_num(taxed_input))
          .unwrap(),
      )
      .unwrap();
  ```
  > 基于恒定乘积公式 `x * y = k`

#### 高级功能

- **集中流动性**
  ```rust
  // 集中流动性配置
  pub struct ConcentratedLiquidityConfig {
      pub enabled: bool,
      pub range_percentage: i64,  // 价格范围百分比
      pub min_width: i64,         // 最小价格区间宽度
  }

  // 计算价格范围
  let lower_price = current_price * (I64F64::from_num(1) - range_percentage);
  let upper_price = current_price * (I64F64::from_num(1) + range_percentage);
  ```

- **动态费用策略**
  ```rust
  // 动态费率计算
  pub enum FeeStrategy {
      Fixed,                  // 固定费率
      Dynamic,                // 根据交易量动态调整
      Tiered,                 // 基于交易量的分层费率
      VolatilityAdjusted,     // 根据市场波动率调整
  }
  ```

- **价格影响保护**
  ```rust
  // 价格影响检查
  if !PriceImpactCalculator::is_price_impact_acceptable(
      &amm.price_impact_config,
      price_impact
  ) {
      return err!(TutorialError::PriceImpactTooHigh);
  }
  ```

- **波动率追踪与无常损失计算**
  ```rust
  // 无常损失计算公式: 2√P/(1+P) - 1，其中P是价格比率
  let price_ratio = current_price / initial_price;
  let sqrt_ratio = price_ratio.sqrt();
  let il_percentage = I64F64::from_num(2)
      .checked_mul(sqrt_ratio)
      .unwrap()
      .checked_div(I64F64::from_num(1).checked_add(price_ratio).unwrap())
      .unwrap()
      .checked_sub(I64F64::from_num(1))
      .unwrap()
      .abs();
  ```