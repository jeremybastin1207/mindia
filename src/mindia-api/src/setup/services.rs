//! Service initialization and application state setup

#[cfg(feature = "video")]
use crate::job_queue::VideoJobQueue;
#[cfg(all(feature = "plugin", feature = "plugin-openai-image-description"))]
use crate::plugins::impls::OpenAiImageDescriptionPlugin;
#[cfg(feature = "plugin")]
use crate::plugins::{PluginRegistry, PluginService};
use anyhow::Context;
use mindia_core::Config;
use mindia_db::{
    create_analytics_repository, ApiKeyRepository, EmbeddingRepository, FileGroupRepository,
    FolderRepository, MediaRepository, MetadataSearchRepository, NamedTransformationRepository,
    StorageMetricsRepository, TaskRepository, TenantRepository, WebhookEventRepository,
    WebhookRepository, WebhookRetryRepository,
};
#[cfg(feature = "plugin")]
use mindia_db::{PluginConfigRepository, PluginExecutionRepository};
use mindia_infra::{
    start_storage_metrics_refresh, AnalyticsService, CapacityChecker, CleanupService, RateLimiter,
    WebhookRetryService, WebhookRetryServiceConfig, WebhookService, WebhookServiceConfig,
};
#[cfg(all(feature = "plugin", feature = "plugin-assembly-ai"))]
use mindia_plugins::AssemblyAiPlugin;
#[cfg(all(feature = "plugin", feature = "content-moderation"))]
use mindia_plugins::AwsRekognitionPlugin;
#[cfg(all(feature = "plugin", feature = "plugin-aws-transcribe"))]
use mindia_plugins::AwsTranscribePlugin;
#[cfg(all(feature = "plugin", feature = "plugin-claude-vision"))]
use mindia_plugins::ClaudeVisionPlugin;
#[cfg(all(feature = "plugin", feature = "plugin-google-vision"))]
use mindia_plugins::GoogleVisionPlugin;
#[cfg(all(feature = "plugin", feature = "plugin-replicate-deoldify"))]
use mindia_plugins::ReplicateDeoldifyPlugin;
#[cfg(feature = "clamav")]
use mindia_services::ClamAVService;
#[cfg(feature = "semantic-search")]
use mindia_services::{AnthropicService, SemanticSearchProvider};
use mindia_services::{S3Service, Storage};
// TaskQueue and task handlers moved to api crate
use crate::state::{
    AppState, AudioConfig, DatabaseConfig, DocumentConfig, ImageConfig, MediaConfig, S3Config,
    SecurityConfig, VideoConfig,
};
#[cfg(feature = "content-moderation")]
use crate::task_handlers::ContentModerationTaskHandler;
#[cfg(feature = "plugin")]
use crate::task_handlers::PluginTaskHandler;
use anyhow::Result;
use mindia_worker::{TaskQueue, TaskQueueConfig};
use sqlx::PgPool;
use std::sync::Arc;

