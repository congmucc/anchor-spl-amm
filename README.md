## Token swap example amm in anchor rust

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
└── state.rs
```

### Core Logic
 

- `deposit_liquidity.rs`
  **Deposit formula**
  ```rust
   let ratio = I64F64::from_num(pool_a.amount)
        .checked_mul(I64F64::from_num(pool_b.amount))
        .unwrap();
    if pool_a.amount > pool_b.amount {
        (
            I64F64::from_num(amount_b)
                .checked_mul(ratio)
                .unwrap()
                .to_num::<u64>(),
            amount_b,
        )
    } 
  ```
  > `ratio = pool_a.amount * pool_b.amount`
  > `adjusted_amount_b = amount_a / ratio`


  **Liquidity injection formula**
    ```rust
    let mut liquidity = I64F64::from_num(amount_a)
        .checked_mul(I64F64::from_num(amount_b))
        .unwrap()
        .sqrt()
        .to_num::<u64>();
    ```
    > `liquidity = sqrt(amount_a * amount_b)`
 

- `swap_exact_tokens_for_tokens.rs`
  **Swap formula**
  ```rust
    I64F64::from_num(taxed_input)
        .checked_mul(I64F64::from_num(pool_b.amount))
        .unwrap()
        .checked_div(
            I64F64::from_num(pool_a.amount)
            .checked_add(I64F64::from_num(taxed_input))
            .unwrap(),
        )
        .unwrap()
  ```

  > `output = (pool_a.amount + taxed_input) / taxed_input * pool_b.amount`
  > This is essentially `x * y = k` x is pool_a.amount, y is pool_b.amount

