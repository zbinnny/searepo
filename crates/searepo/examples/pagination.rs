// 示例：分页查询
//
// 演示如何使用 Repository 的分页功能。
//
// 支持两种分页方式：
// - PageOffset: 传统页码分页（页码从 1 开始）
// - Cursor: 游标分页（适用于大数据集）

use sea_orm::{
    ColumnTrait, Database, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Select,
    entity::prelude::*,
};
use searepo::{FindFilter, Pagination, Repository, SearchFilter, SearchResult};
use std::sync::Arc;

// 使用 sea_orm 内部的 chrono
use sea_orm::prelude::DateTimeWithTimeZone;

// ============================================================================
// Entity 和 Domain 模型定义
// ============================================================================

pub mod entities {
    pub mod post {
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "posts")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i64,
            pub title: String,
            pub content: String,
            pub author_id: i64,
            pub published: bool,
            pub created_at: DateTimeWithTimeZone,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }
}

#[derive(Debug, Clone)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author_id: i64,
    pub published: bool,
    pub created_at: DateTimeWithTimeZone,
}

impl From<entities::post::Model> for Post {
    fn from(model: entities::post::Model) -> Self {
        Self {
            id: model.id,
            title: model.title,
            content: model.content,
            author_id: model.author_id,
            published: model.published,
            created_at: model.created_at,
        }
    }
}

impl From<Post> for entities::post::ActiveModel {
    fn from(post: Post) -> Self {
        use sea_orm::ActiveValue::*;
        Self {
            id: Set(post.id),
            title: Set(post.title),
            content: Set(post.content),
            author_id: Set(post.author_id),
            published: Set(post.published),
            created_at: Set(post.created_at),
        }
    }
}

// ============================================================================
// Repository with Pagination Support
// ============================================================================

#[derive(Repository)]
#[repository(entity = "entities::post", domain = "Post")]
pub struct PostRepository {
    db: Arc<DatabaseConnection>,
}

pub mod filter {
    use super::*;
    use entities::post::{Column, Entity};

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

    #[derive(Default, Clone)]
    pub struct Search {
        pub title: Option<String>,
        pub author_id: Option<i64>,
        pub published: Option<bool>,
        pub pagination: Option<Pagination>,
    }

    impl SearchFilter<Entity> for Search {
        fn to_select(self) -> Select<Entity> {
            let mut select = Entity::find();

            if let Some(title) = self.title {
                select = select.filter(Column::Title.contains(&title));
            }

            if let Some(author_id) = self.author_id {
                select = select.filter(Column::AuthorId.eq(author_id));
            }

            if let Some(published) = self.published {
                select = select.filter(Column::Published.eq(published));
            }

            select
        }

        fn pagination(&self) -> Option<Pagination> {
            self.pagination
        }
    }
}

// ============================================================================
// Repository 分页方法
// ============================================================================

impl PostRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 使用 PageOffset 分页搜索文章
    pub async fn search_paginated(
        &self,
        filter: filter::Search,
        page: u64,
        page_size: u64,
    ) -> Result<SearchResult<Post>, DbErr> {
        // 创建带分页的搜索过滤器
        let filter_with_pagination = filter::Search {
            pagination: Some(Pagination::PageOffset {
                page,
                size: page_size,
            }),
            ..filter
        };
        self.search(filter_with_pagination).await
    }

    /// 获取已发布文章（分页）
    pub async fn get_published_posts(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<SearchResult<Post>, DbErr> {
        self.search_paginated(
            filter::Search {
                published: Some(true),
                ..Default::default()
            },
            page,
            page_size,
        )
        .await
    }

    /// 获取作者的文章（分页）
    pub async fn get_posts_by_author(
        &self,
        author_id: i64,
        page: u64,
        page_size: u64,
    ) -> Result<SearchResult<Post>, DbErr> {
        self.search_paginated(
            filter::Search {
                author_id: Some(author_id),
                ..Default::default()
            },
            page,
            page_size,
        )
        .await
    }

    /// 按标题搜索（分页）
    pub async fn search_by_title(
        &self,
        title_pattern: &str,
        page: u64,
        page_size: u64,
    ) -> Result<SearchResult<Post>, DbErr> {
        self.search_paginated(
            filter::Search {
                title: Some(title_pattern.to_string()),
                ..Default::default()
            },
            page,
            page_size,
        )
        .await
    }
}

// ============================================================================
// 使用示例
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📄 分页查询示例\n");

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());

    let db = Database::connect(&database_url).await?;
    let db = Arc::new(db);

    let _repo = PostRepository::new(db.clone());

    // 示例 1: 基础分页
    println!("📄 示例 1: 获取第一页文章");
    // let result = repo.search_paginated(filter::Search::default(), 1, 10).await?;
    // println!("当前页数据量: {}", result.items.len());
    // println!("总数量: {}", result.total_count);
    // println!("当前页: {}", result.page);
    // println!("总页数: {}", result.total_page);
    // println!("是否有更多: {}", result.has_more());

    // 示例 2: 获取已发布文章
    println!("📰 示例 2: 获取已发布文章（第 1 页）");
    // let published = repo.get_published_posts(1, 20).await?;
    // println!("找到 {} 篇已发布文章", published.items.len());
    // println!("共 {} 篇", published.total_count);

    // 示例 3: 获取作者文章
    println!("👤 示例 3: 获取作者文章");
    // let author_posts = repo.get_posts_by_author(1, 1, 10).await?;
    // println!("作者共有 {} 篇文章", author_posts.total_count);
    // println!("第 {}/{} 页", author_posts.page, author_posts.total_page);

    // 示例 4: 搜索文章
    println!("🔍 示例 4: 按标题搜索");
    // let search_results = repo.search_by_title("rust", 1, 10).await?;
    // println!("找到 {} 篇匹配 'rust' 的文章", search_results.total_count);
    // for post in search_results.items {
    //     println!("  - {}", post.title);
    // }

    // 示例 5: 遍历所有页
    println!("⏭️  示例 5: 遍历所有页");
    // let page_size = 10;
    // let mut current_page = 1;
    // loop {
    //     let result = repo.search_paginated(filter::Search::default(), current_page, page_size).await?;
    //     println!("第 {}/{} 页, 数据量: {}", result.page, result.total_page, result.items.len());
    //     if !result.has_more() {
    //         break;
    //     }
    //     current_page += 1;
    // }

    println!("✨ 分页示例完成！");
    println!("\n💡 提示: 取消注释以上代码运行实际的数据库操作。");

    Ok(())
}
