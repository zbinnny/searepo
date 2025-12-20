// 示例：基础 CRUD 操作
//
// 演示如何使用 Repository 派生宏自动生成 User 实体的 CRUD 操作。
//
// 功能说明：
// - find(): 查找单个实体（返回 Option）
// - load(): 查找单个实体（必须存在）
// - exists(): 检查是否存在
// - search(): 搜索列表（支持分页）
// - insert(): 插入单个实体
// - batch_insert(): 批量插入（自动分块，使用事务）
// - update(): 更新实体
// - delete(): 删除实体（返回被删除的实体列表）
// - delete_count(): 删除实体（只返回数量）

use chrono::Utc;
use sea_orm::{
    ColumnTrait, Database, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Select,
    entity::prelude::*,
};
use searepo::{DeleteFilter, FindFilter, Pagination, Repository, SearchFilter, SearchResult};
use std::sync::Arc;

// Use DateTimeWithTimeZone from sea_orm
use sea_orm::prelude::DateTimeWithTimeZone;

// ============================================================================
// 1. 定义 SeaORM Entity（数据库实体）
// ============================================================================

pub mod entities {
    pub mod user {
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "users")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i64,
            pub username: String,
            pub email: String,
            pub active: bool,
            pub created_at: DateTimeWithTimeZone,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }
}

// ============================================================================
// 2. 定义领域模型
// ============================================================================

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub active: bool,
    pub created_at: DateTimeWithTimeZone,
}

// 数据库模型 -> 领域模型
impl From<entities::user::Model> for User {
    fn from(model: entities::user::Model) -> Self {
        Self {
            id: model.id,
            username: model.username,
            email: model.email,
            active: model.active,
            created_at: model.created_at,
        }
    }
}

// 领域模型 -> ActiveModel（用于插入/更新）
impl From<User> for entities::user::ActiveModel {
    fn from(user: User) -> Self {
        use sea_orm::ActiveValue::*;
        Self {
            id: Set(user.id),
            username: Set(user.username),
            email: Set(user.email),
            active: Set(user.active),
            created_at: Set(user.created_at),
        }
    }
}

// ============================================================================
// 3. 定义 Repository 及过滤器
// ============================================================================

#[derive(Repository)]
#[repository(entity = "entities::user", domain = "User")]
pub struct UserRepository {
    db: Arc<DatabaseConnection>,
}

pub mod filter {
    use super::*;
    use entities::user::{Column, Entity};

    /// 单条查询过滤器
    pub enum Find {
        ById(i64),
        ByEmail(String),
        ByUsername(String),
    }

    impl FindFilter<Entity> for Find {
        fn to_select(self) -> Select<Entity> {
            match self {
                Find::ById(id) => Entity::find_by_id(id),
                Find::ByEmail(email) => Entity::find().filter(Column::Email.eq(email)),
                Find::ByUsername(username) => Entity::find().filter(Column::Username.eq(username)),
            }
        }
    }

    /// 列表搜索过滤器
    #[derive(Default)]
    pub struct Search {
        pub username: Option<String>,
        pub email: Option<String>,
        pub active: Option<bool>,
        pub pagination: Option<Pagination>,
    }

