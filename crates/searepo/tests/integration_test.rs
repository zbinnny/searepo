// Integration tests for the Repository pattern
//
// These tests use an in-memory SQLite database

use sea_orm::{
    ColumnTrait, Database, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Select,
    entity::prelude::*,
};
use searepo::{DeleteFilter, FindFilter, OnConflict, Repository, SearchFilter, Upsertable};
use std::sync::Arc;
// ============================================================================
// Test Entity Definition
// ============================================================================

pub mod entities {
    pub mod test_user {
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
        #[sea_orm(table_name = "test_users")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i32,
            pub username: String,
            pub email: String,
            pub active: bool,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }
}

// ============================================================================
// Domain Model
// ============================================================================
// Domain model
#[derive(Debug, Clone, PartialEq)]
pub struct TestUser {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub active: bool,
}

// Implement Upsertable for TestUser
impl Upsertable<entities::test_user::Entity> for TestUser {
    fn on_conflict() -> OnConflict {
        // Default upsert strategy: do nothing on conflict
        OnConflict::new().do_nothing().to_owned()
    }
}

// For testing update strategy, create a wrapper type
#[derive(Debug, Clone)]
pub struct TestUserWithUpdate(pub TestUser);

impl From<TestUserWithUpdate> for entities::test_user::ActiveModel {
    fn from(wrapper: TestUserWithUpdate) -> Self {
        wrapper.0.into()
    }
}

impl Upsertable<entities::test_user::Entity> for TestUserWithUpdate {
    fn on_conflict() -> OnConflict {
        use entities::test_user::Column;
        OnConflict::new()
            .update_columns([Column::Username, Column::Email, Column::Active])
            .to_owned()
    }
}

impl From<entities::test_user::Model> for TestUser {
    fn from(model: entities::test_user::Model) -> Self {
        Self {
            id: model.id,
            username: model.username,
            email: model.email,
            active: model.active,
        }
    }
}

impl From<TestUser> for entities::test_user::ActiveModel {
    fn from(user: TestUser) -> Self {
        use sea_orm::ActiveValue::*;
        Self {
            id: Set(user.id),
            username: Set(user.username),
            email: Set(user.email),
            active: Set(user.active),
        }
    }
}

// ============================================================================
// Repository Definition
// ============================================================================
// Repository definition
#[derive(Repository)]
#[repository(entity = "entities::test_user", domain = "TestUser", all = true)]
pub struct TestUserRepository {
    db: Arc<DatabaseConnection>,
}

pub mod filter {
    use super::*;
    use entities::test_user::{Column, Entity};

    pub enum Find {
        ById(i32),
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

    #[derive(Default, Clone)]
    pub struct Search {
        pub username: Option<String>,
        pub active: Option<bool>,
        pub pagination: Option<(u64, u64)>,
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

        fn pagination(&self) -> Option<searepo::Pagination> {
            self.pagination
                .map(|(page, page_size)| searepo::Pagination::PageOffset {
                    page,
                    size: page_size,
                })
        }
    }

    pub enum Delete {
        ById(i32),
    }

