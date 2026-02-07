use anyhow::Result;
use clap::Parser;
use uuid::Uuid;

use mindia_cli::api_client::{
    ApiClient, AudioResponse, DocumentResponse, ImageResponse, StorageSummaryResponse,
    VideoResponse,
};

#[derive(Parser, Debug)]
#[command(name = "media_stats")]
#[command(about = "Get statistics about media files")]
struct Args {
    /// Optional folder ID to filter by
    #[arg(long, value_name = "UUID")]
    folder_id: Option<Uuid>,

    /// Output format: json or table (default: table)
    #[arg(long, default_value = "table")]
    format: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    // Initialize API client
    let client = ApiClient::from_env()?;

    // Use API endpoint when no folder filter is provided (faster, uses pre-calculated stats)
    // Fall back to manual calculation when folder_id is provided (API doesn't support folder filtering)
    let stats = if args.folder_id.is_none() {
        // Use API endpoint for better performance
        match client.get_storage_summary().await {
            Ok(api_stats) => map_api_stats_to_media_stats(api_stats, args.folder_id),
            Err(e) => {
                eprintln!("Warning: Failed to fetch stats from API ({}), falling back to manual calculation", e);
                // Fall back to manual calculation on API error
                let limit = 10000;
                let offset = 0;
                let (images, videos, audios, documents) = tokio::try_join!(
                    client.list_images(limit, offset, args.folder_id),
                    client.list_videos(limit, offset, args.folder_id),
                    client.list_audios(limit, offset, args.folder_id),
                    client.list_documents(limit, offset, args.folder_id),
                )?;
                calculate_stats(images, videos, audios, documents, args.folder_id)
            }
        }
    } else {
        // Manual calculation for folder-specific stats
        let limit = 10000;
        let offset = 0;
        let (images, videos, audios, documents) = tokio::try_join!(
            client.list_images(limit, offset, args.folder_id),
            client.list_videos(limit, offset, args.folder_id),
            client.list_audios(limit, offset, args.folder_id),
            client.list_documents(limit, offset, args.folder_id),
        )?;
        calculate_stats(images, videos, audios, documents, args.folder_id)
    };

    // Output results
    match args.format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&stats)?);
        }
        _ => {
            print_stats_table(&stats);
        }
    }

    Ok(())
}

#[derive(serde::Serialize)]
struct MediaStats {
    total_media: i64,
    total_size_bytes: i64,
    total_size_mb: f64,
    by_type: TypeStats,
    in_folder: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    by_content_type: Option<std::collections::HashMap<String, ContentTypeDisplay>>,
}

#[derive(serde::Serialize)]
struct TypeStats {
    images: CountAndSize,
    videos: CountAndSize,
    audio: CountAndSize,
    documents: CountAndSize,
}

#[derive(serde::Serialize)]
struct CountAndSize {
    count: i64,
    size_bytes: i64,
    size_mb: f64,
}

#[derive(serde::Serialize)]
struct ContentTypeDisplay {
    count: i64,
    size_bytes: i64,
    size_mb: f64,
}

/// Map API StorageSummaryResponse to MediaStats format
fn map_api_stats_to_media_stats(
    api_stats: StorageSummaryResponse,
    folder_id: Option<Uuid>,
) -> MediaStats {
    // Calculate document stats from by_content_type or as remainder
    // Document MIME types typically start with "application/" or "text/"
    let document_count = api_stats
        .by_content_type
        .iter()
        .filter(|(content_type, _)| {
            content_type.starts_with("application/")
                || content_type.starts_with("text/")
                || *content_type == "application/pdf"
        })
        .map(|(_, stats)| stats.count)
        .sum::<i64>();

    let document_bytes = api_stats
        .by_content_type
        .iter()
        .filter(|(content_type, _)| {
            content_type.starts_with("application/")
                || content_type.starts_with("text/")
                || *content_type == "application/pdf"
        })
        .map(|(_, stats)| stats.bytes)
        .sum::<i64>();

    // Convert by_content_type to display format
    let by_content_type_display: std::collections::HashMap<String, ContentTypeDisplay> = api_stats
        .by_content_type
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                ContentTypeDisplay {
                    count: v.count,
                    size_bytes: v.bytes,
                    size_mb: (v.bytes as f64) / (1024.0 * 1024.0),
                },
            )
        })
        .collect();

    MediaStats {
        total_media: api_stats.total_files,
        total_size_bytes: api_stats.total_storage_bytes,
        total_size_mb: (api_stats.total_storage_bytes as f64) / (1024.0 * 1024.0),
        by_type: TypeStats {
            images: CountAndSize {
                count: api_stats.image_count,
                size_bytes: api_stats.image_bytes,
                size_mb: (api_stats.image_bytes as f64) / (1024.0 * 1024.0),
            },
            videos: CountAndSize {
                count: api_stats.video_count,
                size_bytes: api_stats.video_bytes,
                size_mb: (api_stats.video_bytes as f64) / (1024.0 * 1024.0),
            },
            audio: CountAndSize {
                count: api_stats.audio_count,
                size_bytes: api_stats.audio_bytes,
                size_mb: (api_stats.audio_bytes as f64) / (1024.0 * 1024.0),
            },
            documents: CountAndSize {
                count: document_count,
                size_bytes: document_bytes,
                size_mb: (document_bytes as f64) / (1024.0 * 1024.0),
            },
        },
        in_folder: folder_id,
        by_content_type: Some(by_content_type_display),
    }
}

