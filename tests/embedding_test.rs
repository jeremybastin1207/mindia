mod helpers;

use helpers::setup_test_app;
use mindia::db::EmbeddingRepository;
use mindia::models::EntityType;
use uuid::Uuid;
use sqlx::Row;

#[tokio::test]
async fn test_insert_embedding_with_tenant_id() {
    let app = setup_test_app().await;
    let pool = app.pool();
    
    let embedding_repo = EmbeddingRepository::new(pool.clone());
    
    // Create test tenant IDs
    let tenant1_id = Uuid::new_v4();
    let tenant2_id = Uuid::new_v4();
    let entity1_id = Uuid::new_v4();
    let entity2_id = Uuid::new_v4();
    
    // Create test embeddings for different tenants
    let embedding1 = embedding_repo
        .insert_embedding(
            tenant1_id,
            entity1_id,
            EntityType::Image,
            "A beautiful sunset over the ocean".to_string(),
            vec![0.1; 768], // Mock 768-dimensional embedding
            "test-model".to_string(),
        )
        .await
        .expect("Failed to insert embedding 1");
    
    let embedding2 = embedding_repo
        .insert_embedding(
            tenant2_id,
            entity2_id,
            EntityType::Image,
            "A cat sitting on a windowsill".to_string(),
            vec![0.2; 768], // Mock 768-dimensional embedding
            "test-model".to_string(),
        )
        .await
        .expect("Failed to insert embedding 2");
    
    // Verify embeddings have correct tenant IDs
    assert_eq!(embedding1.tenant_id, tenant1_id);
    assert_eq!(embedding2.tenant_id, tenant2_id);
    assert_ne!(embedding1.tenant_id, embedding2.tenant_id);
    
    // Verify embeddings are stored
    let retrieved1 = embedding_repo
        .get_embedding(tenant1_id, entity1_id)
        .await
        .expect("Failed to get embedding 1");
    
    assert!(retrieved1.is_some());
    let retrieved1 = retrieved1.unwrap();
    assert_eq!(retrieved1.tenant_id, tenant1_id);
    assert_eq!(retrieved1.entity_id, entity1_id);
}

#[tokio::test]
async fn test_embedding_tenant_isolation() {
    let app = setup_test_app().await;
    let pool = app.pool();
    
    let embedding_repo = EmbeddingRepository::new(pool.clone());
    
    // Create test tenant IDs
    let tenant1_id = Uuid::new_v4();
    let tenant2_id = Uuid::new_v4();
    let entity1_id = Uuid::new_v4();
    let entity2_id = Uuid::new_v4();
    
    // Create embeddings for both tenants with similar descriptions
    embedding_repo
        .insert_embedding(
            tenant1_id,
            entity1_id,
            EntityType::Image,
            "sunset beach ocean".to_string(),
            vec![0.1; 768],
            "test-model".to_string(),
        )
        .await
        .expect("Failed to insert tenant1 embedding");
    
    embedding_repo
        .insert_embedding(
            tenant2_id,
            entity2_id,
            EntityType::Image,
            "sunset beach ocean".to_string(), // Same description
            vec![0.1; 768], // Same embedding (in real scenario would be different)
            "test-model".to_string(),
        )
        .await
        .expect("Failed to insert tenant2 embedding");
    
    // CRITICAL: Search as tenant1 should only return tenant1's results
    let query_embedding = vec![0.1; 768]; // Same as inserted embeddings
    
    let results_tenant1 = embedding_repo
        .search_similar(tenant1_id, query_embedding.clone(), None, 10)
        .await
        .expect("Failed to search as tenant1");
    
    // Verify tenant1 only sees their own results
    assert_eq!(results_tenant1.len(), 1, "Tenant1 should only see their own embedding");
    assert_eq!(results_tenant1[0].id, entity1_id, "Tenant1 should see entity1");
    
    // CRITICAL: Search as tenant2 should only return tenant2's results
    let results_tenant2 = embedding_repo
        .search_similar(tenant2_id, query_embedding, None, 10)
        .await
        .expect("Failed to search as tenant2");
    
    // Verify tenant2 only sees their own results
    assert_eq!(results_tenant2.len(), 1, "Tenant2 should only see their own embedding");
    assert_eq!(results_tenant2[0].id, entity2_id, "Tenant2 should see entity2");
    
    // CRITICAL: Verify cross-tenant isolation - tenant1 should NOT see tenant2's entity
    assert!(
        !results_tenant1.iter().any(|r| r.id == entity2_id),
        "Tenant1 should NOT see tenant2's entities"
    );
    
    assert!(
        !results_tenant2.iter().any(|r| r.id == entity1_id),
        "Tenant2 should NOT see tenant1's entities"
    );
}

