use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::{DeriveInput, Ident, LitStr, Token};

/// Repository 宏的参数配置
pub(crate) struct RepositoryArgs {
    /// SeaORM Entity 模块路径
    pub entity: syn::Path,
    /// 领域模型类型
    pub domain: syn::Path,
    /// 功能开关配置
    pub features: RepositoryFeatures,
}

/// Repository 功能开关
///
/// 支持以下功能：
/// - `find`: find(), load(), exists()
/// - `search`: search(), count()
/// - `insert`: insert(), batch_insert()
/// - `update`: update()
/// - `delete`: delete(), delete_count()
/// - `upsert`: upsert(), batch_upsert()
pub(crate) struct RepositoryFeatures {
    enabled: HashSet<String>,
}

impl RepositoryFeatures {
    /// 所有可用的功能列表
    fn all_features() -> Vec<String> {
        vec![
            "find".to_string(),
            "search".to_string(),
            "insert".to_string(),
            "update".to_string(),
            "delete".to_string(),
            "upsert".to_string(),
        ]
    }

    /// 验证功能名是否有效
    fn is_valid_feature(feature: &str) -> bool {
        matches!(
            feature,
            "find" | "search" | "insert" | "update" | "delete" | "upsert"
        )
    }

    /// 解析 feature 列表（用于 include/exclude）
    fn parse_feature_list(content: ParseStream) -> syn::Result<Vec<String>> {
        let mut features = Vec::new();
        while !content.is_empty() {
            let feature: LitStr = content.parse()?;
            let feature_value = feature.value();

            // 验证 feature 名称
            if !Self::is_valid_feature(&feature_value) {
                return Err(syn::Error::new(
                    feature.span(),
                    format!(
                        "Invalid feature '{}'. Valid features are: find, search, insert, update, delete, upsert",
                        feature_value
                    ),
                ));
            }

            features.push(feature_value);

            if !content.is_empty() {
                content.parse::<Token![,]>()?;
            }
        }
        Ok(features)
    }

    /// 默认配置：启用 find + search
    fn default() -> Self {
        let mut enabled = HashSet::new();
        enabled.insert("find".to_string());
        enabled.insert("search".to_string());
        Self { enabled }
    }

    /// 启用所有功能
    fn all() -> Self {
        let enabled: HashSet<String> = Self::all_features().into_iter().collect();
        Self { enabled }
    }

    /// 从 include 列表创建（只启用列表中的功能）
    fn from_include(features: Vec<String>) -> Self {
        let enabled: HashSet<String> = features.into_iter().collect();
        Self { enabled }
    }

    /// 从 exclude 列表创建（启用除列表外的所有功能）
    fn from_exclude(exclude_features: Vec<String>) -> Self {
        let exclude_set: HashSet<String> = exclude_features.into_iter().collect();
        let enabled: HashSet<String> = Self::all_features()
            .into_iter()
            .filter(|f| !exclude_set.contains(f))
            .collect();
        Self { enabled }
    }

    /// 是否启用 find 功能
    pub fn has_find(&self) -> bool {
        self.enabled.contains("find")
    }

    /// 是否启用 search 功能
    pub fn has_search(&self) -> bool {
        self.enabled.contains("search")
    }

    /// 是否启用 insert 功能
    pub fn has_insert(&self) -> bool {
        self.enabled.contains("insert")
    }

    /// 是否启用 update 功能
    pub fn has_update(&self) -> bool {
        self.enabled.contains("update")
    }

    /// 是否启用 delete 功能
    pub fn has_delete(&self) -> bool {
        self.enabled.contains("delete")
    }

    /// 是否启用 upsert 功能
    pub fn has_upsert(&self) -> bool {
        self.enabled.contains("upsert")
    }
}