fn calculate_stats(
    images: Vec<ImageResponse>,
    videos: Vec<VideoResponse>,
    audios: Vec<AudioResponse>,
    documents: Vec<DocumentResponse>,
    folder_id: Option<Uuid>,
) -> MediaStats {
    let image_count = images.len() as i64;
    let image_bytes: i64 = images.iter().map(|img| img.file_size).sum();

    let video_count = videos.len() as i64;
    let video_bytes: i64 = videos.iter().map(|vid| vid.file_size).sum();

    let audio_count = audios.len() as i64;
    let audio_bytes: i64 = audios.iter().map(|aud| aud.file_size).sum();

    let document_count = documents.len() as i64;
    let document_bytes: i64 = documents.iter().map(|doc| doc.file_size).sum();

    let total_media = image_count + video_count + audio_count + document_count;
    let total_size_bytes = image_bytes + video_bytes + audio_bytes + document_bytes;

    MediaStats {
        total_media,
        total_size_bytes,
        total_size_mb: (total_size_bytes as f64) / (1024.0 * 1024.0),
        by_type: TypeStats {
            images: CountAndSize {
                count: image_count,
                size_bytes: image_bytes,
                size_mb: (image_bytes as f64) / (1024.0 * 1024.0),
            },
            videos: CountAndSize {
                count: video_count,
                size_bytes: video_bytes,
                size_mb: (video_bytes as f64) / (1024.0 * 1024.0),
            },
            audio: CountAndSize {
                count: audio_count,
                size_bytes: audio_bytes,
                size_mb: (audio_bytes as f64) / (1024.0 * 1024.0),
            },
            documents: CountAndSize {
                count: document_count,
                size_bytes: document_bytes,
                size_mb: (document_bytes as f64) / (1024.0 * 1024.0),
            },
        },
        in_folder: folder_id,
        by_content_type: None, // Manual calculation doesn't provide content type breakdown
    }
}

fn print_stats_table(stats: &MediaStats) {
    println!("\n=== Media Statistics ===\n");

    if let Some(folder_id) = stats.in_folder {
        println!("Folder ID: {}", folder_id);
        println!("Note: Folder-specific stats calculated manually (API doesn't support folder filtering)");
    } else {
        println!("Location: Root (all folders)");
        println!("Stats from API endpoint: /api/analytics/storage");
    }

    println!("\nTotal Media: {}", stats.total_media);
    println!(
        "Total Size: {:.2} MB ({} bytes)",
        stats.total_size_mb, stats.total_size_bytes
    );

    println!("\n--- By Type ---");
    println!(
        "Images:    {:>6} files, {:>12.2} MB",
        stats.by_type.images.count, stats.by_type.images.size_mb
    );
    println!(
        "Videos:    {:>6} files, {:>12.2} MB",
        stats.by_type.videos.count, stats.by_type.videos.size_mb
    );
    println!(
        "Audio:     {:>6} files, {:>12.2} MB",
        stats.by_type.audio.count, stats.by_type.audio.size_mb
    );
    println!(
        "Documents: {:>6} files, {:>12.2} MB",
        stats.by_type.documents.count, stats.by_type.documents.size_mb
    );

    // Display content type breakdown if available (from API)
    if let Some(ref by_content_type) = stats.by_content_type {
        if !by_content_type.is_empty() {
            println!("\n--- By Content Type (Top 10) ---");
            let mut sorted_types: Vec<_> = by_content_type.iter().collect();
            sorted_types.sort_by(|a, b| b.1.size_bytes.cmp(&a.1.size_bytes));

            for (content_type, type_stats) in sorted_types.iter().take(10) {
                println!(
                    "{:<40} {:>6} files, {:>12.2} MB",
                    truncate_string(content_type, 38),
                    type_stats.count,
                    type_stats.size_mb
                );
            }

            if by_content_type.len() > 10 {
                println!("... and {} more content types", by_content_type.len() - 10);
            }
        }
    }

    println!();
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
