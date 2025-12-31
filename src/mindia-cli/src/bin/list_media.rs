use anyhow::Result;
use clap::Parser;
use mindia_core::models::MediaType;
use uuid::Uuid;

use mindia_cli::api_client::{
    ApiClient, AudioResponse, DocumentResponse, ImageResponse, VideoResponse,
};

#[derive(Parser, Debug)]
#[command(name = "list_media")]
#[command(about = "List media files in a folder or root")]
struct Args {
    /// Optional folder ID to list (if not provided, lists root folder)
    #[arg(long, value_name = "UUID")]
    folder_id: Option<Uuid>,

    /// Optional media type filter: image, video, audio, document
    #[arg(long, value_name = "TYPE")]
    media_type: Option<String>,

    /// Limit number of results (default: 100)
    #[arg(long, default_value = "100")]
    limit: i64,

    /// Offset for pagination (default: 0)
    #[arg(long, default_value = "0")]
    offset: i64,

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

    // Parse media type if provided
    let media_type_filter: Option<MediaType> = if let Some(ref mt) = args.media_type {
        Some(match mt.as_str() {
            "image" => MediaType::Image,
            "video" => MediaType::Video,
            "audio" => MediaType::Audio,
            "document" => MediaType::Document,
            _ => {
                return Err(anyhow::anyhow!(
                    "Invalid media type. Must be: image, video, audio, or document"
                ))
            }
        })
    } else {
        None
    };

    // List media by calling appropriate API endpoints
    let media_list = if let Some(mt) = media_type_filter {
        match mt {
            MediaType::Image => {
                let images = client
                    .list_images(args.limit, args.offset, args.folder_id)
                    .await?;
                convert_images_to_media_items(images)
            }
            MediaType::Video => {
                let videos = client
                    .list_videos(args.limit, args.offset, args.folder_id)
                    .await?;
                convert_videos_to_media_items(videos)
            }
            MediaType::Audio => {
                let audios = client
                    .list_audios(args.limit, args.offset, args.folder_id)
                    .await?;
                convert_audios_to_media_items(audios)
            }
            MediaType::Document => {
                let documents = client
                    .list_documents(args.limit, args.offset, args.folder_id)
                    .await?;
                convert_documents_to_media_items(documents)
            }
        }
    } else {
        // Fetch all types and combine
        let (images, videos, audios, documents) = tokio::try_join!(
            client.list_images(args.limit, args.offset, args.folder_id),
            client.list_videos(args.limit, args.offset, args.folder_id),
            client.list_audios(args.limit, args.offset, args.folder_id),
            client.list_documents(args.limit, args.offset, args.folder_id),
        )?;

        let mut items = vec![];
        items.extend(convert_images_to_media_items(images).items);
        items.extend(convert_videos_to_media_items(videos).items);
        items.extend(convert_audios_to_media_items(audios).items);
        items.extend(convert_documents_to_media_items(documents).items);

        // Sort by uploaded_at descending
        items.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at));

        // Apply pagination manually since we fetched all types separately
        // Note: This is approximate pagination. For accurate pagination across all types,
        // we'd need a unified API endpoint or to fetch with a larger limit and paginate client-side
        let start = args.offset as usize;
        let end = (args.offset + args.limit) as usize;
        let items: Vec<_> = items
            .into_iter()
            .skip(start)
            .take((end - start).max(0))
            .collect();

        let total_count = items.len() as i64; // Approximate, actual total would need separate endpoint

        MediaList {
            items,
            total_count,
            limit: args.limit,
            offset: args.offset,
        }
    };

    // Output results
    match args.format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&media_list)?);
        }
        _ => {
            print_media_table(&media_list, args.folder_id);
        }
    }

    Ok(())
}

#[derive(serde::Serialize)]
struct MediaItem {
    id: Uuid,
    media_type: String,
    filename: String,
    original_filename: String,
    content_type: String,
    file_size: i64,
    file_size_mb: f64,
    uploaded_at: chrono::DateTime<chrono::Utc>,
    folder_id: Option<Uuid>,
}

#[derive(serde::Serialize)]
struct MediaList {
    items: Vec<MediaItem>,
    total_count: i64,
    limit: i64,
    offset: i64,
}