impl Parse for RepositoryArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut entity = None;
        let mut domain = None;
        let mut has_all = false;
        let mut include_list = Vec::new();
        let mut exclude_list = Vec::new();

        while !input.is_empty() {
            let name: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            if name == "entity" || name == "domain" {
                let path_str: LitStr = input.parse()?;
                let path: syn::Path = syn::parse_str(&path_str.value())?;

                if name == "entity" {
                    entity = Some(path);
                } else {
                    domain = Some(path);
                }
            } else if name == "all" {
                // all = true
                let value: syn::LitBool = input.parse()?;
                has_all = value.value;
            } else if name == "include" {
                // include = ["find", "search"]
                let content;
                syn::bracketed!(content in input);
                include_list = RepositoryFeatures::parse_feature_list(&content)?;
            } else if name == "exclude" {
                // exclude = ["delete", "upsert"]
                let content;
                syn::bracketed!(content in input);
                exclude_list = RepositoryFeatures::parse_feature_list(&content)?;
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        // 处理 features 优先级: all > include > exclude > 默认
        let features = if has_all {
            // 1. all = true，启用所有功能
            RepositoryFeatures::all()
        } else if !include_list.is_empty() {
            // 2. include 列表，只启用指定的功能
            RepositoryFeatures::from_include(include_list)
        } else if !exclude_list.is_empty() {
            // 3. exclude 列表，启用除指定外的所有功能
            RepositoryFeatures::from_exclude(exclude_list)
        } else {
            // 4. 默认：find + search
            RepositoryFeatures::default()
        };

        Ok(Self {
            entity: entity.ok_or_else(|| input.error("Missing required parameter: entity"))?,
            domain: domain.ok_or_else(|| input.error("Missing required parameter: domain"))?,
            features,
        })
    }
}

