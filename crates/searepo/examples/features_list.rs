// 示例：功能列表选择
//
// 演示如何使用 include/exclude/all 参数控制生成的方法。
//
// 功能选择优先级：all > include > exclude > 默认
//
// 可用功能：find, search, insert, update, delete, upsert

use sea_orm::{DatabaseConnection, Select, entity::prelude::*};
use searepo::{DeleteFilter, FindFilter, OnConflict, Repository, SearchFilter, Upsertable};
use std::sync::Arc;

// ============================================================================
// Entity 定义
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
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
}

impl From<entities::user::Model> for User {
    fn from(model: entities::user::Model) -> Self {
        Self {
            id: model.id,
            username: model.username,
            email: model.email,
        }
    }
}

impl From<User> for entities::user::ActiveModel {
    fn from(user: User) -> Self {
        use sea_orm::ActiveValue::Set;
        Self {
            id: Set(user.id),
            username: Set(user.username),
            email: Set(user.email),
        }
    }
}

// ============================================================================
// 示例 1: 默认行为（只有 find 和 search）
// ============================================================================

#[derive(Repository)]
#[repository(entity = "entities::user", domain = "User")]
pub struct DefaultUserRepository {
    db: Arc<DatabaseConnection>,
}

// 默认生成：
// - find(), load(), exists()
// - search(), count()

pub mod default_filter {
    use super::*;
    use entities::user::Entity;

    pub enum Find {
        ById(i64),
    }

    impl FindFilter<Entity> for Find {
        fn to_select(self) -> Select<Entity> {
            Entity::find_by_id(match self {
                Find::ById(id) => id,
            })
        }
    }

    #[derive(Default)]
    pub struct Search;

    impl SearchFilter<Entity> for Search {
        fn to_select(self) -> Select<Entity> {
            Entity::find()
        }
    }
}

// ============================================================================
// 示例 2: 只读 Repository（使用旧版 features 语法）
// ============================================================================

#[derive(Repository)]
#[repository(
    entity = "entities::user",
    domain = "User",
    include = ["find", "search"]
)]
pub struct ReadOnlyUserRepository {
    db: Arc<DatabaseConnection>,
}

// 生成：find(), load(), exists(), search(), count()
// 不需要实现 From<User> for ActiveModel

// ============================================================================
// 示例 3: 只写 Repository（日志系统）
// ============================================================================

#[derive(Repository)]
#[repository(
    entity = "entities::user",
    domain = "User",
    include = ["insert"]
)]
pub struct WriteOnlyUserRepository {
    db: Arc<DatabaseConnection>,
}

// 生成：insert(), batch_insert()
// 不需要实现 FindFilter 和 SearchFilter

// ============================================================================
// 示例 4: 数据导入 Repository
// ============================================================================

#[derive(Repository)]
#[repository(
    entity = "entities::user",
    domain = "User",
    include = ["upsert"]
)]
pub struct UserImportRepository {
    db: Arc<DatabaseConnection>,
}

// 生成：upsert(), batch_upsert()

impl Upsertable<entities::user::Entity> for User {
    fn on_conflict() -> OnConflict {
        use entities::user::Column;
        OnConflict::columns([Column::Email])
            .update_columns([Column::Username])
            .to_owned()
    }
}

// ============================================================================
// 示例 5: 完整 CRUD Repository
// ============================================================================

#[derive(Repository)]
#[repository(entity = "entities::user", domain = "User", all = true)]
pub struct FullUserRepository {
    db: Arc<DatabaseConnection>,
}

// 生成所有方法：
// - find(), load(), exists()
// - search(), count()
// - insert(), batch_insert()
// - update()
// - delete(), delete_count()
// - upsert(), batch_upsert()

pub mod full_filter {
    use super::*;
    use entities::user::Entity;

    pub enum Find {
        ById(i64),
    }

    impl FindFilter<Entity> for Find {
        fn to_select(self) -> Select<Entity> {
            Entity::find_by_id(match self {
                Find::ById(id) => id,
            })
        }
    }

    #[derive(Default)]
    pub struct Search;

    impl SearchFilter<Entity> for Search {
        fn to_select(self) -> Select<Entity> {
            Entity::find()
        }
    }

    pub enum Delete {
        ById(i64),
    }

    impl DeleteFilter<Entity> for Delete {
        fn to_select(self) -> Select<Entity> {
            Entity::find_by_id(match self {
                Delete::ById(id) => id,
            })
        }
    }
}

// ============================================================================
// 示例 6: CQRS - Command Repository
// ============================================================================

#[derive(Repository)]
#[repository(
    entity = "entities::user",
    domain = "User",
    include = ["insert", "update", "delete"]
)]
pub struct UserCommandRepository {
    db: Arc<DatabaseConnection>,
}

// 生成：insert(), batch_insert(), update(), delete(), delete_count()

// ============================================================================
// 示例 7: CQRS - Query Repository
// ============================================================================

#[derive(Repository)]
#[repository(
    entity = "entities::user",
    domain = "User",
    include = ["find", "search"]
)]
pub struct UserQueryRepository {
    db: Arc<DatabaseConnection>,
}

// 生成：find(), load(), exists(), search(), count()

// ============================================================================
// 示例 8: 排除功能（exclude）
// ============================================================================

#[derive(Repository)]
#[repository(
    entity = "entities::user",
    domain = "User",
    exclude = ["delete", "upsert"]
)]
pub struct NoDeleteUserRepository {
    db: Arc<DatabaseConnection>,
}

// 生成除 delete 和 upsert 外的所有功能

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 功能列表示例\n");

    println!("✅ 示例 1: 默认 Repository");
    println!("   #[repository(entity = \"...\", domain = \"...\")]");
    println!("   → 生成: find, load, exists, search, count");
    println!("   → 默认只启用查询功能\n");

    println!("✅ 示例 2: 只读 Repository (include)");
    println!("   include = [\"find\", \"search\"]");
    println!("   → 明确指定只读\n");

    println!("✅ 示例 3: 只写 Repository");
    println!("   include = [\"insert\"]");
    println!("   → 只能写入，适合日志系统\n");

    println!("✅ 示例 4: 数据导入 Repository");
    println!("   include = [\"upsert\"]");
    println!("   → 只有 upsert，适合数据同步\n");

    println!("✅ 示例 5: 完整 CRUD Repository");
    println!("   all = true");
    println!("   → 所有功能\n");

    println!("✅ 示例 6-7: CQRS 模式");
    println!("   Command: include = [\"insert\", \"update\", \"delete\"]");
    println!("   Query: include = [\"find\", \"search\"]\n");

    println!("✅ 示例 8: 排除功能 (exclude)");
    println!("   exclude = [\"delete\", \"upsert\"]");
    println!("   → 启用除 delete 和 upsert 外的所有功能\n");

    println!("💡 可用功能:");
    println!("   - find: find(), load(), exists()");
    println!("   - search: search(), count()");
    println!("   - insert: insert(), batch_insert()");
    println!("   - update: update()");
    println!("   - delete: delete(), delete_count()");
    println!("   - upsert: upsert(), batch_upsert()\n");

    println!("🎯 优先级: all > include > exclude > 默认");

    Ok(())
}
