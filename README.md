# Repository Pattern for SeaORM

一个基于 SeaORM 的 Rust Repository 模式库，通过派生宏自动生成 CRUD 操作。

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## ✨ 特性

- 🚀 **自动生成 CRUD**：通过派生宏自动生成常用方法
- 🎯 **类型安全**：基于 SeaORM 的强类型系统
- 🔍 **灵活查询**：支持自定义 FindFilter、SearchFilter、DeleteFilter
- 📦 **分页支持**：支持 PageOffset 和 Cursor 两种分页方式
- 🔄 **批量操作**：支持批量插入、Upsert（自动分块处理）
- 🎨 **功能选择**：通过 `all`、`include`、`exclude` 灵活控制生成的方法
- 💾 **事务支持**：批量操作自动使用事务

## 📦 安装

```toml
[dependencies]
searepo = "0.1"
sea-orm = "2.0"
tokio = { version = "1", features = ["full"] }
```

## 🚀 快速开始

### 1. 定义 Repository

```rust
use searepo::Repository;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[derive(Repository)]
#[repository(entity = "entities::user", domain = "User")]
pub struct UserRepository {
    db: Arc<DatabaseConnection>,
}

impl UserRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}
```

### 2. 定义过滤器

```rust
use searepo::{FindFilter, SearchFilter, DeleteFilter, Pagination};
use sea_orm::*;

pub mod filter {
    use super::*;

    // 单条查询过滤器
    pub enum Find {
        ById(i64),
        ByEmail(String),
    }

    impl FindFilter<entities::user::Entity> for Find {
        fn to_select(self) -> Select<entities::user::Entity> {
            match self {
                Find::ById(id) => entities::user::Entity::find_by_id(id),
                Find::ByEmail(email) => {
                    entities::user::Entity::find()
                        .filter(entities::user::Column::Email.eq(email))
                }
            }
        }
    }

    // 搜索过滤器（支持分页）
    #[derive(Default)]
    pub struct Search {
        pub username: Option<String>,
        pub active: Option<bool>,
        pub pagination: Option<Pagination>,
    }

    impl SearchFilter<entities::user::Entity> for Search {
        fn to_select(self) -> Select<entities::user::Entity> {
            let mut query = entities::user::Entity::find();

            if let Some(username) = self.username {
                query = query.filter(entities::user::Column::Username.contains(&username));
            }
            if let Some(active) = self.active {
                query = query.filter(entities::user::Column::Active.eq(active));
            }

            query
        }

        fn pagination(&self) -> Option<Pagination> {
            self.pagination
        }
    }

    // 删除过滤器
    pub enum Delete {
        ById(i64),
    }

    impl DeleteFilter<entities::user::Entity> for Delete {
        fn to_select(self) -> Select<entities::user::Entity> {
            match self {
                Delete::ById(id) => entities::user::Entity::find_by_id(id),
            }
        }
    }
}
```

### 3. 使用 Repository

```rust
// 查询
let user = repo.find(filter::Find::ById(1)).await?;        // Option<User>
let user = repo.load(filter::Find::ById(1)).await?;        // User (不存在则报错)
let exists = repo.exists(filter::Find::ById(1)).await?;    // bool

// 搜索（支持分页）
let result = repo.search(filter::Search::default ()).await?;
println!("总数: {}, 当前页: {}", result.total_count, result.items.len());

// 插入
let new_user = User { id: 1, username: "alice".into(), email: "alice@example.com".into() };
let created = repo.insert(new_user).await?;

// 批量插入（自动分块，使用事务）
let users = vec![user1, user2, user3];
let created = repo.batch_insert(users).await?;

// 更新
repo.update(user).await?;

// 删除（返回被删除的实体）
let deleted_users = repo.delete(filter::Delete::ById(1)).await?;

// 或只获取删除数量
let count = repo.delete_count(filter::Delete::ById(1)).await?;
```

## 📖 功能开关

通过宏参数灵活控制生成的方法：

### 1. 默认行为（只生成 find + search）

```rust
#[derive(Repository)]
#[repository(entity = "entities::user", domain = "User")]
pub struct UserRepository {
    db: Arc<DatabaseConnection>
}
// 生成: find(), load(), exists(), search(), count()
```

### 2. 启用所有功能

```rust
#[repository(entity = "...", domain = "...", all = true)]
```

### 3. 指定启用的功能（include）

```rust
#[repository(
    entity = "...",
    domain = "...",
    include = ["find", "insert", "delete"]
)]
```

### 4. 排除某些功能（exclude）

```rust
#[repository(
    entity = "...",
    domain = "...",
    exclude = ["delete", "upsert"]
)]
// 启用除 delete 和 upsert 外的所有功能
```

### 可用的功能

| Feature  | 生成的方法                          |
|----------|--------------------------------|
| `find`   | `find()`, `load()`, `exists()` |
| `search` | `search()`, `count()`          |
| `insert` | `insert()`, `batch_insert()`   |
| `update` | `update()`                     |
| `delete` | `delete()`, `delete_count()`   |
| `upsert` | `upsert()`, `batch_upsert()`   |

