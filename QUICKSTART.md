# 快速开始指南 🚀

## 📦 安装

```toml
[dependencies]
searepo = "0.1"
sea-orm = { version = "2.0", features = ["sqlx-sqlite", "runtime-tokio-native-tls"] }
tokio = { version = "1", features = ["full"] }
```

## 🏗️ 基础设置

### 1. 定义数据库实体

```rust
// src/entities/user.rs
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub username: String,
    pub email: String,
    pub active: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
```

### 2. 定义领域模型

```rust
// src/domain/user.rs
#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub active: bool,
}

// Model -> Domain
impl From<crate::entities::user::Model> for User {
    fn from(m: crate::entities::user::Model) -> Self {
        Self {
            id: m.id,
            username: m.username,
            email: m.email,
            active: m.active,
        }
    }
}

// Domain -> ActiveModel
impl From<User> for crate::entities::user::ActiveModel {
    fn from(u: User) -> Self {
        use sea_orm::ActiveValue::Set;
        Self {
            id: Set(u.id),
            username: Set(u.username),
            email: Set(u.email),
            active: Set(u.active),
        }
    }
}
```

### 3. 创建 Repository

```rust
use searepo::{Repository, FindFilter, SearchFilter, DeleteFilter, Pagination};
use sea_orm::{DatabaseConnection, Select, EntityTrait, ColumnTrait, QueryFilter};
use std::sync::Arc;

#[derive(Repository)]
#[repository(entity = "crate::entities::user", domain = "User")]
pub struct UserRepository {
    db: Arc<DatabaseConnection>,
}

impl UserRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

pub mod filter {
    use super::*;
    use crate::entities::user::{Entity, Column};

    // 查找单个记录
    pub enum Find {
        ById(i64),
        ByEmail(String),
    }

    impl FindFilter<Entity> for Find {
        fn to_select(self) -> Select<Entity> {
            match self {
                Find::ById(id) => Entity::find_by_id(id),
                Find::ByEmail(email) => Entity::find().filter(Column::Email.eq(email)),
            }
        }
    }

    // 搜索多个记录
    #[derive(Default)]
    pub struct Search {
        pub username: Option<String>,
        pub active: Option<bool>,
        pub pagination: Option<Pagination>,
    }

    impl SearchFilter<Entity> for Search {
        fn to_select(self) -> Select<Entity> {
            let mut select = Entity::find();

            if let Some(username) = self.username {
                select = select.filter(Column::Username.contains(&username));
            }
            if let Some(active) = self.active {
                select = select.filter(Column::Active.eq(active));
            }

            select
        }

        fn pagination(&self) -> Option<Pagination> {
            self.pagination
        }
    }

    // 删除过滤器
    pub enum Delete {
        ById(i64),
    }

    impl DeleteFilter<Entity> for Delete {
        fn to_select(self) -> Select<Entity> {
            match self {
                Delete::ById(id) => Entity::find_by_id(id),
            }
        }
    }
}
```

## 💡 使用示例

### 基本 CRUD 操作

```rust
use std::sync::Arc;
use sea_orm::Database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::connect("sqlite::memory:").await?;
    let repo = UserRepository::new(Arc::new(db));

    // 插入
    let user = User {
        id: 0,
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        active: true,
    };
    let created = repo.insert(user).await?;

    // 查找（返回 Option）
    let found = repo.find(filter::Find::ById(created.id)).await?;

    // 加载（必须存在，否则报错）
    let user = repo.load(filter::Find::ById(created.id)).await?;

    // 检查是否存在
    let exists = repo.exists(filter::Find::ByEmail("alice@example.com".into())).await?;

    // 搜索
    let result = repo.search(filter::Search {
        active: Some(true),
        ..Default::default()
    }).await?;
    println!("找到 {} 个用户", result.total_count);

    // 更新
    let mut user = repo.load(filter::Find::ById(created.id)).await?;
    user.username = "alice_updated".to_string();
    repo.update(user).await?;

    // 删除（返回被删除的实体）
    let deleted = repo.delete(filter::Delete::ById(created.id)).await?;

    Ok(())
}
```

