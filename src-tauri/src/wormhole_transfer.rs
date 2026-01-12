use magic_wormhole::{
    transfer::{receive_file, send_file},
    transit::{Abilities, TransitInfo},
    Code, Wormhole,
};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::sync::oneshot;

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
                let (welcome, wormhole_future) = Wormhole::connect_without_code(
                    magic_wormhole::transfer::APPID,
                    1, // TODO: Use proper relay server
                )
                .await?;

                let (wormhole, code) = wormhole_future.await?;
                
                // Send the code back to the caller
                let _ = code_tx.send(code);

                // Wait for receiver and send file
                send_file(
                    wormhole,
                    vec![file_path_clone],
                    Abilities::ALL_ABILITIES,
                    TransitInfo::allow_everything(),
                    &mut |_| {}, // Progress handler
                    async { },    // Cancel future
                )
                .await?;

                Ok::<(), TransferError>(())
            }
            .await;

            if let Err(e) = result {
                eprintln!("Send task failed: {}", e);
            }
        });

        // Wait for the code to be generated
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
            // Abilities::ALL_ABILITIES, // Not needed for receive?
            // TransitInfo::allow_everything(),
            &mut |_| {}, // Progress handler
            async { },    // Cancel future
        ).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_local_transfer() -> Result<(), TransferError> {
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

        // 1. Start Sender
        let code = TransferManager::send_file(src_file.clone()).await?;
        println!("Generated Code: {}", code);

        // 2. Start Receiver
        TransferManager::receive_file(code, dest_dir.clone()).await?;

        // 3. Verify
        let received_file = dest_dir.join("test_src.txt");
        let received_content = fs::read_to_string(received_file).await?;

        assert_eq!(content, received_content);

        // Cleanup
        fs::remove_dir_all(test_dir).await?;

        Ok(())
    }
}
