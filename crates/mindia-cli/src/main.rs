//! Mindia CLI — command-line client for the Mindia API.
//!
//! Set MINDIA_API_KEY and MINDIA_API_URL (or API_URL). Uses X-API-Key auth.

use anyhow::Context;
use clap::{Parser, Subcommand};
use mindia_api_client::ApiClient;
use mindia_cli::init_tracing;
use serde::Serialize;

#[derive(Parser)]
#[command(name = "mindia", about = "Mindia API CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Upload a file (image, document, video, or audio)
    Upload {
        /// Path to the file to upload
        file: std::path::PathBuf,
    },
    /// Upload an image from a URL
    UploadUrl {
        /// URL of the image to download and upload
        url: String,
    },
    /// List media with optional type filter and pagination
    List {
        /// Filter by type: image, video, audio, document
        #[arg(long)]
        r#type: Option<String>,
        /// Maximum number of items
        #[arg(long, default_value = "20")]
        limit: u32,
        /// Offset for pagination
        #[arg(long, default_value = "0")]
        offset: u32,
    },
    /// Get a single media item by ID
    Get {
        /// Media UUID
        id: String,
    },
    /// Semantic search
    Search {
        /// Search query
        query: String,
        /// Maximum number of results
        #[arg(long, default_value = "20")]
        limit: Option<u32>,
    },
    /// Delete a media item by ID
    Delete {
        /// Media UUID
        id: String,
    },
    /// Get a transformed image URL (resize dimensions)
    Transform {
        /// Image UUID
        image_id: String,
        /// Width in pixels
        #[arg(long)]
        width: Option<u32>,
        /// Height in pixels
        #[arg(long)]
        height: Option<u32>,
    },
    /// Folder operations
    Folder {
        #[command(subcommand)]
        sub: FolderCommands,
    },
    /// Get storage summary (analytics)
    Storage,
}

#[derive(Subcommand)]
enum FolderCommands {
    /// Create a new folder
    Create {
        /// Folder name
        name: String,
        /// Parent folder UUID
        #[arg(long)]
        parent: Option<String>,
    },
}

fn print_json(value: &impl Serialize) -> anyhow::Result<()> {
    let out = serde_json::to_string_pretty(value).context("Serialize response")?;
    println!("{}", out);
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    dotenvy::dotenv().ok();

    let client = ApiClient::from_env().context(
        "Failed to create API client. Set MINDIA_API_KEY and MINDIA_API_URL (or API_URL)",
    )?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Upload { file } => {
            let path = file.to_string_lossy();
            let response = client.upload_image(&path).await?;
            print_json(&response)?;
        }
        Commands::UploadUrl { url } => {
            let response = client.upload_image_from_url(&url).await?;
            print_json(&response)?;
        }
        Commands::List {
            r#type,
            limit,
            offset,
        } => {
            let media_type = r#type.as_deref();
            let response = client
                .list_media(media_type, Some(limit), Some(offset))
                .await?;
            print_json(&response)?;
        }
        Commands::Get { id } => {
            let response = client.get_media(&id).await?;
            print_json(&response)?;
        }
        Commands::Search { query, limit } => {
            let response = client.search_media(&query, limit).await?;
            print_json(&response)?;
        }
        Commands::Delete { id } => {
            client.delete_media(&id).await?;
            print_json(
                &serde_json::json!({ "success": true, "message": format!("Media {} deleted", id) }),
            )?;
        }
        Commands::Transform {
            image_id,
            width,
            height,
        } => {
            let url = client.transform_image(&image_id, width, height).await?;
            print_json(&serde_json::json!({ "transformed_url": url }))?;
        }
        Commands::Folder { sub } => match sub {
            FolderCommands::Create { name, parent } => {
                let response = client.create_folder(&name, parent.as_deref()).await?;
                print_json(&response)?;
            }
        },
        Commands::Storage => {
            let response = client.get_storage_summary().await?;
            print_json(&response)?;
        }
    }

    Ok(())
}
