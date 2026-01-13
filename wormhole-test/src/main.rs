use magic_wormhole::{
    transfer::{receive_file, send_file},
    transit::{Abilities, TransitInfo},
    Code, Wormhole,
};
use std::path::PathBuf;
use thiserror::Error;
use tokio::sync::oneshot;
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[derive(Error, Debug)]
pub enum TransferError {
    #[error("Wormhole error: {0}")]
    Wormhole(#[from] magic_wormhole::wormhole::WormholeError),
    #[error("Transfer error: {0}")]
    Transfer(#[from] magic_wormhole::transfer::TransferError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("Cancelled")]
    Cancelled,
}

pub struct TransferManager;

impl TransferManager {
    pub async fn send_file(file_path: PathBuf) -> Result<String, TransferError> {
        let (code_tx, code_rx) = oneshot::channel();
        let file_path_clone = file_path.clone();

        tokio::spawn(async move {
            let result = async move {
                let (_welcome, wormhole_future) = Wormhole::connect_without_code(
                    magic_wormhole::transfer::APPID,
                    1,
                )
                .await?;

                let (wormhole, code) = wormhole_future.await?;
                let _ = code_tx.send(code);

                send_file(
                    wormhole,
                    vec![file_path_clone],
                    Abilities::ALL_ABILITIES,
                    TransitInfo::allow_everything(),
                    &mut |_| {},
                    async { },
                )
                .await?;

                Ok::<(), TransferError>(())
            }
            .await;

            if let Err(e) = result {
                eprintln!("Send task failed: {}", e);
            }
        });

        let code = code_rx.await.map_err(|_| TransferError::Cancelled)?;
        Ok(code.0)
    }

    pub async fn receive_file(code: String, output_dir: PathBuf) -> Result<(), TransferError> {
        let (_, wormhole) = Wormhole::connect_with_code(
            magic_wormhole::transfer::APPID, 
            1, 
            Code(code)
        ).await?;

        receive_file(
            wormhole,
            &output_dir,
            &mut |_| {},
            async { },
        ).await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), TransferError> {
    println!("=== Wormhole Transfer Test ===");
    
    let test_dir = std::env::temp_dir().join("wormhole_test");
    if test_dir.exists() {
        fs::remove_dir_all(&test_dir).await?;
    }
    fs::create_dir_all(&test_dir).await?;

    let src_file = test_dir.join("test_src.txt");
    let dest_dir = test_dir.join("output");
    fs::create_dir_all(&dest_dir).await?;

    let content = "Hello Wormhole from Rust!";
    let mut file = fs::File::create(&src_file).await?;
    file.write_all(content.as_bytes()).await?;
    println!("Created test file: {:?}", src_file);

    // 1. Start Sender
    println!("Starting sender...");
    let code = TransferManager::send_file(src_file.clone()).await?;
    println!("Generated Code: {}", code);

    // 2. Start Receiver
    println!("Starting receiver...");
    TransferManager::receive_file(code, dest_dir.clone()).await?;

    // 3. Verify
    let received_file = dest_dir.join("test_src.txt");
    let received_content = fs::read_to_string(&received_file).await?;

    if content == received_content {
        println!("✅ SUCCESS! File transferred and verified correctly!");
    } else {
        println!("❌ FAILED! Content mismatch.");
    }

    // Cleanup
    fs::remove_dir_all(test_dir).await?;

    Ok(())
}