    impl SearchFilter<Entity> for Search {
        fn to_select(self) -> Select<Entity> {
            let mut select = Entity::find();

            if let Some(username) = self.username {
                select = select.filter(Column::Username.contains(&username));
            }

            if let Some(email) = self.email {
                select = select.filter(Column::Email.contains(&email));
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

    /// 删除过滤器
    pub enum Delete {
        ById(i64),
        ByEmail(String),
    }

    impl DeleteFilter<Entity> for Delete {
        fn to_select(self) -> Select<Entity> {
            match self {
                Delete::ById(id) => Entity::find_by_id(id),
                Delete::ByEmail(email) => Entity::find().filter(Column::Email.eq(email)),
            }
        }
    }
}

// ============================================================================
// 4. 添加自定义方法
// ============================================================================

impl UserRepository {
    /// 创建 Repository 实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 查找所有活跃用户
    pub async fn find_active_users(&self) -> Result<Vec<User>, DbErr> {
        let result = self
            .search(filter::Search {
                active: Some(true),
                ..Default::default()
            })
            .await?;
        Ok(result.items)
    }

    /// 按用户名模式搜索
    pub async fn find_by_username_pattern(&self, pattern: &str) -> Result<Vec<User>, DbErr> {
        let result = self
            .search(filter::Search {
                username: Some(pattern.to_string()),
                ..Default::default()
            })
            .await?;
        Ok(result.items)
    }

    /// 分页查询用户
    pub async fn find_users_paginated(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<SearchResult<User>, DbErr> {
        self.search(filter::Search {
            pagination: Some(Pagination::PageOffset {
                page,
                size: page_size,
            }),
            ..Default::default()
        })
        .await
    }

    /// 统计用户总数（自定义实现，避免与宏生成的 count 冲突）
    pub async fn total_count(&self) -> Result<u64, DbErr> {
        use entities::user::Entity;
        Entity::find().count(&*self.db).await
    }

    /// 检查邮箱是否已存在
    pub async fn email_exists(&self, email: &str) -> Result<bool, DbErr> {
        Ok(self
            .find(filter::Find::ByEmail(email.to_string()))
            .await?
            .is_some())
    }
}

// ============================================================================
// 5. 使用示例
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to database
    // Update this connection string for your database
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());

    let db = Database::connect(&database_url).await?;
    let db = Arc::new(db);

    // Create repository
    let _repo = UserRepository::new(db.clone());

    println!("🚀 Repository Pattern Example\n");

    // Example 1: Insert a new user
    println!("📝 Example 1: Insert a new user");
    let _new_user = User {
        id: 0, // Will be auto-generated
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        active: true,
        created_at: Utc::now().fixed_offset(),
    };

    // Uncomment to actually insert:
    // let created_user = repo.insert(new_user).await?;
    // println!("✅ Created user: {:?}\n", created_user);

    // Example 2: Find a user by ID
    println!("🔍 Example 2: Find a user by ID");
    // let user = repo.find(filter::Find::ById(1)).await?;
    // if let Some(user) = user {
    //     println!("✅ Found user: {:?}\n", user);
    // } else {
    //     println!("❌ User not found\n");
    // }

    // Example 3: Find a user by email (must exist)
    println!("🔍 Example 3: Find a user by email (must exist)");
    // let user = repo.find_one(filter::Find::ByEmail("alice@example.com".to_string())).await?;
    // println!("✅ Found user: {:?}\n", user);

    // Example 4: Search users with filters
    println!("🔎 Example 4: Search users with filters");
    // let users = repo.search(filter::Search {
    //     username: Some("ali".to_string()),
    //     active: Some(true),
    //     ..Default::default()
    // }).await?;
    // println!("✅ Found {} users\n", users.len());

    // Example 5: Find all active users (custom method)
    println!("👥 Example 5: Find all active users");
    // let active_users = repo.find_active_users().await?;
    // println!("✅ Found {} active users\n", active_users.len());

    // Example 6: Update a user
    println!("✏️  Example 6: Update a user");
    // let mut user = repo.find_one(filter::Find::ById(1)).await?;
    // user.username = "alice_updated".to_string();
    // let updated_user = repo.update(user).await?;
    // println!("✅ Updated user: {:?}\n", updated_user);

    // Example 7: Batch insert
    println!("📦 Example 7: Batch insert");
    // let users = vec![
    //     User {
    //         id: 0,
    //         username: "bob".to_string(),
    //         email: "bob@example.com".to_string(),
    //         active: true,
    //         created_at: chrono::Utc::now().into(),
    //     },
    //     User {
    //         id: 0,
    //         username: "charlie".to_string(),
    //         email: "charlie@example.com".to_string(),
    //         active: true,
    //         created_at: chrono::Utc::now().into(),
    //     },
    // ];
    // let created_users = repo.insert_many(users).await?;
    // println!("✅ Created {} users\n", created_users.len());

    // Example 8: Check if email exists
    println!("🔍 Example 8: Check if email exists");
    // let exists = repo.email_exists("alice@example.com").await?;
    // println!("✅ Email exists: {}\n", exists);

    // Example 9: Count users
    println!("🔢 Example 9: Count users");
    // let count = repo.count().await?;
    // println!("✅ Total users: {}\n", count);

    // Example 10: Delete a user
    println!("🗑️  Example 10: Delete a user");
    // let deleted_users = repo.delete(filter::Delete::ById(1)).await?;
    // println!("✅ Deleted {} user(s)", deleted_users.len());
    // for user in deleted_users {
    //     println!("   - Deleted: {} ({})", user.username, user.email);
    // }

    // Or just get the count
    // let count = repo.delete_count(filter::Delete::ById(1)).await?;
    // println!("✅ Deleted {} user(s)", count);

    println!("✨ All examples completed!");
    println!("\n💡 Note: Uncomment the actual database operations to run them.");

    Ok(())
}
