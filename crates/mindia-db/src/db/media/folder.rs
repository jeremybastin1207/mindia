use mindia_core::{
    models::{Folder, FolderTreeNode},
    AppError,
};
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

/// Repository for managing folders
#[derive(Clone)]
pub struct FolderRepository {
    pool: PgPool,
}

impl FolderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new folder
    #[tracing::instrument(skip(self), fields(db.table = "folders", db.operation = "insert"))]
    pub async fn create_folder(
        &self,
        tenant_id: Uuid,
        name: String,
        parent_id: Option<Uuid>,
    ) -> Result<Folder, AppError> {
        // Validate parent exists and belongs to tenant if provided
        if let Some(pid) = parent_id {
            let parent_exists = sqlx::query_scalar::<Postgres, bool>(
                "SELECT EXISTS(SELECT 1 FROM folders WHERE id = $1 AND tenant_id = $2)",
            )
            .bind(pid)
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;

            if !parent_exists {
                return Err(anyhow::anyhow!("Parent folder not found").into());
            }
        }

        // Check for duplicate name in same parent
        let duplicate_exists = sqlx::query_scalar::<Postgres, bool>(
            "SELECT EXISTS(SELECT 1 FROM folders WHERE tenant_id = $1 AND parent_id IS NOT DISTINCT FROM $2 AND name = $3)"
        )
        .bind(tenant_id)
        .bind(parent_id)
        .bind(&name)
        .fetch_one(&self.pool)
        .await?;

        if duplicate_exists {
            return Err(anyhow::anyhow!("Duplicate folder name in same parent").into());
        }

        // Insert folder
        let folder = sqlx::query_as::<Postgres, Folder>(
            r#"
            INSERT INTO folders (tenant_id, name, parent_id)
            VALUES ($1, $2, $3)
            RETURNING id, tenant_id, name, parent_id, created_at, updated_at
            "#,
        )
        .bind(tenant_id)
        .bind(&name)
        .bind(parent_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(folder)
    }

    /// Get folder by ID (tenant-scoped)
    #[tracing::instrument(skip(self), fields(db.table = "folders", db.operation = "select", db.record_id = %id))]
    pub async fn get_folder(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<Folder>, AppError> {
        let folder = sqlx::query_as::<Postgres, Folder>(
            "SELECT id, tenant_id, name, parent_id, created_at, updated_at FROM folders WHERE tenant_id = $1 AND id = $2"
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(folder)
    }

    /// List folders, optionally filtered by parent
    #[tracing::instrument(skip(self), fields(db.table = "folders", db.operation = "select"))]
    pub async fn list_folders(
        &self,
        tenant_id: Uuid,
        parent_id: Option<Option<Uuid>>, // Option<Option> to distinguish None from Some(None)
    ) -> Result<Vec<Folder>, AppError> {
        let folders = match parent_id {
            None => {
                // Return all folders for tenant
                sqlx::query_as::<Postgres, Folder>(
                    "SELECT id, tenant_id, name, parent_id, created_at, updated_at FROM folders WHERE tenant_id = $1 ORDER BY name ASC"
                )
                .bind(tenant_id)
                .fetch_all(&self.pool)
                .await?
            }
            Some(None) => {
                // Return only root folders (parent_id IS NULL)
                sqlx::query_as::<Postgres, Folder>(
                    "SELECT id, tenant_id, name, parent_id, created_at, updated_at FROM folders WHERE tenant_id = $1 AND parent_id IS NULL ORDER BY name ASC"
                )
                .bind(tenant_id)
                .fetch_all(&self.pool)
                .await?
            }
            Some(Some(pid)) => {
                // Return folders with specific parent
                sqlx::query_as::<Postgres, Folder>(
                    "SELECT id, tenant_id, name, parent_id, created_at, updated_at FROM folders WHERE tenant_id = $1 AND parent_id = $2 ORDER BY name ASC"
                )
                .bind(tenant_id)
                .bind(pid)
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(folders)
    }

    /// Update folder (name and/or parent)
    #[tracing::instrument(skip(self), fields(db.table = "folders", db.operation = "update", db.record_id = %id))]
    pub async fn update_folder(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        name: Option<String>,
        parent_id: Option<Option<Uuid>>,
    ) -> Result<Folder, AppError> {
        // Get current folder
        let current_folder = self
            .get_folder(tenant_id, id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        // Validate new parent if provided
        if let Some(Some(pid)) = parent_id {
            // Check parent exists and belongs to tenant
            let parent_exists = sqlx::query_scalar::<Postgres, bool>(
                "SELECT EXISTS(SELECT 1 FROM folders WHERE id = $1 AND tenant_id = $2)",
            )
            .bind(pid)
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;

            if !parent_exists {
                return Err(anyhow::anyhow!("Parent folder not found").into());
            }

            // Prevent cycle: check if new parent is a descendant of this folder
            if self.is_descendant(tenant_id, pid, id).await? {
                return Err(anyhow::anyhow!("Cannot move folder: would create a cycle").into());
            }

            // Check for duplicate name in new parent if name is also being updated
            let check_name = name.as_ref().unwrap_or(&current_folder.name);
            let duplicate_exists = sqlx::query_scalar::<Postgres, bool>(
                "SELECT EXISTS(SELECT 1 FROM folders WHERE tenant_id = $1 AND parent_id = $2 AND name = $3 AND id != $4)"
            )
            .bind(tenant_id)
            .bind(pid)
            .bind(check_name)
            .bind(id)
            .fetch_one(&self.pool)
            .await?;

            if duplicate_exists {
                return Err(anyhow::anyhow!("Duplicate folder name in same parent").into());
            }
        } else if let Some(None) = parent_id {
            // Moving to root - check for duplicate name at root
            let check_name = name.as_ref().unwrap_or(&current_folder.name);
            let duplicate_exists = sqlx::query_scalar::<Postgres, bool>(
                "SELECT EXISTS(SELECT 1 FROM folders WHERE tenant_id = $1 AND parent_id IS NULL AND name = $2 AND id != $3)"
            )
            .bind(tenant_id)
            .bind(check_name)
            .bind(id)
            .fetch_one(&self.pool)
            .await?;

            if duplicate_exists {
                return Err(anyhow::anyhow!("Duplicate folder name at root").into());
            }
        } else if let Some(ref new_name) = name {
            // Only name is being updated - check for duplicate in current parent
            let duplicate_exists = sqlx::query_scalar::<Postgres, bool>(
                "SELECT EXISTS(SELECT 1 FROM folders WHERE tenant_id = $1 AND parent_id IS NOT DISTINCT FROM $2 AND name = $3 AND id != $4)"
            )
            .bind(tenant_id)
            .bind(current_folder.parent_id)
            .bind(new_name)
            .bind(id)
            .fetch_one(&self.pool)
            .await?;

            if duplicate_exists {
                return Err(anyhow::anyhow!("Duplicate folder name in same parent").into());
            }
        }

        // Build update query
        let mut query = String::from("UPDATE folders SET updated_at = NOW()");
        let mut bind_index = 1;

        if let Some(ref _new_name) = name {
            query.push_str(&format!(", name = ${}", bind_index));
            bind_index += 1;
        }

        if let Some(_new_parent) = parent_id {
            query.push_str(&format!(", parent_id = ${}", bind_index));
            bind_index += 1;
        }

        query.push_str(&format!(" WHERE tenant_id = ${} AND id = ${} RETURNING id, tenant_id, name, parent_id, created_at, updated_at", bind_index, bind_index + 1));

        let mut query_builder = sqlx::query_as::<Postgres, Folder>(&query);
        if let Some(ref new_name) = name {
            query_builder = query_builder.bind(new_name);
        }
        if let Some(new_parent) = parent_id {
            query_builder = query_builder.bind(new_parent);
        }
        query_builder = query_builder.bind(tenant_id).bind(id);

        let folder = query_builder.fetch_one(&self.pool).await?;

        Ok(folder)
    }

    /// Delete folder (must be empty)
    #[tracing::instrument(skip(self), fields(db.table = "folders", db.operation = "delete", db.record_id = %id))]
    pub async fn delete_folder(&self, tenant_id: Uuid, id: Uuid) -> Result<bool, AppError> {
        // Check if folder has media
        let media_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM media WHERE tenant_id = $1 AND folder_id = $2",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        if media_count > 0 {
            return Err(anyhow::anyhow!("Cannot delete folder: contains media files").into());
        }

        // Check if folder has subfolders
        let subfolder_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM folders WHERE tenant_id = $1 AND parent_id = $2",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        if subfolder_count > 0 {
            return Err(anyhow::anyhow!("Cannot delete folder: contains subfolders").into());
        }

        // Delete folder
        let rows_affected = sqlx::query("DELETE FROM folders WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected();

        Ok(rows_affected > 0)
    }

    /// Get hierarchical folder tree
    #[tracing::instrument(skip(self), fields(db.table = "folders", db.operation = "select"))]
    pub async fn get_folder_tree(&self, tenant_id: Uuid) -> Result<Vec<FolderTreeNode>, AppError> {
        // Get all folders for tenant
        let all_folders = self.list_folders(tenant_id, None).await?;

        // Get media counts for each folder
        let mut folder_nodes: Vec<FolderTreeNode> = all_folders
            .into_iter()
            .map(|folder| FolderTreeNode {
                id: folder.id,
                name: folder.name,
                parent_id: folder.parent_id,
                created_at: folder.created_at,
                updated_at: folder.updated_at,
                media_count: None,
                children: Vec::new(),
            })
            .collect();

        // Fetch media counts
        for node in &mut folder_nodes {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM media WHERE tenant_id = $1 AND folder_id = $2",
            )
            .bind(tenant_id)
            .bind(node.id)
            .fetch_one(&self.pool)
            .await?;
            node.media_count = Some(count);
        }

        // Build tree structure
        let mut root_nodes = Vec::new();
        let mut node_map: std::collections::HashMap<Uuid, usize> = std::collections::HashMap::new();

        // Create index map
        for (idx, node) in folder_nodes.iter().enumerate() {
            node_map.insert(node.id, idx);
        }

        // Build parent-child relationships
        let mut children_to_add: Vec<(usize, FolderTreeNode)> = Vec::new();
        for (idx, node) in folder_nodes.iter().enumerate() {
            if let Some(parent_id) = node.parent_id {
                if let Some(&parent_idx) = node_map.get(&parent_id) {
                    let node_clone = folder_nodes[idx].clone();
                    children_to_add.push((parent_idx, node_clone));
                }
            } else {
                // Root node
                root_nodes.push(folder_nodes[idx].clone());
            }
        }

        // Add children after iteration
        for (parent_idx, node_clone) in children_to_add {
            folder_nodes[parent_idx].children.push(node_clone);
        }

        // Sort children recursively
        fn sort_children(nodes: &mut [FolderTreeNode]) {
            nodes.sort_by(|a, b| a.name.cmp(&b.name));
            for node in nodes.iter_mut() {
                sort_children(&mut node.children);
            }
        }

        sort_children(&mut root_nodes);

        Ok(root_nodes)
    }

    /// Count media items in folder
    #[tracing::instrument(skip(self), fields(db.table = "folders", db.operation = "select"))]
    pub async fn count_media_in_folder(
        &self,
        tenant_id: Uuid,
        folder_id: Uuid,
    ) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM media WHERE tenant_id = $1 AND folder_id = $2",
        )
        .bind(tenant_id)
        .bind(folder_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// Count subfolders in folder
    pub async fn count_subfolders(
        &self,
        tenant_id: Uuid,
        folder_id: Uuid,
    ) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM folders WHERE tenant_id = $1 AND parent_id = $2",
        )
        .bind(tenant_id)
        .bind(folder_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// Check if a folder is a descendant of another folder (for cycle prevention)
    async fn is_descendant(
        &self,
        tenant_id: Uuid,
        ancestor_id: Uuid,
        descendant_id: Uuid,
    ) -> Result<bool, AppError> {
        if ancestor_id == descendant_id {
            return Ok(true);
        }

        // Use recursive CTE to check if descendant_id is in the subtree of ancestor_id
        let is_desc: bool = sqlx::query_scalar(
            r#"
            WITH RECURSIVE folder_tree AS (
                SELECT id, parent_id
                FROM folders
                WHERE id = $1 AND tenant_id = $3
                UNION ALL
                SELECT f.id, f.parent_id
                FROM folders f
                INNER JOIN folder_tree ft ON f.parent_id = ft.id
                WHERE f.tenant_id = $3
            )
            SELECT EXISTS(SELECT 1 FROM folder_tree WHERE id = $2)
            "#,
        )
        .bind(ancestor_id)
        .bind(descendant_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(is_desc)
    }
}
