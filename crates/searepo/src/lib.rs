//! # Repository Pattern for SeaORM
//!
//! 通过派生宏自动为 Repository 结构体生成 CRUD 方法。
//!
//! ## 基本用法
//!
//! ```ignore
//! use searepo::{Repository, FindFilter, SearchFilter};
//!
//! #[derive(Repository)]
//! #[repository(entity = "entities::user", domain = "User")]
//! pub struct UserRepository {
//!     db: Arc<DatabaseConnection>,
//! }
//! ```
//!
//! ## 功能选择
//!
//! - 默认：只生成 `find` + `search` 相关方法
//! - `all = true`：生成所有方法
//! - `include = ["find", "insert"]`：只生成指定功能
//! - `exclude = ["delete"]`：排除指定功能

// Re-export async_trait for internal use
pub use sea_macros::Repository;
pub use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, ModelTrait,
    NotSet, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Select, Set, TransactionTrait,
    sea_query::OnConflict,
};

/// 分页策略
///
/// 支持两种分页方式：
/// - `PageOffset`: 传统的页码分页（页码从 1 开始）
/// - `Cursor`: 游标分页，适用于大数据集
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pagination {
    /// 页码分页
    ///
    /// - `page`: 页码（从 1 开始）
    /// - `size`: 每页大小
    PageOffset { page: u64, size: u64 },

    /// 游标分页
    ///
    /// - `limit`: 每次获取的数量
    ///
    /// 注意：游标条件应在 `SearchFilter::to_select()` 中处理
    Cursor { limit: u64 },
}

impl Pagination {
    /// 创建页码分页
    pub fn page(page: u64, size: u64) -> Self {
        Self::PageOffset { page, size }
    }

    /// 创建游标分页
    pub fn cursor(limit: u64) -> Self {
        Self::Cursor { limit }
    }
}

/// 搜索结果封装
///
/// 包含查询结果和分页元数据。
///
/// # 字段说明
///
/// - `total_count`: 符合条件的总记录数
/// - `total_page`: 总页数
/// - `page`: 当前页码
/// - `size`: 当前页大小
/// - `items`: 当前页的数据
#[derive(Debug, Clone)]
pub struct SearchResult<T> {
    /// 总数量
    pub total_count: u64,
    /// 总页数
    pub total_page: u64,
    /// 当前页码
    pub page: u64,
    /// 当前页大小
    pub size: u64,
    /// 当前页数据
    pub items: Vec<T>,
}

impl<T> SearchResult<T> {
    /// 创建非分页查询结果
    pub fn all(items: Vec<T>) -> Self {
        let total_count = items.len() as u64;
        Self {
            total_count,
            total_page: 1,
            page: 1,
            size: total_count,
            items,
        }
    }

    /// 创建分页查询结果
    pub fn new_pagination(
        items: Vec<T>,
        total_count: u64,
        total_page: u64,
        page: u64,
        size: u64,
    ) -> Self {
        Self {
            total_count,
            total_page,
            page,
            size,
            items,
        }
    }

    /// 是否有更多数据
    pub fn has_more(&self) -> bool {
        self.total_page > self.page
    }

    /// 当前结果数量
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// 结果是否为空
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// 单条查询过滤器 Trait
///
/// 用于 `find()`、`load()`、`exists()` 方法。
///
/// # 示例
///
/// ```ignore
/// use searepo::FindFilter;
/// use sea_orm::*;
///
/// pub enum UserFind {
///     ById(i64),
///     ByEmail(String),
/// }
///
/// impl FindFilter<Entity> for UserFind {
///     fn to_select(self) -> Select<Entity> {
///         match self {
///             UserFind::ById(id) => Entity::find_by_id(id),
///             UserFind::ByEmail(email) => Entity::find().filter(Column::Email.eq(email)),
///         }
///     }
/// }
/// ```
pub trait FindFilter<E: sea_orm::EntityTrait> {
    /// 转换为 SeaORM Select 查询
    fn to_select(self) -> sea_orm::Select<E>;
}

/// 列表搜索过滤器 Trait
///
/// 用于 `search()`、`count()` 方法，支持分页。
///
/// # 示例
///
/// ```ignore
/// use searepo::{SearchFilter, Pagination};
/// use sea_orm::*;
///
/// #[derive(Default)]
/// pub struct UserSearch {
///     pub name: Option<String>,
///     pub active: Option<bool>,
///     pub pagination: Option<Pagination>,
/// }
///
/// impl SearchFilter<Entity> for UserSearch {
///     fn to_select(self) -> Select<Entity> {
///         let mut select = Entity::find();
///         if let Some(name) = self.name {
///             select = select.filter(Column::Name.contains(&name));
///         }
///         if let Some(active) = self.active {
///             select = select.filter(Column::Active.eq(active));
///         }
///         select
///     }
///
///     fn pagination(&self) -> Option<Pagination> {
///         self.pagination
///     }
/// }
/// ```
pub trait SearchFilter<E: sea_orm::EntityTrait> {
    /// 转换为 SeaORM Select 查询
    fn to_select(self) -> sea_orm::Select<E>;

    /// 返回分页参数，默认不分页
    fn pagination(&self) -> Option<Pagination> {
        None
    }
}

/// Upsert 冲突处理策略 Trait
///
/// 用于 `upsert()`、`batch_upsert()` 方法，定义冲突时的处理策略。
///
/// # 示例
///
/// ```ignore
/// use searepo::Upsertable;
/// use sea_orm::sea_query::OnConflict;
///
/// impl Upsertable<Entity> for User {
///     fn on_conflict() -> OnConflict {
///         use entities::user::Column;
///         // 当 email 冲突时，更新 username 和 updated_at
///         OnConflict::columns([Column::Email])
///             .update_columns([Column::Username, Column::UpdatedAt])
///             .to_owned()
///     }
/// }
/// ```
pub trait Upsertable<E: sea_orm::EntityTrait> {
    /// 定义冲突处理策略
    fn on_conflict() -> sea_orm::sea_query::OnConflict;
}

/// 删除过滤器 Trait
///
/// 用于 `delete()`、`delete_count()` 方法。
///
/// # 示例
///
/// ```ignore
/// use searepo::DeleteFilter;
/// use sea_orm::*;
///
/// pub enum UserDelete {
///     ById(i64),
///     ByUsername(String),
/// }
///
/// impl DeleteFilter<Entity> for UserDelete {
///     fn to_select(self) -> Select<Entity> {
///         match self {
///             UserDelete::ById(id) => Entity::find_by_id(id),
///             UserDelete::ByUsername(username) => {
///                 Entity::find().filter(Column::Username.eq(username))
///             }
///         }
///     }
/// }
/// ```
pub trait DeleteFilter<E: sea_orm::EntityTrait> {
    /// 转换为 Select 查询，Repository 会查找并删除匹配的实体
    fn to_select(self) -> sea_orm::Select<E>;
}