/// Initialize all services and repositories, returning the application state
pub async fn initialize_services(
    config: &Config,
    pool: PgPool,
    s3_service: Option<S3Service>,
    storage: Arc<dyn Storage>,
) -> Result<Arc<AppState>> {
    // Initialize ClamAV service if enabled
    #[cfg(feature = "clamav")]
    let clamav_service = if config.clamav_enabled() {
        tracing::info!(
            host = %config.clamav_host(),
            port = config.clamav_port(),
            fail_closed = config.clamav_fail_closed(),
            "ClamAV scanning enabled (fail-closed: {})",
            if config.clamav_fail_closed() { "enabled" } else { "disabled (fail-open)" }
        );
        Some(ClamAVService::new(
            config.clamav_host().to_string(),
            config.clamav_port(),
            config.clamav_fail_closed(),
        ))
    } else {
        tracing::info!("ClamAV scanning disabled");
        None
    };
    #[cfg(not(feature = "clamav"))]
    let clamav_service = None;

    // Initialize encryption service (REQUIRED for production security)
    let encryption_service = mindia_core::EncryptionService::new()
        .context("ENCRYPTION_KEY environment variable must be set. Generate with: ./scripts/generate-encryption-key.sh")?;
    tracing::info!("Encryption service initialized - plugin configs will be encrypted");

    tracing::info!("Initializing analytics service...");
    let analytics_repo = create_analytics_repository(config, pool.clone()).await?;
    let storage_repo = StorageMetricsRepository::new(pool.clone());
    let analytics_service = AnalyticsService::new(analytics_repo, storage_repo);
    tracing::info!("Analytics service initialized successfully");

    let embedding_db = EmbeddingRepository::new(pool.clone());
    let metadata_search_db = MetadataSearchRepository::new(pool.clone());
    let task_db = TaskRepository::new(pool.clone());

    // Initialize webhook repositories
    let webhook_db = WebhookRepository::new(pool.clone());
    let webhook_event_db = WebhookEventRepository::new(pool.clone());
    let webhook_retry_db = WebhookRetryRepository::new(pool.clone());

    // Initialize authentication repositories
    let tenant_db = TenantRepository::new(pool.clone());
    let api_key_db = ApiKeyRepository::new(pool.clone());

    // Auth/tenant repositories (API keys, tenants)
    // NOTE: User/OAuth authentication removed - using API keys and master key only

    let folder_db = FolderRepository::new(pool.clone());
    let named_transformation_db = NamedTransformationRepository::new(pool.clone());

    let media_db = MediaRepository::new(pool.clone(), storage.clone());

    let is_production = config.is_production();

    tracing::info!(
        environment = %config.environment(),
        is_production = is_production,
        "Environment configuration loaded"
    );

    // Initialize semantic search if enabled (Anthropic/Claude cloud)
    #[cfg(feature = "semantic-search")]
    let semantic_search = if config.semantic_search_enabled() {
        let api_key = config
            .anthropic_api_key()
            .ok_or_else(|| {
                anyhow::anyhow!("SEMANTIC_SEARCH_ENABLED=true requires ANTHROPIC_API_KEY")
            })?
            .to_string();
        let voyage_key = config.voyage_api_key().map(|k| k.to_string());

        tracing::info!(
            vision_model = %config.anthropic_vision_model(),
            embedding_model = %config.voyage_embedding_model(),
            has_voyage_key = voyage_key.is_some(),
            "Semantic search enabled: Claude Vision + Voyage AI embeddings"
        );

        let svc = AnthropicService::new_with_voyage(
            api_key,
            voyage_key,
            config.anthropic_vision_model().to_string(),
            config.voyage_embedding_model().to_string(),
        );
        match svc.health_check().await {
            Ok(true) => tracing::info!("✅ Semantic search ready: Claude Vision + Voyage AI embeddings"),
            Ok(false) | Err(_) => tracing::warn!(
                "⚠️  Semantic search health check failed - check ANTHROPIC_API_KEY and VOYAGE_API_KEY"
            ),
        }
        Some(Arc::new(svc) as Arc<dyn SemanticSearchProvider + Send + Sync>)
    } else {
        tracing::info!("Semantic search disabled");
        None
    };
    #[cfg(not(feature = "semantic-search"))]
    let semantic_search = None;

    // Create configuration sub-structs
    // S3Config is only created if S3Service is available (S3 backend)
    let s3_config = s3_service.as_ref().map(|s3_svc| S3Config {
        service: s3_svc.clone(),
        bucket: config.s3_bucket().unwrap_or_default().to_string(),
        region: config
            .s3_region()
            .or_else(|| config.aws_region())
            .unwrap_or_default()
            .to_string(),
        endpoint_url: config.s3_endpoint().map(|s| s.to_string()),
    });

    let security_config = SecurityConfig {
        #[cfg(feature = "clamav")]
        clamav: clamav_service.clone(),
        clamav_enabled: config.clamav_enabled(),
        cors_origins: config.cors_origins().to_vec(),
    };

    let database_config = DatabaseConfig {
        max_connections: config.db_max_connections(),
        timeout_seconds: config.db_timeout_seconds(),
    };

    let image_config = ImageConfig {
        repository: media_db.clone(),
        max_file_size: config.max_file_size_bytes(),
        allowed_extensions: config.allowed_extensions().to_vec(),
        allowed_content_types: config.allowed_content_types().to_vec(),
        remove_exif: config.remove_exif(),
    };

    #[cfg(feature = "document")]
    let document_config = DocumentConfig {
        repository: media_db.clone(),
        max_file_size: config.max_document_size_bytes(),
        allowed_extensions: config.document_allowed_extensions().to_vec(),
        allowed_content_types: config.document_allowed_content_types().to_vec(),
    };

    #[cfg(feature = "audio")]
    let audio_config = AudioConfig {
        repository: media_db.clone(),
        max_file_size: config.max_audio_size_bytes(),
        allowed_extensions: config.audio_allowed_extensions().to_vec(),
        allowed_content_types: config.audio_allowed_content_types().to_vec(),
    };

    // Unified media configuration
    let file_group_repo = FileGroupRepository::new(pool.clone(), storage.clone());
    let media_config = MediaConfig {
        repository: media_db.clone(),
        file_group_repository: file_group_repo,
        storage: storage.clone(),
        // Image settings
        image_max_file_size: config.max_file_size_bytes(),
        image_allowed_extensions: config.allowed_extensions().to_vec(),
        image_allowed_content_types: config.allowed_content_types().to_vec(),
        remove_exif: config.remove_exif(),
        // Video settings
        video_max_file_size: config.max_video_size_bytes(),
        video_allowed_extensions: config.video_allowed_extensions().to_vec(),
        video_allowed_content_types: config.video_allowed_content_types().to_vec(),
        ffmpeg_path: config.ffmpeg_path().to_string(),
        hls_segment_duration: config.hls_segment_duration(),
        hls_variants: config.hls_variants().to_vec(),
        // Audio settings
        audio_max_file_size: config.max_audio_size_bytes(),
        audio_allowed_extensions: config.audio_allowed_extensions().to_vec(),
        audio_allowed_content_types: config.audio_allowed_content_types().to_vec(),
        // Document settings
        document_max_file_size: config.max_document_size_bytes(),
        document_allowed_extensions: config.document_allowed_extensions().to_vec(),
        document_allowed_content_types: config.document_allowed_content_types().to_vec(),
    };
    tracing::info!("MediaConfig initialized with unified repository");

    #[cfg(feature = "video")]
    let video_db = media_db.clone();
    #[cfg(feature = "video")]
    let video_config_init = VideoConfig {
        repository: media_db.clone(),
        job_queue: VideoJobQueue::dummy(),
        max_file_size: config.max_video_size_bytes(),
        allowed_extensions: config.video_allowed_extensions().to_vec(),
        allowed_content_types: config.video_allowed_content_types().to_vec(),
        ffmpeg_path: config.ffmpeg_path().to_string(),
        hls_segment_duration: config.hls_segment_duration(),
        hls_variants: config.hls_variants().to_vec(),
    };

    // Initialize cleanup service
    // CleanupService now uses the Storage trait abstraction instead of S3Service directly
    tracing::info!("Initializing cleanup service...");
    #[cfg(all(feature = "video", feature = "document", feature = "audio"))]
    let cleanup_service = CleanupService::new(
        Arc::new(media_db.clone()),
        storage.clone(),
        Some(Arc::new(task_db.clone())),
        config.task_retention_days(),
    );
    #[cfg(all(feature = "video", feature = "document", not(feature = "audio")))]
    let cleanup_service = CleanupService::new(
        Arc::new(media_db.clone()),
        storage.clone(),
        Some(Arc::new(task_db.clone())),
        config.task_retention_days(),
    );
    #[cfg(all(feature = "video", not(feature = "document"), feature = "audio"))]
    let cleanup_service = CleanupService::new(
        Arc::new(media_db.clone()),
        storage.clone(),
        Some(Arc::new(task_db.clone())),
        config.task_retention_days(),
    );
    #[cfg(all(feature = "video", not(feature = "document"), not(feature = "audio")))]
    let cleanup_service = CleanupService::new(
        Arc::new(media_db.clone()),
        storage.clone(),
        Some(Arc::new(task_db.clone())),
        config.task_retention_days(),
    );
    #[cfg(all(not(feature = "video"), feature = "document", feature = "audio"))]
    let cleanup_service = CleanupService::new(
        Arc::new(media_db.clone()),
        storage.clone(),
        Some(Arc::new(task_db.clone())),
        config.task_retention_days(),
    );
    #[cfg(all(not(feature = "video"), feature = "document", not(feature = "audio")))]
    let cleanup_service = CleanupService::new(
        Arc::new(media_db.clone()),
        storage.clone(),
        Some(Arc::new(task_db.clone())),
        config.task_retention_days(),
    );
    #[cfg(all(not(feature = "video"), not(feature = "document"), feature = "audio"))]
    let cleanup_service = CleanupService::new(
        Arc::new(media_db.clone()),
        storage.clone(),
        Some(Arc::new(task_db.clone())),
        config.task_retention_days(),
    );
    #[cfg(all(
        not(feature = "video"),
        not(feature = "document"),
        not(feature = "audio")
    ))]
    let cleanup_service = CleanupService::new(
        Arc::new(media_db.clone()),
        storage.clone(),
        Some(Arc::new(task_db.clone())),
        config.task_retention_days(),
    );
    tracing::info!("Cleanup service initialized successfully");

    // Initialize webhook services
    tracing::info!("Initializing webhook services...");
    let webhook_service_config = WebhookServiceConfig {
        timeout_seconds: config.webhook_timeout_seconds(),
        max_retries: config.webhook_max_retries() as i32,
        max_concurrent_deliveries: config.webhook_max_concurrent_deliveries(),
    };

    let webhook_service = WebhookService::new(
        webhook_db.clone(),
        webhook_event_db.clone(),
        webhook_retry_db.clone(),
        webhook_service_config,
    )
    .context("Failed to create webhook service")?;

    let webhook_retry_config = WebhookRetryServiceConfig {
        poll_interval_seconds: config.webhook_retry_poll_interval_seconds(),
        batch_size: config.webhook_retry_batch_size() as i64,
        max_concurrent_retries: config.webhook_max_concurrent_retries(),
    };

    let webhook_retry_service = WebhookRetryService::new(
        webhook_retry_db.clone(),
        Arc::new(webhook_service.clone()),
        webhook_retry_config,
    );
    tracing::info!("Webhook services initialized successfully");

    // Initialize plugin system - variables defined here are used conditionally in AppState construction
    #[cfg(feature = "plugin")]
    let (
        plugin_config_repo_init,
        plugin_execution_repo_init,
        plugin_registry_init,
        plugin_task_handler_init,
    ) = {
        tracing::info!("Initializing plugin system...");
        let plugin_config_repo =
            PluginConfigRepository::new_with_encryption(pool.clone(), encryption_service.clone());
        let plugin_execution_repo = PluginExecutionRepository::new(pool.clone());

        // Create plugin registry and register plugins
        let plugin_registry = PluginRegistry::new();

        #[cfg(all(feature = "plugin", feature = "plugin-assembly-ai"))]
        {
            let assembly_ai_plugin =
                Arc::new(AssemblyAiPlugin::new().context("Failed to create Assembly AI plugin")?);
            plugin_registry
                .register(
                    assembly_ai_plugin.clone(),
                    crate::plugins::PluginInfo {
                        name: "assembly_ai".to_string(),
                        description: "Assembly AI transcription service for audio files"
                            .to_string(),
                        supported_media_types: vec!["audio".to_string()],
                    },
                )
                .await
                .context("Failed to register Assembly AI plugin")?;
        }

        #[cfg(all(feature = "plugin", feature = "content-moderation"))]
        {
            let aws_rekognition_plugin = Arc::new(AwsRekognitionPlugin::new());
            plugin_registry
                .register(
                    aws_rekognition_plugin.clone(),
                    crate::plugins::PluginInfo {
                        name: "aws_rekognition".to_string(),
                        description: "AWS Rekognition object detection for images".to_string(),
                        supported_media_types: vec!["image".to_string()],
                    },
                )
                .await
                .context("Failed to register AWS Rekognition plugin")?;
        }

        // Note: AwsRekognitionModerationPlugin from mindia_plugins uses a different Plugin trait
        // than crate::plugins::Plugin, so we can't register it directly here.
        // Enabling it requires an adapter that maps the mindia_plugins traits to the local
        // plugin surface; enable when an adapter exists.
        // #[cfg(all(
        //     feature = "plugin",
        //     feature = "content-moderation",
        //     feature = "plugin-aws-rekognition-moderation"
        // ))]
        // {
        //     let aws_rekognition_moderation_plugin = Arc::new(AwsRekognitionModerationPlugin::new());
        //     plugin_registry
        //         .register(
        //             aws_rekognition_moderation_plugin.clone(),
        //             crate::plugins::PluginInfo {
        //                 name: "aws_rekognition_moderation".to_string(),
        //                 description: "AWS Rekognition content moderation for images and videos"
        //                     .to_string(),
        //                 supported_media_types: vec!["image".to_string(), "video".to_string()],
        //             },
        //         )
        //         .await
        //         .context("Failed to register AWS Rekognition moderation plugin")?;
        // }

        #[cfg(all(feature = "plugin", feature = "plugin-aws-transcribe"))]
        {
            let aws_transcribe_plugin = Arc::new(AwsTranscribePlugin::new());
            plugin_registry
                .register(
                    aws_transcribe_plugin.clone(),
                    crate::plugins::PluginInfo {
                        name: "aws_transcribe".to_string(),
                        description: "AWS Transcribe transcription service for audio files"
                            .to_string(),
                        supported_media_types: vec!["audio".to_string()],
                    },
                )
                .await
                .context("Failed to register AWS Transcribe plugin")?;
        }

        #[cfg(all(feature = "plugin", feature = "plugin-google-vision"))]
        {
            let google_vision_plugin = Arc::new(
                GoogleVisionPlugin::new().context("Failed to create Google Vision plugin")?,
            );
            plugin_registry
                .register(
                    google_vision_plugin.clone(),
                    crate::plugins::PluginInfo {
                        name: "google_vision".to_string(),
                        description: "Google Cloud Vision API for comprehensive image analysis"
                            .to_string(),
                        supported_media_types: vec!["image".to_string()],
                    },
                )
                .await
                .context("Failed to register Google Vision plugin")?;
        }

        #[cfg(all(feature = "plugin", feature = "plugin-claude-vision"))]
        {
            let claude_vision_plugin = Arc::new(
                ClaudeVisionPlugin::new().context("Failed to create Claude Vision plugin")?,
            );
            plugin_registry
                .register(
                    claude_vision_plugin.clone(),
                    crate::plugins::PluginInfo {
                        name: "claude_vision".to_string(),
                        description: "Anthropic Claude Vision API for comprehensive image analysis"
                            .to_string(),
                        supported_media_types: vec!["image".to_string()],
                    },
                )
                .await
                .context("Failed to register Claude Vision plugin")?;
        }

        #[cfg(all(feature = "plugin", feature = "plugin-openai-image-description"))]
        {
            let openai_plugin = Arc::new(OpenAiImageDescriptionPlugin::new());
            plugin_registry
                .register(
                    openai_plugin.clone(),
                    crate::plugins::PluginInfo {
                        name: "openai_image_description".to_string(),
                        description: "OpenAI ChatGPT image description generation using gpt-4o"
                            .to_string(),
                        supported_media_types: vec!["image".to_string()],
                    },
                )
                .await
                .context("Failed to register OpenAI Image Description plugin")?;
        }

        #[cfg(all(feature = "plugin", feature = "plugin-replicate-deoldify"))]
        {
            let replicate_deoldify_plugin = Arc::new(
                ReplicateDeoldifyPlugin::new()
                    .context("Failed to create Replicate DeOldify plugin")?,
            );
            plugin_registry
                .register(
                    replicate_deoldify_plugin.clone(),
                    crate::plugins::PluginInfo {
                        name: "replicate_deoldify".to_string(),
                        description: "Replicate DeOldify image colorization - add colors to old black and white images"
                            .to_string(),
                        supported_media_types: vec!["image".to_string()],
                    },
                )
                .await
                .context("Failed to register Replicate DeOldify plugin")?;
        }

        let plugin_registry = Arc::new(plugin_registry);
        tracing::info!(
            "Plugin registry initialized with {} plugin(s)",
            plugin_registry
                .list()
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!(error = %e, "Failed to list plugins for logging");
                    Vec::new()
                })
                .len()
        );

        // Create plugin task handler early (it doesn't depend on task_queue)
        let plugin_task_handler = PluginTaskHandler::new_with_encryption(
            plugin_registry.clone(),
            plugin_config_repo.clone(),
            plugin_execution_repo.clone(),
            encryption_service.clone(),
        );
        tracing::info!("Plugin task handler initialized successfully");

        (
            plugin_config_repo,
            plugin_execution_repo,
            plugin_registry,
            plugin_task_handler,
        )
    };

    // Initialize content moderation handler (uses plugin system)
    #[cfg(feature = "content-moderation")]
    let content_moderation_handler = {
        #[cfg(feature = "plugin")]
        {
            ContentModerationTaskHandler::new(plugin_registry_init.clone())
        }
        #[cfg(not(feature = "plugin"))]
        {
            return Err(anyhow::anyhow!(
                "Content moderation requires plugin feature to be enabled"
            ));
        }
    };

    tracing::info!("Plugin repositories initialized successfully");

    // Initialize task queue components
    tracing::info!("Initializing task queue system...");
    let video_rate = config.task_queue_video_rate_limit();
    let embedding_rate = config.task_queue_embedding_rate_limit();
    let rate_limiter = std::env::var("TASK_RATE_LIMITER_SHARD_COUNT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .map(|shard_count| {
            tracing::info!(
                shard_count = shard_count,
                "Task rate limiter using custom shard count"
            );
            RateLimiter::with_shards(video_rate, embedding_rate, shard_count.max(1))
        })
        .unwrap_or_else(|| RateLimiter::new(video_rate, embedding_rate));

    let task_queue_config = TaskQueueConfig {
        max_workers: config.task_queue_max_workers(),
        poll_interval_ms: config.task_queue_poll_interval_ms(),
        default_timeout_seconds: config.task_queue_default_timeout_seconds(),
        max_retries: config.task_queue_max_retries(),
    };

    // Initialize capacity checker (needed for temp_state)
    let capacity_checker_temp = Arc::new(CapacityChecker::new(config.clone()));

    // Create temporary state with a TaskQueue that does not spawn a worker, to break
    // circular dependency. VideoJobQueue holds Arc<AppState> to this; if we used a real
    // TaskQueue here, we would have two worker pools and duplicate shutdown logs.
    let temp_state = AppState {
        db_pool: pool.clone(),
        media: media_config.clone(),
        image: image_config.clone(),
        #[cfg(feature = "video")]
        video: video_config_init.clone(),
        #[cfg(feature = "document")]
        document: document_config.clone(),
        #[cfg(feature = "audio")]
        audio: audio_config.clone(),
        s3: s3_config.clone(),
        security: security_config.clone(),
        database: database_config.clone(),
        analytics: analytics_service.clone(),
        is_production,
        #[cfg(feature = "semantic-search")]
        semantic_search: semantic_search.clone(),
        embedding_repository: embedding_db.clone(),
        metadata_search_repository: metadata_search_db.clone(),
        cleanup_service: Some(cleanup_service.clone()),
        config: config.clone(),
        task_queue: TaskQueue::new_no_worker(
            task_db.clone(),
            rate_limiter.clone(),
            task_queue_config.clone(),
        ),
        task_repository: task_db.clone(),
        #[cfg(feature = "video")]
        video_db: video_db.clone(),
        webhook_repository: webhook_db.clone(),
        webhook_event_repository: webhook_event_db.clone(),
        webhook_retry_repository: webhook_retry_db.clone(),
        webhook_service: webhook_service.clone(),
        webhook_retry_service: webhook_retry_service.clone(),
        folder_repository: folder_db.clone(),
        api_key_repository: api_key_db.clone(),
        tenant_repository: tenant_db.clone(),
        named_transformation_repository: named_transformation_db.clone(),
        #[cfg(feature = "plugin")]
        plugin_registry: plugin_registry_init.clone(),
        #[cfg(feature = "plugin")]
        plugin_task_handler: plugin_task_handler_init.clone(),
        #[cfg(feature = "content-moderation")]
        content_moderation_handler: content_moderation_handler.clone(),
        capacity_checker: capacity_checker_temp.clone(),
        #[cfg(feature = "plugin")]
        plugin_service: PluginService::new_with_encryption(
            plugin_registry_init.clone(),
            plugin_config_repo_init.clone(),
            plugin_execution_repo_init.clone(),
            TaskQueue::new_no_worker(
                task_db.clone(),
                rate_limiter.clone(),
                task_queue_config.clone(),
            ),
            encryption_service.clone(),
        ),
        #[cfg(feature = "plugin")]
        plugin_config_repository: plugin_config_repo_init.clone(),
        #[cfg(feature = "plugin")]
        plugin_execution_repository: plugin_execution_repo_init.clone(),
    };

    let temp_state_arc = Arc::new(temp_state);
    #[cfg(feature = "video")]
    let video_job_queue =
        VideoJobQueue::new(temp_state_arc.clone(), config.max_concurrent_transcodes());

    // Embedding jobs are now handled via TaskQueue with EmbeddingTaskHandler
    // No separate embedding_queue needed

    // Create final video config with real job queue
    #[cfg(feature = "video")]
    let video_config = VideoConfig {
        repository: video_db.clone(),
        job_queue: video_job_queue,
        max_file_size: config.max_video_size_bytes(),
        allowed_extensions: config.video_allowed_extensions().to_vec(),
        allowed_content_types: config.video_allowed_content_types().to_vec(),
        ffmpeg_path: config.ffmpeg_path().to_string(),
        hls_segment_duration: config.hls_segment_duration(),
        hls_variants: config.hls_variants().to_vec(),
    };

    let state_weak = Arc::downgrade(&temp_state_arc);
    let task_queue = TaskQueue::new(
        task_db.clone(),
        rate_limiter.clone(),
        task_queue_config.clone(),
        state_weak,
        Some(pool.clone()),
    );
    tracing::info!(
        max_workers = config.task_queue_max_workers(),
        video_rate_limit = config.task_queue_video_rate_limit(),
        embedding_rate_limit = config.task_queue_embedding_rate_limit(),
        "Task queue system initialized successfully"
    );

    #[cfg(feature = "plugin")]
    let plugin_service_final = PluginService::new_with_encryption(
        plugin_registry_init.clone(),
        plugin_config_repo_init.clone(),
        plugin_execution_repo_init.clone(),
        task_queue.clone(),
        encryption_service.clone(),
    );
    #[cfg(feature = "plugin")]
    tracing::info!("Plugin service initialized successfully");

    // Initialize capacity checker
    tracing::info!("Initializing capacity checker...");
    let capacity_checker = Arc::new(CapacityChecker::new(config.clone()));
    tracing::info!(
        min_disk_free_gb = config.min_disk_free_gb(),
        max_memory_usage_percent = config.max_memory_usage_percent(),
        max_cpu_usage_percent = config.max_cpu_usage_percent(),
        "Capacity checker initialized"
    );

    // Create final state with all components including properly initialized task queue
    let state = Arc::new(AppState {
        db_pool: pool,
        media: media_config,
        image: image_config,
        #[cfg(feature = "video")]
        video: video_config,
        #[cfg(feature = "document")]
        document: document_config,
        #[cfg(feature = "audio")]
        audio: audio_config,
        s3: s3_config,
        security: security_config,
        database: database_config,
        analytics: analytics_service,
        is_production,
        #[cfg(feature = "semantic-search")]
        semantic_search,
        embedding_repository: embedding_db,
        metadata_search_repository: metadata_search_db,
        cleanup_service: Some(cleanup_service.clone()),
        config: config.clone(),
        task_queue,
        task_repository: task_db,
        #[cfg(feature = "video")]
        video_db,
        webhook_repository: webhook_db,
        webhook_event_repository: webhook_event_db,
        webhook_retry_repository: webhook_retry_db,
        webhook_service,
        webhook_retry_service,
        folder_repository: folder_db,
        api_key_repository: api_key_db,
        tenant_repository: tenant_db,
        named_transformation_repository: named_transformation_db,
        #[cfg(feature = "plugin")]
        plugin_registry: plugin_registry_init.clone(),
        #[cfg(feature = "plugin")]
        plugin_service: plugin_service_final,
        #[cfg(feature = "plugin")]
        plugin_config_repository: plugin_config_repo_init.clone(),
        #[cfg(feature = "plugin")]
        plugin_execution_repository: plugin_execution_repo_init.clone(),
        #[cfg(feature = "plugin")]
        plugin_task_handler: plugin_task_handler_init.clone(),
        #[cfg(feature = "content-moderation")]
        content_moderation_handler: content_moderation_handler.clone(),
        capacity_checker,
    });

    // Start background tasks
    let analytics_for_background = Arc::new(state.analytics.clone());
    start_storage_metrics_refresh(analytics_for_background, 6);
    tracing::info!("Started storage metrics refresh background task (every 6 hours)");

    if let Some(cleanup) = &state.cleanup_service {
        Arc::new(cleanup.clone()).start();
        tracing::info!("Started file cleanup background task (runs every hour)");
    }

    // Start pool metrics collection if OpenTelemetry is enabled
    #[cfg(feature = "observability-opentelemetry")]
    {
        use crate::telemetry::PoolMetrics;
        let meter = opentelemetry::global::meter("mindia");
        let pool_metrics = PoolMetrics::new(meter);
        crate::telemetry::start_pool_metrics_collector(state.db_pool.clone(), pool_metrics);
        tracing::info!("Started database pool metrics collection (every 30 seconds)");
    }

    Ok(state)
}
