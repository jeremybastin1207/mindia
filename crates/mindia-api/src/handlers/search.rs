use std::sync::Arc;

use axum::{
    extract::{Query, Request, State},
    response::IntoResponse,
    Json,
};
use mindia_core::models::{EntityType, SearchQuery, SearchResult};
use mindia_core::AppError;
use mindia_db::media::metadata_search::MetadataFilters;
use mindia_services::SemanticSearchProvider;
use percent_encoding::percent_decode_str;
use serde::Serialize;
use utoipa::ToSchema;

use crate::auth::models::TenantContext;
use crate::error::{ErrorResponse, HttpAppError};
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResponse {
    pub query: Option<String>,
    pub results: Vec<SearchResult>,
    pub count: usize,
}

/// Search strategy enum to encapsulate search mode logic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchStrategy {
    Metadata,
    Semantic,
    Both,
}

impl SearchStrategy {
    fn from_str(s: &str) -> Result<Self, AppError> {
        match s.to_lowercase().as_str() {
            "metadata" => Ok(SearchStrategy::Metadata),
            "semantic" => Ok(SearchStrategy::Semantic),
            "both" => Ok(SearchStrategy::Both),
            _ => Err(AppError::InvalidInput(format!(
                "Invalid search mode: {}. Must be 'metadata', 'semantic', or 'both'",
                s
            ))),
        }
    }
}

/// Parse entity type from string
fn parse_entity_type(type_str: &str) -> Result<EntityType, AppError> {
    match type_str.to_lowercase().as_str() {
        "image" => Ok(EntityType::Image),
        "video" => Ok(EntityType::Video),
        "document" => Ok(EntityType::Document),
        "audio" => Ok(EntityType::Audio),
        _ => Err(AppError::InvalidInput(format!(
            "Invalid entity type: {}. Must be 'image', 'video', 'document', or 'audio'",
            type_str
        ))),
    }
}

/// Helper function to properly decode URL-encoded string
fn url_decode(s: &str) -> String {
    percent_decode_str(s).decode_utf8_lossy().replace('+', " ")
}

/// Helper function to parse metadata filters from query string
///
/// Parses query parameters like ?metadata.userId=123&metadata.type=avatar
/// into MetadataFilters structure
fn parse_metadata_filters_from_query(query_str: Option<&str>) -> Result<MetadataFilters, AppError> {
    let mut filters = MetadataFilters::new();

    if let Some(query) = query_str {
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                let key = url_decode(key);
                let value = url_decode(value);

                if key.starts_with("metadata.") {
                    let metadata_key = key.trim_start_matches("metadata.");
                    if metadata_key.is_empty() {
                        return Err(AppError::InvalidInput(
                            "Metadata key cannot be empty".to_string(),
                        ));
                    }
                    filters.exact.push((metadata_key.to_string(), value));
                } else if key.starts_with("metadata_min.") {
                    let metadata_key = key.trim_start_matches("metadata_min.");
                    if metadata_key.is_empty() {
                        return Err(AppError::InvalidInput(
                            "Metadata key cannot be empty".to_string(),
                        ));
                    }
                    // Find or create range entry
                    if let Some(existing) = filters.ranges.iter_mut().find(|r| r.0 == metadata_key)
                    {
                        existing.1 = Some(value);
                    } else {
                        filters
                            .ranges
                            .push((metadata_key.to_string(), Some(value), None));
                    }
                } else if key.starts_with("metadata_max.") {
                    let metadata_key = key.trim_start_matches("metadata_max.");
                    if metadata_key.is_empty() {
                        return Err(AppError::InvalidInput(
                            "Metadata key cannot be empty".to_string(),
                        ));
                    }
                    // Find or create range entry
                    if let Some(existing) = filters.ranges.iter_mut().find(|r| r.0 == metadata_key)
                    {
                        existing.2 = Some(value);
                    } else {
                        filters
                            .ranges
                            .push((metadata_key.to_string(), None, Some(value)));
                    }
                } else if key.starts_with("metadata_contains.") {
                    let metadata_key = key.trim_start_matches("metadata_contains.");
                    if metadata_key.is_empty() {
                        return Err(AppError::InvalidInput(
                            "Metadata key cannot be empty".to_string(),
                        ));
                    }
                    filters
                        .text_contains
                        .push((metadata_key.to_string(), value));
                }
            }
        }
    }

    Ok(filters)
}

/// Validate and normalize search parameters
fn validate_search_params(params: &SearchQuery) -> Result<(i64, i64), AppError> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);
    Ok((limit, offset))
}

/// Execute metadata-only search
async fn execute_metadata_search(
    state: &Arc<AppState>,
    tenant_id: uuid::Uuid,
    filters: &MetadataFilters,
    entity_type: Option<EntityType>,
    folder_id: Option<uuid::Uuid>,
    limit: i64,
    offset: i64,
) -> Result<Vec<SearchResult>, AppError> {
    state.db
        .metadata_search_repository
        .search_by_metadata(tenant_id, filters, entity_type, folder_id, limit, offset)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, tenant_id = %tenant_id, "Failed to search by metadata");
            convert_search_error(e)
        })
}

