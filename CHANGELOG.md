# Changelog

All notable changes to this project will be documented in this file.

## [1.0.0] - 2024

### Features

#### Repository 派生宏

通过 `#[derive(Repository)]` 自动生成 CRUD 方法：

```rust
#[derive(Repository)]
#[repository(entity = "entities::user", domain = "User")]
pub struct UserRepository {
    db: Arc<DatabaseConnection>,
}
```

#### 功能选择系统

灵活控制生成的方法：

- **默认**：只生成 `find` + `search` 相关方法
- **all = true**：生成所有方法
- **include = [...]**：只生成指定的功能
- **exclude = [...]**：排除指定的功能

```rust
// 只启用指定功能
#[repository(entity = "...", domain = "...", include = ["find", "insert"])]

// 排除某些功能
#[repository(entity = "...", domain = "...", exclude = ["delete"])]
```

可用的功能：

| Feature  | 生成的方法                          |
|----------|--------------------------------|
| `find`   | `find()`, `load()`, `exists()` |
| `search` | `search()`, `count()`          |
| `insert` | `insert()`, `batch_insert()`   |
| `update` | `update()`                     |
| `delete` | `delete()`, `delete_count()`   |
| `upsert` | `upsert()`, `batch_upsert()`   |

#### 分页支持

支持两种分页方式：

```rust
// PageOffset 分页（页码从 1 开始）
Pagination::PageOffset { page: 1, size: 20 }

// Cursor 分页
Pagination::Cursor { limit: 50 }
```

SearchResult 返回完整的分页信息：

```rust
pub struct SearchResult<T> {
    pub total_count: u64,  // 总数量
    pub total_page: u64,   // 总页数
    pub page: u64,         // 当前页码
    pub size: u64,         // 当前页大小
    pub items: Vec<T>,     // 当前页数据
}
```

#### 批量操作

批量操作自动分块处理，使用事务确保原子性：

- `batch_insert()`: 批量插入
- `batch_upsert()`: 批量 Upsert

自动计算分块大小，避免 PostgreSQL 参数数量限制。

#### Trait 系统

- `FindFilter`: 单条查询过滤器
- `SearchFilter`: 列表搜索过滤器（支持分页）
- `DeleteFilter`: 删除过滤器
- `Upsertable`: Upsert 冲突处理策略

