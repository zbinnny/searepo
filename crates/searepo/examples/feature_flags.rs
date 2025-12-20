// 示例：功能选择
//
// 演示如何使用功能开关按需生成 Repository 方法。
//
// 功能选择规则：
// - 默认：只生成 find + search
// - all = true：生成所有功能
// - include = ["..."]：只生成指定功能
// - exclude = ["..."]：排除指定功能
//
// 可用功能：find, search, insert, update, delete, upsert

use sea_orm::{ColumnTrait, DatabaseConnection, Select, entity::prelude::*};
use searepo::{FindFilter, Repository, SearchFilter};
use std::sync::Arc;

// ============================================================================
// Entity 和 Domain 模型定义
// ============================================================================

pub mod entities {
    pub mod product {
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "products")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i64,
            pub name: String,
            pub price: i64,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }
}

#[derive(Debug, Clone)]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub price: i64,
}

impl From<entities::product::Model> for Product {
    fn from(model: entities::product::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            price: model.price,
        }
    }
}

impl From<Product> for entities::product::ActiveModel {
    fn from(product: Product) -> Self {
        use sea_orm::ActiveValue::Set;
        Self {
            id: Set(product.id),
            name: Set(product.name),
            price: Set(product.price),
        }
    }
}

// ============================================================================
// 示例 1: 只读 Repository（使用 include 语法）
// ============================================================================

#[derive(Repository)]
#[repository(
    entity = "entities::product",
    domain = "Product",
    include = ["find", "search"]
)]
pub struct ReadOnlyProductRepository {
    db: Arc<DatabaseConnection>,
}

pub mod readonly_filter {
    use super::*;
    use entities::product::{Column, Entity};

    pub enum Find {
        ById(i64),
    }

    impl FindFilter<Entity> for Find {
        fn to_select(self) -> Select<Entity> {
            match self {
                Find::ById(id) => Entity::find_by_id(id),
            }
        }
    }

    #[derive(Default)]
    pub struct Search {
        pub name: Option<String>,
    }

    impl SearchFilter<Entity> for Search {
        fn to_select(self) -> Select<Entity> {
            let mut select = Entity::find();
            if let Some(name) = self.name {
                select = select.filter(Column::Name.contains(&name));
            }
            select
        }
    }
}

// ============================================================================
// 示例 2: 只写 Repository（只有插入功能）
// ============================================================================

#[derive(Repository)]
#[repository(
    entity = "entities::product",
    domain = "Product",
    include = ["insert"]
)]
pub struct InsertOnlyProductRepository {
    db: Arc<DatabaseConnection>,
}

// ============================================================================
// 示例 3: 完整 Repository（所有功能，默认行为）
// ============================================================================

// 不指定功能开关时，默认只启用 find + search
#[derive(Repository)]
#[repository(entity = "entities::product", domain = "Product")]
pub struct FullProductRepository {
    db: Arc<DatabaseConnection>,
}

pub mod full_filter {
    use super::*;
    use entities::product::{Column, Entity};
    use searepo::DeleteFilter;

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
    pub struct Search {
        pub name: Option<String>,
    }

    impl SearchFilter<Entity> for Search {
        fn to_select(self) -> Select<Entity> {
            let mut select = Entity::find();
            if let Some(name) = self.name {
                select = select.filter(Column::Name.contains(&name));
            }
            select
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
// 主函数示例
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 功能选择示例\n");

    println!("✅ 示例 1: 只读 Repository");
    println!("   - 包含: find(), load(), exists(), search(), count()");
    println!("   - 不包含: insert(), update(), delete(), upsert()");
    println!("   - 使用场景: 查询服务，只读数据库副本\n");

    println!("✅ 示例 2: 只写 Repository");
    println!("   - 包含: insert(), batch_insert()");
    println!("   - 不包含: find(), search(), update(), delete(), upsert()");
    println!("   - 使用场景: 日志写入服务，事件存储\n");

    println!("✅ 示例 3: 默认 Repository");
    println!("   - 默认只包含: find + search 相关方法");
    println!("   - 使用场景: 大多数查询场景\n");

    println!("💡 功能选择的优势:");
    println!("   ✓ 只生成需要的方法");
    println!("   ✓ 不需要实现不必要的 trait");
    println!("   ✓ 减少编译时间");
    println!("   ✓ 更清晰的代码意图");

    Ok(())
}