/// Execute semantic-only search
async fn execute_semantic_search(
    state: &Arc<AppState>,
    tenant_id: uuid::Uuid,
    query: &str,
    entity_type: Option<EntityType>,
    folder_id: Option<uuid::Uuid>,
    limit: i64,
    offset: i64,
) -> Result<Vec<SearchResult>, AppError> {
    let semantic_search = state.semantic_search.as_ref().ok_or_else(|| {
        tracing::warn!("Semantic search requested but feature is not enabled");
        AppError::InvalidInput("Semantic search is not enabled".to_string())
    })?;

    let query_embedding = semantic_search
        .generate_embedding(query)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to generate query embedding");
            AppError::Internal(format!("Failed to generate query embedding: {}", e))
        })?;

    state
        .embedding_repository
        .search_similar(
            tenant_id,
            query_embedding,
            entity_type,
            folder_id,
            limit,
            offset,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, tenant_id = %tenant_id, "Failed to search embeddings");
            AppError::Internal(format!("Failed to search embeddings: {}", e))
        })
}

/// Execute combined semantic + metadata search
#[allow(clippy::too_many_arguments)]
async fn execute_combined_search(
    state: &Arc<AppState>,
    tenant_id: uuid::Uuid,
    query_embedding: Vec<f32>,
    filters: &MetadataFilters,
    entity_type: Option<EntityType>,
    folder_id: Option<uuid::Uuid>,
    limit: i64,
    offset: i64,
) -> Result<Vec<SearchResult>, AppError> {
    state.db
        .metadata_search_repository
        .search_with_metadata_filters(
            tenant_id,
            Some(query_embedding),
            &Some(filters.clone()),
            entity_type,
            folder_id,
            limit,
            offset,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, tenant_id = %tenant_id, "Failed to search with metadata filters");
            convert_search_error(e)
        })
}

/// Convert anyhow::Error to AppError with proper error type detection
fn convert_search_error(e: anyhow::Error) -> AppError {
    let error_msg = e.to_string();
    if error_msg.contains("Too many metadata filters") {
        AppError::MetadataFilterLimitExceeded(error_msg)
    } else if error_msg.contains("not yet implemented") {
        AppError::InvalidMetadataFilter(error_msg)
    } else {
        AppError::Internal(format!("Search error: {}", error_msg))
    }
}

/// Generate embedding from query text
async fn generate_query_embedding(
    semantic_search: &(dyn SemanticSearchProvider + Send + Sync),
    query: &str,
) -> Result<Vec<f32>, AppError> {
    semantic_search
        .generate_embedding(query)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to generate query embedding");
            AppError::Internal(format!("Failed to generate query embedding: {}", e))
        })
}