#[tokio::test]
async fn test_embedding_search_with_entity_type_filter() {
    let app = setup_test_app().await;
    let pool = app.pool();
    
    let embedding_repo = EmbeddingRepository::new(pool.clone());
    
    let tenant_id = Uuid::new_v4();
    let image_id = Uuid::new_v4();
    let video_id = Uuid::new_v4();
    let document_id = Uuid::new_v4();
    
    // Insert embeddings for different entity types
    embedding_repo
        .insert_embedding(
            tenant_id,
            image_id,
            EntityType::Image,
            "test image".to_string(),
            vec![0.1; 768],
            "test-model".to_string(),
        )
        .await
        .expect("Failed to insert image embedding");
    
    embedding_repo
        .insert_embedding(
            tenant_id,
            video_id,
            EntityType::Video,
            "test video".to_string(),
            vec![0.2; 768],
            "test-model".to_string(),
        )
        .await
        .expect("Failed to insert video embedding");
    
    embedding_repo
        .insert_embedding(
            tenant_id,
            document_id,
            EntityType::Document,
            "test document".to_string(),
            vec![0.3; 768],
            "test-model".to_string(),
        )
        .await
        .expect("Failed to insert document embedding");
    
    // Search with entity type filter - should only return images
    let query_embedding = vec![0.1; 768];
    let results = embedding_repo
        .search_similar(tenant_id, query_embedding, Some(EntityType::Image), 10)
        .await
        .expect("Failed to search with entity type filter");
    
    // Verify only images are returned
    assert_eq!(results.len(), 1, "Should return only one image result");
    assert_eq!(results[0].id, image_id, "Should return the image entity");
    assert_eq!(results[0].entity_type, EntityType::Image, "Entity type should be Image");
}

#[tokio::test]
async fn test_get_entities_without_embeddings_includes_tenant_id() {
    let app = setup_test_app().await;
    let pool = app.pool();
    
    let embedding_repo = EmbeddingRepository::new(pool.clone());
    
    // First, create an image entity directly in the database
    // We'll need to use sqlx directly since we don't have a full test setup for images
    let tenant1_id = Uuid::new_v4();
    let tenant2_id = Uuid::new_v4();
    let image1_id = Uuid::new_v4();
    let image2_id = Uuid::new_v4();
    
    // Insert images directly via SQL
    sqlx::query(
        "INSERT INTO images (tenant_id, id, filename, original_filename, s3_key, s3_bucket, s3_url, content_type, file_size, uploaded_at, updated_at, store_behavior, store_permanently) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW(), $10, $11)"
    )
    .bind(tenant1_id)
    .bind(image1_id)
    .bind("test1.jpg")
    .bind("test1.jpg")
    .bind("images/test1.jpg")
    .bind("test-bucket")
    .bind("http://test.com/test1.jpg")
    .bind("image/jpeg")
    .bind(1000i64)
    .bind("keep")
    .bind(true)
    .execute(pool)
    .await
    .expect("Failed to insert test image 1");
    
    sqlx::query(
        "INSERT INTO images (tenant_id, id, filename, original_filename, s3_key, s3_bucket, s3_url, content_type, file_size, uploaded_at, updated_at, store_behavior, store_permanently) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW(), $10, $11)"
    )
    .bind(tenant2_id)
    .bind(image2_id)
    .bind("test2.jpg")
    .bind("test2.jpg")
    .bind("images/test2.jpg")
    .bind("test-bucket")
    .bind("http://test.com/test2.jpg")
    .bind("image/jpeg")
    .bind(1000i64)
    .bind("keep")
    .bind(true)
    .execute(pool)
    .await
    .expect("Failed to insert test image 2");
    
    // Create embedding for image1 only
    embedding_repo
        .insert_embedding(
            tenant1_id,
            image1_id,
            EntityType::Image,
            "test description".to_string(),
            vec![0.1; 768],
            "test-model".to_string(),
        )
        .await
        .expect("Failed to insert embedding");
    
    // Get entities without embeddings - test with None (all tenants) for system-level testing
    // In production, always use Some(tenant_id) for tenant isolation
    let entities = embedding_repo
        .get_entities_without_embeddings(None, EntityType::Image, 100)
        .await
        .expect("Failed to get entities without embeddings");
    
    // Verify tenant2's image is in the list (no embedding) but tenant1's is not (has embedding)
    assert!(
        entities.iter().any(|(id, tenant_id, _)| *id == image2_id && *tenant_id == tenant2_id),
        "Should return tenant2's image which has no embedding"
    );
    
    assert!(
        !entities.iter().any(|(id, _, _)| *id == image1_id),
        "Should NOT return tenant1's image which already has an embedding"
    );
}