**优先级**: `all` > `include` > `exclude` > 默认 (`find` + `search`)

## 🔥 高级功能

### 分页查询

支持两种分页方式：

```rust
use searepo::Pagination;

// 1. PageOffset 分页（页码 + 页大小）
let result = repo.search(filter::Search {
pagination: Some(Pagination::PageOffset { page: 1, size: 20 }),
..Default::default ()
}).await?;

println!("第 {}/{} 页", result.page, result.total_page);
println!("共 {} 条记录", result.total_count);
println!("是否有更多: {}", result.has_more());

// 2. Cursor 分页（游标分页，适用于大数据集）
let result = repo.search(filter::Search {
pagination: Some(Pagination::Cursor { limit: 50 }),
..Default::default ()
}).await?;
```

### Upsert 操作

```rust
use searepo::Upsertable;
use sea_orm::sea_query::OnConflict;

// 在领域实体上实现 Upsertable trait
impl Upsertable<entities::product::Entity> for Product {
    fn on_conflict() -> OnConflict {
        use entities::product::Column;
        OnConflict::columns([Column::Sku])
            .update_columns([Column::Name, Column::Price])
            .to_owned()
    }
}

// 使用
repo.upsert(product).await?;
repo.batch_upsert(products).await?;  // 批量 upsert（自动分块）
```

### 批量操作

批量操作会自动计算最大分块大小（基于 PostgreSQL 参数限制），并使用事务确保原子性：

```rust
// 批量插入
let users = generate_users(10000);  // 大量数据
let inserted = repo.batch_insert(users).await?;  // 自动分块处理

// 批量 upsert
repo.batch_upsert(products).await?;
```

## 📚 生成的方法

### 查询方法 (find)

| 方法               | 说明          | 返回类型        |
|------------------|-------------|-------------|
| `find(filter)`   | 查找单个（可能不存在） | `Option<T>` |
| `load(filter)`   | 查找单个（必须存在）  | `T`         |
| `exists(filter)` | 检查是否存在      | `bool`      |

### 搜索方法 (search)

| 方法               | 说明       | 返回类型              |
|------------------|----------|-------------------|
| `search(filter)` | 搜索（支持分页） | `SearchResult<T>` |
| `count(filter)`  | 统计数量     | `u64`             |

### 插入方法 (insert)

| 方法                       | 说明            | 返回类型     |
|--------------------------|---------------|----------|
| `insert(entity)`         | 插入单个          | `T`      |
| `batch_insert(entities)` | 批量插入（自动分块，事务） | `Vec<T>` |

### 更新方法 (update)

| 方法               | 说明   | 返回类型 |
|------------------|------|------|
| `update(entity)` | 更新单个 | `T`  |

### 删除方法 (delete)

| 方法                     | 说明          | 返回类型     |
|------------------------|-------------|----------|
| `delete(filter)`       | 删除并返回被删除的实体 | `Vec<T>` |
| `delete_count(filter)` | 删除并返回数量     | `u64`    |

### Upsert 方法 (upsert)

| 方法                       | 说明                 | 返回类型 |
|--------------------------|--------------------|------|
| `upsert(entity)`         | 插入或更新              | `()` |
| `batch_upsert(entities)` | 批量 upsert（自动分块，事务） | `()` |

## 🎯 使用场景

### CQRS 模式

```rust
// Command 侧
#[repository(entity = "...", domain = "...", include = ["insert", "update", "delete"])]
pub struct UserCommandRepository {
    db: Arc<DatabaseConnection>
}

// Query 侧
#[repository(entity = "...", domain = "...")] // 默认只有 find + search
pub struct UserQueryRepository {
    db: Arc<DatabaseConnection>
}
```

### 审计日志（只写 + 查询）

```rust
#[repository(entity = "...", domain = "...", include = ["insert", "search"])]
pub struct AuditLogRepository {
    db: Arc<DatabaseConnection>
}
```

### 数据同步（只需要 upsert）

```rust
#[repository(entity = "...", domain = "...", include = ["upsert"])]
pub struct DataSyncRepository {
    db: Arc<DatabaseConnection>
}
```

## 🔧 必需实现的 Trait

根据启用的功能，需要实现相应的 trait：

| 功能                  | 需要实现的 Trait                                              |
|---------------------|----------------------------------------------------------|
| `find`              | `FindFilter`                                             |
| `search`            | `SearchFilter`                                           |
| `insert` / `update` | `From<Domain> for ActiveModel`, `From<Model> for Domain` |
| `delete`            | `DeleteFilter`                                           |
| `upsert`            | `Upsertable`, `From<Domain> for ActiveModel`             |

## 📝 示例

查看 `examples/` 目录获取更多示例：

```bash
cargo run --example basic_crud
cargo run --example pagination
cargo run --example feature_flags
```

## 📄 许可证

MIT License - 查看 [LICENSE](LICENSE) 文件了解详情。