    impl DeleteFilter<Entity> for Delete {
        fn to_select(self) -> Select<Entity> {
            match self {
                Delete::ById(id) => Entity::find_by_id(id),
            }
        }
    }
}

impl TestUserRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// Test Setup Helper
// ============================================================================

async fn setup_test_db() -> Result<Arc<DatabaseConnection>, DbErr> {
    let db = Database::connect("sqlite::memory:").await?;

    // Create table using raw SQL
    use sea_orm::ConnectionTrait;
    db.execute_unprepared(
        r#"
        CREATE TABLE test_users (
            id INTEGER PRIMARY KEY,
            username TEXT NOT NULL,
            email TEXT NOT NULL,
            active INTEGER NOT NULL
        )
        "#,
    )
    .await?;

    Ok(Arc::new(db))
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_insert_and_find() {
    let db = setup_test_db().await.unwrap();
    let repo = TestUserRepository::new(db);

    // Insert a user
    let user = TestUser {
        id: 1,
        username: "alice".to_string(),
        email: "alice@test.com".to_string(),
        active: true,
    };

    let created = repo.insert(user.clone()).await.unwrap();
    assert_eq!(created.username, "alice");

    // Find by ID
    let found = repo.find(filter::Find::ById(1)).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().email, "alice@test.com");
}

#[tokio::test]
async fn test_load() {
    let db = setup_test_db().await.unwrap();
    let repo = TestUserRepository::new(db);

    let user = TestUser {
        id: 1,
        username: "bob".to_string(),
        email: "bob@test.com".to_string(),
        active: true,
    };

    repo.insert(user).await.unwrap();

    // Find one (should succeed)
    let found = repo.load(filter::Find::ById(1)).await.unwrap();
    assert_eq!(found.username, "bob");

    // Find one (should fail)
    let not_found = repo.load(filter::Find::ById(999)).await;
    assert!(not_found.is_err());
}

#[tokio::test]
async fn test_search() {
    let db = setup_test_db().await.unwrap();
    let repo = TestUserRepository::new(db);

    // Insert multiple users
    let users = vec![
        TestUser {
            id: 1,
            username: "alice".to_string(),
            email: "alice@test.com".to_string(),
            active: true,
        },
        TestUser {
            id: 2,
            username: "bob".to_string(),
            email: "bob@test.com".to_string(),
            active: false,
        },
        TestUser {
            id: 3,
            username: "charlie".to_string(),
            email: "charlie@test.com".to_string(),
            active: true,
        },
    ];

    for user in users {
        repo.insert(user).await.unwrap();
    }

    // Search all users
    let result = repo.search(filter::Search::default()).await.unwrap();
    assert_eq!(result.items.len(), 3);
    assert_eq!(result.total_count, 3);

    // Search active only
    let result = repo
        .search(filter::Search {
            active: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(result.items.len(), 2);
    assert_eq!(result.total_count, 2);

    // Search by username pattern
    let result = repo
        .search(filter::Search {
            username: Some("ali".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.total_count, 1);
    assert_eq!(result.items[0].username, "alice");
}

#[tokio::test]
async fn test_pagination() {
    let db = setup_test_db().await.unwrap();
    let repo = TestUserRepository::new(db);

    // Insert 25 users
    for i in 1..=25 {
        let user = TestUser {
            id: i,
            username: format!("user{}", i),
            email: format!("user{}@test.com", i),
            active: i % 2 == 0,
        };
        repo.insert(user).await.unwrap();
    }

    // Get first page
    let result = repo
        .search(filter::Search {
            pagination: Some((1, 10)),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(result.items.len(), 10);
    assert_eq!(result.total_count, 25);
    assert_eq!(result.page, 1);
    assert_eq!(result.size, 10);
    assert_eq!(result.total_page, 3);
    assert!(result.has_more());

    // Get second page
    let result = repo
        .search(filter::Search {
            pagination: Some((2, 10)),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(result.items.len(), 10);
    assert!(result.has_more());

    // Get last page
    let result = repo
        .search(filter::Search {
            pagination: Some((3, 10)),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(result.items.len(), 5);
    assert!(!result.has_more());
}

#[tokio::test]
async fn test_cursor_pagination() {
    let db = setup_test_db().await.unwrap();
    let repo = TestUserRepository::new(db);

    // Insert 15 users
    for i in 1..=15 {
        let user = TestUser {
            id: i,
            username: format!("user{}", i),
            email: format!("user{}@test.com", i),
            active: true,
        };
        repo.insert(user).await.unwrap();
    }

    // Test cursor pagination with custom filter
    #[derive(Default)]
    struct CursorSearch {
        pub cursor: Option<u64>,
        pub limit: u64,
    }

    impl searepo::SearchFilter<entities::test_user::Entity> for CursorSearch {
        fn to_select(self) -> Select<entities::test_user::Entity> {
            use entities::test_user::Column;
            use searepo::QueryFilter;

            let mut select = entities::test_user::Entity::find();

            // 如果有 cursor，则过滤 id > cursor 的记录
            if let Some(cursor) = self.cursor {
                if cursor > 0 {
                    select = select.filter(Column::Id.gt(cursor as i32));
                }
            }

            select
        }

        fn pagination(&self) -> Option<searepo::Pagination> {
            Some(searepo::Pagination::Cursor { limit: self.limit })
        }
    }

    // First batch with cursor
    let result = repo
        .search(CursorSearch {
            cursor: Some(0),
            limit: 10,
        })
        .await
        .unwrap();

    assert_eq!(result.items.len(), 10);
    assert!(result.has_more()); // 还有更多数据

    // 手动管理游标：使用最后一条记录的 ID 作为下一个游标
    let last_id = result.items.last().map(|u| u.id as u64).unwrap();

    // Second batch using cursor from last record
    let result = repo
        .search(CursorSearch {
            cursor: Some(last_id),
            limit: 10,
        })
        .await
        .unwrap();

    // 由于有 cursor 过滤，这批应该获取剩余的记录
    assert!(result.items.len() <= 10);

    // 如果没有更多数据了
    if result.items.len() < 10 {
        assert!(!result.has_more());
    }
}

#[tokio::test]
async fn test_update() {
    let db = setup_test_db().await.unwrap();
    let repo = TestUserRepository::new(db);

    let user = TestUser {
        id: 1,
        username: "alice".to_string(),
        email: "alice@test.com".to_string(),
        active: true,
    };

    repo.insert(user).await.unwrap();

    // Update user
    let mut updated_user = repo.load(filter::Find::ById(1)).await.unwrap();
    updated_user.username = "alice_updated".to_string();
    updated_user.active = false;

    let result = repo.update(updated_user).await.unwrap();
    assert_eq!(result.username, "alice_updated");
    assert!(!result.active);

    // Verify update
    let found = repo.load(filter::Find::ById(1)).await.unwrap();
    assert_eq!(found.username, "alice_updated");
}

#[tokio::test]
async fn test_delete() {
    let db = setup_test_db().await.unwrap();
    let repo = TestUserRepository::new(db);

    let user = TestUser {
        id: 1,
        username: "alice".to_string(),
        email: "alice@test.com".to_string(),
        active: true,
    };

    repo.insert(user.clone()).await.unwrap();

    // Delete user - 返回被删除的实体
    let deleted_users = repo.delete(filter::Delete::ById(1)).await.unwrap();
    assert_eq!(deleted_users.len(), 1);
    assert_eq!(deleted_users[0].id, 1);
    assert_eq!(deleted_users[0].username, "alice");
    assert_eq!(deleted_users[0].email, "alice@test.com");

    // Verify deletion
    let found = repo.find(filter::Find::ById(1)).await.unwrap();
    assert!(found.is_none());

    // Test delete_count
    let user2 = TestUser {
        id: 2,
        username: "bob".to_string(),
        email: "bob@test.com".to_string(),
        active: true,
    };
    repo.insert(user2).await.unwrap();

    let count = repo.delete_count(filter::Delete::ById(2)).await.unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_batch_insert() {
    let db = setup_test_db().await.unwrap();
    let repo = TestUserRepository::new(db);

    let users = vec![
        TestUser {
            id: 1,
            username: "alice".to_string(),
            email: "alice@test.com".to_string(),
            active: true,
        },
        TestUser {
            id: 2,
            username: "bob".to_string(),
            email: "bob@test.com".to_string(),
            active: true,
        },
        TestUser {
            id: 3,
            username: "charlie".to_string(),
            email: "charlie@test.com".to_string(),
            active: true,
        },
    ];

    let created = repo.batch_insert(users).await.unwrap();
    assert_eq!(created.len(), 3);

    // Verify all inserted
    let result = repo.search(filter::Search::default()).await.unwrap();
    assert_eq!(result.items.len(), 3);
    assert_eq!(result.total_count, 3);
}

#[tokio::test]
async fn test_count() {
    let db = setup_test_db().await.unwrap();
    let repo = TestUserRepository::new(db);

    // Insert users
    for i in 1..=5 {
        let user = TestUser {
            id: i,
            username: format!("user{}", i),
            email: format!("user{}@test.com", i),
            active: i % 2 == 0,
        };
        repo.insert(user).await.unwrap();
    }

    // Count all
    let total = repo.count(filter::Search::default()).await.unwrap();
    assert_eq!(total, 5);

    // Count active
    let active = repo
        .count(filter::Search {
            active: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(active, 2);
}

#[tokio::test]
async fn test_exists() {
    let db = setup_test_db().await.unwrap();
    let repo = TestUserRepository::new(db);

    let user = TestUser {
        id: 1,
        username: "alice".to_string(),
        email: "alice@test.com".to_string(),
        active: true,
    };

    repo.insert(user).await.unwrap();

    // Check exists
    let exists = repo.exists(filter::Find::ById(1)).await.unwrap();
    assert!(exists);

    // Check not exists
    let not_exists = repo.exists(filter::Find::ById(999)).await.unwrap();
    assert!(!not_exists);
}

#[tokio::test]
async fn test_upsert() {
    let db = setup_test_db().await.unwrap();
    let repo = TestUserRepository::new(db);

    // Test 1: Upsert on empty table with new id - should insert
    let user = TestUser {
        id: 1,
        username: "alice".to_string(),
        email: "alice@test.com".to_string(),
        active: true,
    };
    repo.upsert(user).await.unwrap();

    // Verify inserted
    let found = repo.find(filter::Find::ById(1)).await.unwrap().unwrap();
    assert_eq!(found.username, "alice");

    // Test 2: Upsert with same id and do_nothing - should keep original
    let user = TestUser {
        id: 1,
        username: "alice_updated".to_string(),
        email: "alice_new@test.com".to_string(),
        active: false,
    };
    repo.upsert(user).await.unwrap();

    // Verify nothing changed (do_nothing on conflict due to TestUser's Upsertable impl)
    let found = repo.find(filter::Find::ById(1)).await.unwrap().unwrap();
    assert_eq!(found.username, "alice");
    assert_eq!(found.email, "alice@test.com");
    assert!(found.active);
}

#[tokio::test]
async fn test_batch_upsert() {
    let db = setup_test_db().await.unwrap();
    let repo = TestUserRepository::new(db);

    // Insert initial users
    let users = vec![
        TestUser {
            id: 1,
            username: "alice".to_string(),
            email: "alice@test.com".to_string(),
            active: true,
        },
        TestUser {
            id: 2,
            username: "bob".to_string(),
            email: "bob@test.com".to_string(),
            active: true,
        },
    ];
    repo.batch_insert(users).await.unwrap();

    // Upsert with do_nothing - should keep existing and insert new
    let users = vec![
        TestUser {
            id: 1,
            username: "alice_updated".to_string(),
            email: "alice_new@test.com".to_string(),
            active: false,
        },
        TestUser {
            id: 3,
            username: "charlie".to_string(),
            email: "charlie@test.com".to_string(),
            active: true,
        },
    ];
    repo.batch_upsert(users).await.unwrap();

    // Verify: alice not changed (do_nothing), charlie inserted
    let result = repo.search(filter::Search::default()).await.unwrap();
    assert_eq!(result.total_count, 3);

    let alice = repo.find(filter::Find::ById(1)).await.unwrap().unwrap();
    assert_eq!(alice.username, "alice"); // Not updated due to do_nothing

    let charlie = repo.find(filter::Find::ById(3)).await.unwrap().unwrap();
    assert_eq!(charlie.username, "charlie");
}