/// Determine search strategy based on mode and available parameters
fn determine_search_strategy(
    strategy: SearchStrategy,
    has_query: bool,
    has_metadata_filters: bool,
) -> Result<(bool, bool, bool), AppError> {
    match strategy {
        SearchStrategy::Metadata => {
            if !has_metadata_filters {
                return Err(AppError::InvalidInput(
                    "Metadata search mode requires metadata filters".to_string(),
                ));
            }
            Ok((true, false, false)) // metadata only
        }
        SearchStrategy::Semantic => {
            if !has_query && !has_metadata_filters {
                return Err(AppError::InvalidInput(
                    "Semantic search mode requires either query parameter 'q' or metadata filters"
                        .to_string(),
                ));
            }
            Ok((has_metadata_filters, true, false)) // semantic, possibly with metadata
        }
        SearchStrategy::Both => {
            if !has_query && !has_metadata_filters {
                return Err(AppError::InvalidInput(
                    "Either query parameter 'q' or metadata filters are required".to_string(),
                ));
            }
            Ok((has_metadata_filters, has_query, true)) // both if available
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v0/search",
    tag = "search",
    params(
        SearchQuery
    ),
    responses(
        (status = 200, description = "Search results", body = SearchResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(skip(state), fields(
    search.query = tracing::field::Empty,
    search.entity_type = tracing::field::Empty,
    search.search_mode = tracing::field::Empty,
    tenant_id = tracing::field::Empty
))]
pub async fn search_files(
    tenant_ctx: TenantContext,
    Query(params): Query<SearchQuery>,
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Result<impl IntoResponse, HttpAppError> {
    tracing::Span::current().record("tenant_id", tenant_ctx.tenant_id.to_string());

    // Validate search parameters early
    params.validate().map_err(AppError::InvalidInput)?;

    // Parse and normalize search parameters
    let (limit, offset) = validate_search_params(&params)?;

    // Parse entity type filter if provided
    let entity_type_filter = params
        .entity_type
        .as_ref()
        .map(|s| parse_entity_type(s))
        .transpose()?;

    // Parse metadata filters from query string
    let metadata_filters = {
        let query_str = request.uri().query();
        let filters = parse_metadata_filters_from_query(query_str)?;
        if filters.is_empty() {
            None
        } else {
            Some(filters)
        }
    };

    // Determine search mode/strategy
    let search_mode_str = params.search_mode.as_deref().unwrap_or("both");
    let search_strategy = SearchStrategy::from_str(search_mode_str)?;
    tracing::Span::current().record("search_mode", search_mode_str);

    let has_query = params.q.as_ref().map(|q| !q.is_empty()).unwrap_or(false);
    let has_metadata_filters = metadata_filters
        .as_ref()
        .map(|f| !f.is_empty())
        .unwrap_or(false);

    // Record query in span if present
    if let Some(ref q) = params.q {
        if !q.is_empty() {
            tracing::Span::current().record("search.query", q.as_str());
        }
    }

    // Determine what type of search to execute
    let (use_metadata, use_semantic, allow_combined) =
        determine_search_strategy(search_strategy, has_query, has_metadata_filters)?;

    // Execute the appropriate search strategy
    let results = if allow_combined && has_query && has_metadata_filters {
        // Combined search: semantic + metadata
        let semantic_search = state
            .semantic_search
            .as_ref()
            .ok_or_else(|| AppError::InvalidInput("Semantic search is not enabled".to_string()))?;

        let query_str = params.q.as_ref().ok_or_else(|| {
            AppError::Internal("Query parameter should be present but was None".to_string())
        })?;
        let query_embedding = generate_query_embedding(semantic_search.as_ref(), query_str).await?;
        let filters = metadata_filters.ok_or_else(|| {
            AppError::Internal("Metadata filters should be present but were None".to_string())
        })?;
        execute_combined_search(
            &state,
            tenant_ctx.tenant_id,
            query_embedding,
            &filters,
            entity_type_filter,
            params.folder_id,
            limit,
            offset,
        )
        .await?
    } else if use_semantic && has_query {
        // Pure semantic search
        if let Some(ref metadata_filters) = metadata_filters {
            if !metadata_filters.is_empty() {
                // Semantic search with metadata filters
                let semantic_search = state.semantic_search.as_ref().ok_or_else(|| {
                    AppError::InvalidInput("Semantic search is not enabled".to_string())
                })?;
                let query_str = params.q.as_ref().ok_or_else(|| {
                    AppError::Internal("Query parameter should be present but was None".to_string())
                })?;
                let query_embedding =
                    generate_query_embedding(semantic_search.as_ref(), query_str).await?;
                execute_combined_search(
                    &state,
                    tenant_ctx.tenant_id,
                    query_embedding,
                    metadata_filters,
                    entity_type_filter,
                    params.folder_id,
                    limit,
                    offset,
                )
                .await?
            } else {
                // Pure semantic without metadata
                let query_str = params.q.as_ref().ok_or_else(|| {
                    AppError::Internal("Query parameter should be present but was None".to_string())
                })?;
                execute_semantic_search(
                    &state,
                    tenant_ctx.tenant_id,
                    query_str,
                    entity_type_filter,
                    params.folder_id,
                    limit,
                    offset,
                )
                .await?
            }
        } else {
            // Pure semantic without metadata
            let query_str = params.q.as_ref().ok_or_else(|| {
                AppError::Internal("Query parameter should be present but was None".to_string())
            })?;
            execute_semantic_search(
                &state,
                tenant_ctx.tenant_id,
                query_str,
                entity_type_filter,
                params.folder_id,
                limit,
                offset,
            )
            .await?
        }
    } else if use_metadata && has_metadata_filters {
        // Pure metadata search
        let filters = metadata_filters.ok_or_else(|| {
            AppError::Internal("Metadata filters should be present but were None".to_string())
        })?;
        if filters.is_empty() {
            return Err(HttpAppError::from(AppError::InvalidInput(
                "Metadata search requires at least one metadata filter".to_string(),
            )));
        }
        execute_metadata_search(
            &state,
            tenant_ctx.tenant_id,
            &filters,
            entity_type_filter,
            params.folder_id,
            limit,
            offset,
        )
        .await?
    } else {
        // This should not happen due to validation, but handle gracefully
        return Err(HttpAppError::from(AppError::InvalidInput(
            "Invalid search configuration: neither query nor metadata filters provided".to_string(),
        )));
    };

    // Apply similarity threshold filtering if min_similarity is specified
    let min_similarity = params.min_similarity.unwrap_or(0.3);
    let results: Vec<SearchResult> = results
        .into_iter()
        .filter(|r| r.similarity_score >= min_similarity)
        .collect();

    let count = results.len();

    tracing::info!(
        search_mode = search_mode_str,
        results_count = count,
        limit = limit,
        offset = offset,
        "Search completed successfully"
    );

    let response = SearchResponse {
        query: params.q.clone(),
        results,
        count,
    };

    Ok(Json(response))
}