fn convert_images_to_media_items(images: Vec<ImageResponse>) -> MediaList {
    let items: Vec<MediaItem> = images
        .into_iter()
        .map(|img| {
            let file_size = img.file_size;
            MediaItem {
                id: img.id,
                media_type: "image".to_string(),
                filename: img.filename.clone(),
                original_filename: img.filename,
                content_type: img.content_type,
                file_size,
                file_size_mb: (file_size as f64) / (1024.0 * 1024.0),
                uploaded_at: img.uploaded_at,
                folder_id: img.folder_id,
            }
        })
        .collect();

    MediaList {
        total_count: items.len() as i64,
        limit: items.len() as i64,
        offset: 0,
        items,
    }
}

fn convert_videos_to_media_items(videos: Vec<VideoResponse>) -> MediaList {
    let items: Vec<MediaItem> = videos
        .into_iter()
        .map(|vid| {
            let file_size = vid.file_size;
            MediaItem {
                id: vid.id,
                media_type: "video".to_string(),
                filename: vid.filename.clone(),
                original_filename: vid.filename,
                content_type: vid.content_type,
                file_size,
                file_size_mb: (file_size as f64) / (1024.0 * 1024.0),
                uploaded_at: vid.uploaded_at,
                folder_id: vid.folder_id,
            }
        })
        .collect();

    MediaList {
        total_count: items.len() as i64,
        limit: items.len() as i64,
        offset: 0,
        items,
    }
}

fn convert_audios_to_media_items(audios: Vec<AudioResponse>) -> MediaList {
    let items: Vec<MediaItem> = audios
        .into_iter()
        .map(|aud| {
            let file_size = aud.file_size;
            MediaItem {
                id: aud.id,
                media_type: "audio".to_string(),
                filename: aud.filename.clone(),
                original_filename: aud.filename,
                content_type: aud.content_type,
                file_size,
                file_size_mb: (file_size as f64) / (1024.0 * 1024.0),
                uploaded_at: aud.uploaded_at,
                folder_id: aud.folder_id,
            }
        })
        .collect();

    MediaList {
        total_count: items.len() as i64,
        limit: items.len() as i64,
        offset: 0,
        items,
    }
}

fn convert_documents_to_media_items(documents: Vec<DocumentResponse>) -> MediaList {
    let items: Vec<MediaItem> = documents
        .into_iter()
        .map(|doc| {
            let file_size = doc.file_size;
            MediaItem {
                id: doc.id,
                media_type: "document".to_string(),
                filename: doc.filename.clone(),
                original_filename: doc.filename,
                content_type: doc.content_type,
                file_size,
                file_size_mb: (file_size as f64) / (1024.0 * 1024.0),
                uploaded_at: doc.uploaded_at,
                folder_id: doc.folder_id,
            }
        })
        .collect();

    MediaList {
        total_count: items.len() as i64,
        limit: items.len() as i64,
        offset: 0,
        items,
    }
}

fn print_media_table(list: &MediaList, folder_id: Option<Uuid>) {
    println!("\n=== Media List ===\n");

    if let Some(fid) = folder_id {
        println!("Folder ID: {}", fid);
    } else {
        println!("Location: Root (no folder)");
    }

    println!(
        "Total: {} items (showing {} to {} of {})",
        list.total_count,
        list.offset + 1,
        std::cmp::min(list.offset + list.items.len() as i64, list.total_count),
        list.total_count
    );

    if list.items.is_empty() {
        println!("\nNo media found.");
        return;
    }

    println!(
        "\n{:<36} {:<8} {:<30} {:<20} {:>12} {:>20}",
        "ID", "Type", "Original Filename", "Content Type", "Size (MB)", "Uploaded At"
    );
    println!("{}", "-".repeat(150));

    for item in &list.items {
        println!(
            "{:<36} {:<8} {:<30} {:<20} {:>12.2} {:>20}",
            item.id.to_string().chars().take(36).collect::<String>(),
            item.media_type,
            truncate_string(&item.original_filename, 30),
            truncate_string(&item.content_type, 20),
            item.file_size_mb,
            item.uploaded_at.format("%Y-%m-%d %H:%M:%S")
        );
    }

    if (list.offset + list.items.len() as i64) < list.total_count {
        println!("\n... (more items available, use --offset to see more)");
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
