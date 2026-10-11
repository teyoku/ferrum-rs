use std::error::Error;

use futures_util::StreamExt;
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::mpsc};
use tokio_util::codec::{FramedRead, LinesCodec};

use crate::server::{SESSION_MAILBOX_CAPACITY, ServerHandle, SessionEvent};

const MAX_LINE_LENGTH: usize = 4 * 1024;

type BoxError = Box<dyn Error + Send + Sync>;

pub async fn process_client(stream: TcpStream, server: ServerHandle) -> Result<(), BoxError> {
    let address = stream.peer_addr()?;
    let (reader, mut writer) = stream.into_split();

    let mut lines = FramedRead::new(reader, LinesCodec::new_with_max_length(MAX_LINE_LENGTH));

    let (output, mut events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

    let session_id = server.connect(address, output).await?;

    let result = async {
        writer
            .write_all(
                b"Welcome to Ferrum MUD!\r\n\
                Commands: who, say <message>, quit.\r\n",
            )
            .await?;

        loop {
            tokio::select! {
                incoming = lines.next() => {
                    match incoming {
                        Some(Ok(line)) if line.trim().is_empty() => {

                        }
                        Some(Ok(line)) => server.input(session_id, line).await?,
                        Some(Err(error)) => return Err(error.into()),
                        None => break,
                    }
                }

                event = events.recv() => {
                    match event {
                        Some(SessionEvent::Text(text)) => writer.write_all(text.as_bytes()).await?,
                        Some(SessionEvent::Close) => {
                            writer.write_all(b"Goodbye!\r\n").await?;
                            break;
                        }
                        None => break,
                    }
                }
            }
        }

        Ok(())
    }
    .await;

    let _ = server.disconnect(session_id).await;

    result
}