### 分页查询

```rust
use searepo::Pagination;

// PageOffset 分页
let result = repo.search(filter::Search {
pagination: Some(Pagination::PageOffset { page: 1, size: 10 }),
..Default::default ()
}).await?;

println!("第 {}/{} 页", result.page, result.total_page);
println!("共 {} 条", result.total_count);
println!("是否有更多: {}", result.has_more());

// Cursor 分页
let result = repo.search(filter::Search {
pagination: Some(Pagination::Cursor { limit: 50 }),
..Default::default ()
}).await?;
```

### 批量操作

```rust
// 批量插入（自动分块，使用事务）
let users = vec![
    User { id: 0, username: "user1".to_string(), email: "user1@example.com".to_string(), active: true },
    User { id: 0, username: "user2".to_string(), email: "user2@example.com".to_string(), active: true },
];
let created = repo.batch_insert(users).await?;
```

### Upsert 操作

```rust
use searepo::Upsertable;
use sea_orm::sea_query::OnConflict;

impl Upsertable<crate::entities::user::Entity> for User {
    fn on_conflict() -> OnConflict {
        use crate::entities::user::Column;
        OnConflict::columns([Column::Email])
            .update_columns([Column::Username, Column::Active])
            .to_owned()
    }
}

// 单个 upsert
repo.upsert(user).await?;

// 批量 upsert
repo.batch_upsert(users).await?;
```

## 🎯 功能选择

```rust
// 默认：只有 find + search
#[repository(entity = "...", domain = "...")]

// 启用所有功能
#[repository(entity = "...", domain = "...", all = true)]

// 只启用指定功能
#[repository(entity = "...", domain = "...", include = ["find", "insert"])]

// 排除某些功能
#[repository(entity = "...", domain = "...", exclude = ["delete"])]
```

## 📚 自动生成的方法

| 功能       | 方法                       | 返回类型                   |
|----------|--------------------------|------------------------|
| `find`   | `find(filter)`           | `Option<Domain>`       |
| `find`   | `load(filter)`           | `Domain`               |
| `find`   | `exists(filter)`         | `bool`                 |
| `search` | `search(filter)`         | `SearchResult<Domain>` |
| `search` | `count(filter)`          | `u64`                  |
| `insert` | `insert(entity)`         | `Domain`               |
| `insert` | `batch_insert(entities)` | `Vec<Domain>`          |
| `update` | `update(entity)`         | `Domain`               |
| `delete` | `delete(filter)`         | `Vec<Domain>`          |
| `delete` | `delete_count(filter)`   | `u64`                  |
| `upsert` | `upsert(entity)`         | `()`                   |
| `upsert` | `batch_upsert(entities)` | `()`                   |

## ❓ 常见问题

### Q: 如何添加自定义方法？

```rust
impl UserRepository {
    pub async fn find_active_users(&self) -> Result<Vec<User>, DbErr> {
        let result = self.search(filter::Search {
            active: Some(true),
            ..Default::default()
        }).await?;
        Ok(result.items)
    }
}
```

### Q: 如何添加更多过滤条件？

```rust
pub struct Search {
    pub username: Option<String>,
    pub created_after: Option<DateTime>,
    pub pagination: Option<Pagination>,
}

impl SearchFilter<Entity> for Search {
    fn to_select(self) -> Select<Entity> {
        let mut select = Entity::find();

        if let Some(username) = self.username {
            select = select.filter(Column::Username.contains(&username));
        }
        if let Some(created_after) = self.created_after {
            select = select.filter(Column::CreatedAt.gte(created_after));
        }

        select
    }

    fn pagination(&self) -> Option<Pagination> {
        self.pagination
    }
}
```

## 🎉 开始使用

查看 [examples/](./examples/) 目录获取更多示例。

