use proc_macro::TokenStream;
use syn::parse_macro_input;

mod searepo;

/// 派生宏：为 Repository 结构体自动生成 CRUD 方法
///
/// # 基本用法
///
/// ```ignore
/// use searepo::Repository;
/// use std::sync::Arc;
/// use sea_orm::DatabaseConnection;
///
/// #[derive(Repository)]
/// #[repository(entity = "crate::entities::user", domain = "User")]
/// pub struct UserRepository {
///     db: Arc<DatabaseConnection>,
/// }
/// ```
///
/// # 功能选择
///
/// ## 默认行为（只生成 find + search）
///
/// ```ignore
/// #[repository(entity = "...", domain = "...")]
/// // 生成: find(), load(), exists(), search(), count()
/// ```
///
/// ## 启用所有功能
///
/// ```ignore
/// #[repository(entity = "...", domain = "...", all = true)]
/// ```
///
/// ## 指定启用的功能 (include)
///
/// ```ignore
/// #[repository(entity = "...", domain = "...", include = ["find", "insert", "delete"])]
/// ```
///
/// ## 排除某些功能 (exclude)
///
/// ```ignore
/// #[repository(entity = "...", domain = "...", exclude = ["delete", "upsert"])]
/// // 启用除 delete 和 upsert 外的所有功能
/// ```
///
/// # 可用功能
///
/// | Feature    | 生成的方法                          |
/// |------------|-------------------------------------|
/// | `find`     | `find()`, `load()`, `exists()`      |
/// | `search`   | `search()`, `count()`               |
/// | `insert`   | `insert()`, `batch_insert()`        |
/// | `update`   | `update()`                          |
/// | `delete`   | `delete()`, `delete_count()`        |
/// | `upsert`   | `upsert()`, `batch_upsert()`        |
///
/// **优先级**: `all` > `include` > `exclude` > 默认 (`find` + `search`)
///
/// # 必需实现的 Trait
///
/// 根据启用的功能，需要实现相应的 trait：
///
/// - `find` → `FindFilter`
/// - `search` → `SearchFilter`
/// - `insert` / `update` → `From<Domain> for ActiveModel`, `From<Model> for Domain`
/// - `delete` → `DeleteFilter`
/// - `upsert` → `Upsertable`, `From<Domain> for ActiveModel`
#[proc_macro_derive(Repository, attributes(repository))]
pub fn derive_repository(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    searepo::derive_searepo_impl(input)
}