/// Repository 派生宏的实际实现
pub(crate) fn derive_searepo_impl(input: DeriveInput) -> TokenStream {
    let struct_name = &input.ident;

    // 解析 #[repository(...)] 属性
    let mut args = None;
    for attr in &input.attrs {
        if attr.path().is_ident("repository") {
            args = Some(attr.parse_args::<RepositoryArgs>().unwrap());
            break;
        }
    }

    let args = args.expect("Missing #[repository(entity = \"...\", domain = \"...\")] attribute");

    let entity_path = &args.entity;
    let domain_name = &args.domain;
    let features = &args.features;

    // 生成类型引用
    let entity_ref = quote! { #entity_path::Entity };
    let active_model_ref = quote! { #entity_path::ActiveModel };

    // 查找 db 字段
    let db_field = find_db_field(&input);

    // 根据功能开关生成方法
    let find_methods = if features.has_find() {
        quote! {
            /// 查找单个实体（返回 Option）
            pub async fn find<F>(&self, filter: F) -> ::std::result::Result<::std::option::Option<#domain_name>, ::sea_orm::DbErr>
            where
                F: searepo::FindFilter<#entity_ref>
            {
                let select = searepo::FindFilter::to_select(filter);
                let model = select.one(&*self.#db_field).await?;
                Ok(model.map(|m| m.into()))
            }

            /// 加载单个实体（必须存在）
            pub async fn load<F>(&self, filter: F) -> ::std::result::Result<#domain_name, ::sea_orm::DbErr>
            where
                F: searepo::FindFilter<#entity_ref>
            {
                self.find(filter).await?
                    .ok_or_else(|| ::sea_orm::DbErr::RecordNotFound("Record not found".to_string()))
            }

            /// 检查是否存在
            pub async fn exists<F>(&self, filter: F) -> ::std::result::Result<bool, ::sea_orm::DbErr>
            where
                F: searepo::FindFilter<#entity_ref>
            {
                use ::sea_orm::{EntityTrait, QuerySelect};
                let select = searepo::FindFilter::to_select(filter);
                let result = select.limit(1).one(&*self.#db_field).await?;
                Ok(result.is_some())
            }
        }
    } else {
        quote! {}
    };

    let search_methods = if features.has_search() {
        quote! {
            /// 搜索列表（支持分页和非分页）
            pub async fn search<F>(&self, filter: F) -> ::std::result::Result<searepo::SearchResult<#domain_name>, ::sea_orm::DbErr>
            where
                F: searepo::SearchFilter<#entity_ref>
            {
                let pagination = filter.pagination();
                let select = searepo::SearchFilter::to_select(filter);

                // 根据分页类型查询并构造结果
                let result = match pagination {
                    None => {
                        // 非分页查询
                        let models = select.all(&*self.#db_field).await?;
                        let total = models.len() as u64;
                        let items = models.into_iter().map(|m| m.into()).collect();

                        searepo::SearchResult::all(items)
                    }
                    Some(searepo::Pagination::PageOffset { page, size }) => {
                        // 页码分页
                        use searepo::PaginatorTrait;
                        let paginator = select.paginate(&*self.#db_field, size);
                        let models = paginator.fetch_page(page.saturating_sub(1)).await?;
                        let page_info = paginator.num_items_and_pages().await?;
                        let items = models.into_iter().map(|m| m.into()).collect();

                        searepo::SearchResult::new_pagination(
                            items,
                            page_info.number_of_items,
                            page_info.number_of_pages,
                            page,
                            size,
                        )
                    }
                    Some(searepo::Pagination::Cursor { limit }) => {
                        // 游标分页 - cursor 条件已在 to_select 中处理
                        use searepo::PaginatorTrait;
                        let paginator = select.paginate(&*self.#db_field, limit);
                        let models = paginator.fetch_page(0).await?;
                        let page_info = paginator.num_items_and_pages().await?;
                        let items = models.into_iter().map(|m| m.into()).collect();

                        searepo::SearchResult::new_pagination(
                            items,
                            page_info.number_of_items,
                            page_info.number_of_pages,
                            1,
                            limit,
                        )
                    }
                };

                Ok(result)
            }

            /// 统计数量
            pub async fn count<F>(&self, filter: F) -> ::std::result::Result<u64, ::sea_orm::DbErr>
            where
                F: searepo::SearchFilter<#entity_ref>
            {
                use ::sea_orm::EntityTrait;
                let select = searepo::SearchFilter::to_select(filter);
                select.count(&*self.#db_field).await
            }
        }
    } else {
        quote! {}
    };

    let insert_methods = if features.has_insert() {
        quote! {
            /// 插入单个实体
            pub async fn insert(&self, entity: #domain_name) -> ::std::result::Result<#domain_name, ::sea_orm::DbErr> {
                use ::sea_orm::ActiveModelTrait;
                let active_model: #active_model_ref = entity.into();
                let model = active_model.insert(&*self.#db_field).await?;
                Ok(model.into())
            }

            /// 批量插入（自动分块，使用事务）
            pub async fn batch_insert(&self, entities: ::std::vec::Vec<#domain_name>) -> ::std::result::Result<::std::vec::Vec<#domain_name>, ::sea_orm::DbErr> {
                use ::sea_orm::{ActiveModelTrait, EntityTrait, Iterable, TransactionTrait, TryInsertResult};

                if entities.is_empty() {
                    return Ok(::std::vec::Vec::new());
                }

                // 计算批量插入的最大条数
                // https://github.com/launchbadge/sqlx/issues/3464
                // https://www.postgresql.org/docs/current/limits.html
                let column_count = <#entity_ref as EntityTrait>::Column::iter().count() as u16;
                assert!(column_count > 0, "entity must have at least one column");

                let chunk_size = (u16::MAX / column_count) as usize;
                assert!(chunk_size > 0, "chunk_size must be greater than 0");

                let mut results = ::std::vec::Vec::with_capacity(entities.len());
                let mut models_iter = entities.into_iter().map(|e| -> #active_model_ref { e.into() });

                let tx = self.#db_field.begin().await?;
                loop {
                    let chunk_n: Vec<_> = models_iter.by_ref().take(chunk_size).collect();
                    if chunk_n.is_empty() {
                        break;
                    }

                    let result = #entity_ref::insert_many(chunk_n).on_conflict_do_nothing().exec_with_returning_many(&tx).await?;
                    match result {
                        TryInsertResult::Empty => {}
                        TryInsertResult::Conflicted => {}
                        TryInsertResult::Inserted(inserts) => {
                            results.extend(inserts.into_iter().map(|m| m.into()));
                        }
                    }
                }
                tx.commit().await?;

                Ok(results)
            }
        }
    } else {
        quote! {}
    };

    let update_methods = if features.has_update() {
        quote! {
            /// 更新实体
            pub async fn update(&self, entity: #domain_name) -> ::std::result::Result<#domain_name, ::sea_orm::DbErr> {
                use ::sea_orm::ActiveModelTrait;
                let active_model: #active_model_ref = entity.into();
                let model = active_model.update(&*self.#db_field).await?;
                Ok(model.into())
            }
        }
    } else {
        quote! {}
    };

    let upsert_methods = if features.has_upsert() {
        quote! {
            /// Upsert 单个实体（插入或更新）
            pub async fn upsert(&self, entity: #domain_name) -> ::std::result::Result<(), ::sea_orm::DbErr>
            where
                #domain_name: searepo::Upsertable<#entity_ref>
            {
                use ::sea_orm::EntityTrait;
                let active_model: #active_model_ref = entity.into();
                let on_conflict = <#domain_name as searepo::Upsertable<#entity_ref>>::on_conflict();

                match #entity_ref::insert(active_model)
                    .on_conflict(on_conflict)
                    .exec(&*self.#db_field)
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(::sea_orm::DbErr::RecordNotInserted) => Ok(()),
                    Err(e) => Err(e),
                }
            }

            /// 批量 Upsert（插入或更新，自动分块，使用事务）
            pub async fn batch_upsert(&self, entities: ::std::vec::Vec<#domain_name>) -> ::std::result::Result<(), ::sea_orm::DbErr>
            where
                #domain_name: searepo::Upsertable<#entity_ref>
            {
                use ::sea_orm::{EntityTrait, Iterable, TransactionTrait};

                if entities.is_empty() {
                    return Ok(());
                }

                // 计算批量 upsert 的最大条数
                let column_count = <#entity_ref as EntityTrait>::Column::iter().count() as u16;
                assert!(column_count > 0, "entity must have at least one column");

                let chunk_size = (u16::MAX / column_count) as usize;
                assert!(chunk_size > 0, "chunk_size must be greater than 0");

                let mut models_iter = entities.into_iter().map(|e: #domain_name| -> #active_model_ref { e.into() });
                let on_conflict = <#domain_name as searepo::Upsertable<#entity_ref>>::on_conflict();

                let tx = self.#db_field.begin().await?;
                loop {
                    let chunk: ::std::vec::Vec<_> = models_iter.by_ref().take(chunk_size).collect();
                    if chunk.is_empty() {
                        break;
                    }

                    #entity_ref::insert_many(chunk)
                        .on_conflict(on_conflict.clone())
                        .exec(&tx)
                        .await?;
                }

                tx.commit().await?;
                Ok(())
            }
        }
    } else {
        quote! {}
    };

    let delete_methods = if features.has_delete() {
        quote! {
            /// 删除实体（返回被删除的实体列表）
            pub async fn delete<F>(&self, filter: F) -> ::std::result::Result<::std::vec::Vec<#domain_name>, ::sea_orm::DbErr>
            where
                F: searepo::DeleteFilter<#entity_ref>
            {
                use ::sea_orm::{EntityTrait, ModelTrait};

                // 使用 filter 获取要删除的实体
                let select = searepo::DeleteFilter::to_select(filter);
                let models = select.all(&*self.#db_field).await?;

                // 转换为领域对象
                let entities: ::std::vec::Vec<#domain_name> = models.iter()
                    .map(|m| m.clone().into())
                    .collect();

                // 删除这些实体
                for model in models {
                    model.delete(&*self.#db_field).await?;
                }

                Ok(entities)
            }

            /// 删除实体（只返回删除数量）
            pub async fn delete_count<F>(&self, filter: F) -> ::std::result::Result<u64, ::sea_orm::DbErr>
            where
                F: searepo::DeleteFilter<#entity_ref>
            {
                use ::sea_orm::EntityTrait;

                let select = searepo::DeleteFilter::to_select(filter);
                let models = select.all(&*self.#db_field).await?;
                let count = models.len() as u64;

                for model in models {
                    use ::sea_orm::ModelTrait;
                    model.delete(&*self.#db_field).await?;
                }

                Ok(count)
            }
        }
    } else {
        quote! {}
    };

    // 组合所有方法
    let expanded = quote! {
        impl #struct_name {
            #find_methods
            #search_methods
            #insert_methods
            #update_methods
            #upsert_methods
            #delete_methods
        }
    };

    TokenStream::from(expanded)
}

/// 在结构体中查找数据库连接字段
fn find_db_field(input: &DeriveInput) -> Ident {
    if let syn::Data::Struct(data) = &input.data
        && let syn::Fields::Named(fields) = &data.fields
    {
        // 查找第一个名为 db 的字段
        for field in &fields.named {
            if let Some(ident) = &field.ident
                && ident == "db"
            {
                return ident.clone();
            }
        }
        // 如果没有找到 db，返回第一个字段
        if let Some(field) = fields.named.first()
            && let Some(ident) = &field.ident
        {
            return ident.clone();
        }
    }

    // 默认使用 "db"
    Ident::new("db", proc_macro2::Span::call_site())
}
