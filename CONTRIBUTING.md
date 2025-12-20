# Contributing

感谢你考虑为这个项目做贡献！

## 如何贡献

### 报告问题

1. 检查是否已经有相关的 issue
2. 创建新的 issue 并清晰描述问题
3. 提供代码示例（如果适用）

### 提交代码

1. Fork 仓库并创建功能分支
2. 进行修改并添加测试
3. 运行测试和格式化：
   ```bash
   cargo test --all
   cargo fmt --all
   cargo clippy --all
   ```
4. 提交改动（使用 Conventional Commits）

## 代码规范

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查代码
- 添加文档注释
- 编写测试

## 项目结构

```
crates/
├── searepo/       # 核心库（Trait 定义、类型、re-exports）
│   └── src/
│       └── lib.rs
└── sea-macros/    # 派生宏实现
    └── src/
        ├── lib.rs
        └── searepo.rs

examples/          # 使用示例
tests/             # 集成测试
```

