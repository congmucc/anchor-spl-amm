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
    > 此时可以在 [Explorer | Solana](https://explorer.solana.com/?cluster=custom)进行查看带有Metaplex部署好的Solana区块链，默认端口`8900`


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






### File Tree

```rust
programs/token-swap/src/
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