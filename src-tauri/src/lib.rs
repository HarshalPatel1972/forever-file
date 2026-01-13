// Forever File - Tauri Bridge
// Event-Driven Async File Transfer Commands

use iroh::{protocol::Router, Endpoint};
use iroh_blobs::{net_protocol::Blobs, store::fs::Store, ticket::BlobTicket, util::local_pool::LocalPool};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use thiserror::Error;
use tokio::sync::oneshot;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum TransferError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Iroh error: {0}")]
    Iroh(#[from] anyhow::Error),
    #[error("Cancelled")]
    Cancelled,
}

// ============================================================================
// Event Payloads (match TypeScript interfaces)
// ============================================================================

#[derive(Clone, Serialize, Deserialize)]
pub struct TicketGeneratedEvent {
    pub ticket: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    pub sent: u64,
    pub total: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TransferCompleteEvent {
    pub success: bool,
    pub message: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub message: String,
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Start sending a file - returns immediately, emits events for progress
#[tauri::command]
async fn start_send(app: AppHandle, filepath: String) -> Result<(), String> {
    let app_clone = app.clone();
    
    // Spawn background task for async transfer
    tokio::spawn(async move {
        if let Err(e) = send_file_async(app_clone.clone(), filepath).await {
            let _ = app_clone.emit("forever-file://error", ErrorEvent {
                message: e.to_string(),
            });
        }
    });
    
    Ok(())
}

/// Start receiving a file using a ticket - returns immediately, emits events for progress
#[tauri::command]
async fn start_receive(app: AppHandle, ticket: String, output_dir: String) -> Result<(), String> {
    let app_clone = app.clone();
    
    // Spawn background task for async transfer
    tokio::spawn(async move {
        if let Err(e) = receive_file_async(app_clone.clone(), ticket, output_dir).await {
            let _ = app_clone.emit("forever-file://error", ErrorEvent {
                message: e.to_string(),
            });
        }
    });
    
    Ok(())
}

// ============================================================================
// Async Transfer Logic
// ============================================================================

async fn send_file_async(app: AppHandle, filepath: String) -> Result<(), TransferError> {
    let path = PathBuf::from(&filepath);
    
    // Get file size for progress tracking
    let metadata = tokio::fs::metadata(&path).await?;
    let total_size = metadata.len();
    
    // Create iroh endpoint
    let endpoint = Endpoint::builder().bind().await?;
    
    // Create blob store in temp directory
    let data_dir = std::env::temp_dir().join("forever-file-blobs");
    tokio::fs::create_dir_all(&data_dir).await?;
    let store = Store::load(&data_dir).await?;
    
    // Create local pool for blob operations
    let local_pool = LocalPool::default();
    
    // Create blobs protocol
    let blobs = Blobs::builder(store.clone())
        .local_pool(&local_pool)
        .build(&endpoint);
    
    // Add file to blob store
    let add_progress = blobs.add_from_path(path, true, Default::default()).await?;
    let hash = add_progress.hash;
    let blob_size = add_progress.size;
    
    // Create router to accept connections
    let router = Router::builder(endpoint.clone())
        .accept(iroh_blobs::ALPN, blobs.clone())
        .spawn();
    
    // Wait for endpoint to be online
    endpoint.home_relay().initialized().await?;
    
    // Generate ticket for receiver
    let ticket = BlobTicket::new(endpoint.node_addr().await?, hash, iroh_blobs::BlobFormat::Raw)?;
    
    // Emit ticket generated event
    app.emit("forever-file://ticket-generated", TicketGeneratedEvent {
        ticket: ticket.to_string(),
    })?;
    
    // Emit initial progress
    app.emit("forever-file://progress", ProgressEvent {
        sent: 0,
        total: blob_size,
    })?;
    
    // Keep the sender alive until explicitly stopped
    // In a real app, you'd wait for the transfer to complete
    // For now, we'll wait for a signal or timeout
    tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
    
    // Emit completion
    app.emit("forever-file://complete", TransferCompleteEvent {
        success: true,
        message: "File ready for transfer".to_string(),
    })?;
    
    Ok(())
}

async fn receive_file_async(app: AppHandle, ticket_str: String, output_dir: String) -> Result<(), TransferError> {
    // Parse the ticket
    let ticket: BlobTicket = ticket_str.parse()?;
    
    // Create iroh endpoint
    let endpoint = Endpoint::builder().bind().await?;
    
    // Create blob store
    let data_dir = std::env::temp_dir().join("forever-file-blobs-recv");
    tokio::fs::create_dir_all(&data_dir).await?;
    let store = Store::load(&data_dir).await?;
    
    // Create local pool
    let local_pool = LocalPool::default();
    
    // Create blobs protocol
    let blobs = Blobs::builder(store.clone())
        .local_pool(&local_pool)
        .build(&endpoint);
    
    // Emit starting
    app.emit("forever-file://progress", ProgressEvent {
        sent: 0,
        total: 0,
    })?;
    
    // Download the blob
    let download = blobs.download(ticket.hash(), ticket.node_addr().clone()).await?;
    download.await?;
    
    // Export to output directory
    let output_path = PathBuf::from(&output_dir).join(format!("{}", ticket.hash()));
    let entry = blobs.read(ticket.hash()).await?;
    
    if let Some(reader) = entry {
        let data = reader.read_to_bytes().await?;
        tokio::fs::write(&output_path, data).await?;
    }
    
    // Emit completion
    app.emit("forever-file://complete", TransferCompleteEvent {
        success: true,
        message: format!("File saved to {:?}", output_path),
    })?;
    
    Ok(())
}

// ============================================================================
// Tauri Entry Point
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![start_send, start_receive])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
