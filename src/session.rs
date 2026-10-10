use std::error::Error;

use futures_util::StreamExt;
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::mpsc};
use tokio_util::codec::{FramedRead, LinesCodec};

use crate::server::{SESSION_MAILBOX_CAPACITY, ServerHandle, SessionEvent};

/// Reject input lines larger than 4 KiB to keep an untrusted client from
/// growing the decoder buffer without a limit.
const MAX_LINE_LENGTH: usize = 4 * 1024;

type BoxError = Box<dyn Error + Send + Sync>;

/// Runs the complete lifecycle of one TCP client.
///
/// The task owns the socket. Shared server state remains inside `ServerActor`
/// and is accessed only by sending messages through `ServerHandle`.
pub async fn process_client(stream: TcpStream, server: ServerHandle) -> Result<(), BoxError> {
    let address = stream.peer_addr()?;

    // The read and write halves may be awaited independently by `select!`.
    let (reader, mut writer) = stream.into_split();

    // LinesCodec turns the byte stream into bounded, newline-delimited input.
    let mut lines = FramedRead::new(reader, LinesCodec::new_with_max_length(MAX_LINE_LENGTH));

    // This is the session's private outbound mailbox. The actor keeps the
    // sender, while this task exclusively owns and drains the receiver.
    let (output, mut events) = mpsc::channel(SESSION_MAILBOX_CAPACITY);

    let session_id = server.connect(address, output).await?;

    // Keep post-registration work inside one future so cleanup below runs
    // after normal disconnects as well as read, write, or mailbox errors.
    let result = async {
        writer
            .write_all(
                b"Welcome to Ferrum MUD!\r\n\
                Commands: who, say <message>, quit.\r\n",
            )
            .await?;

        loop {
            tokio::select! {
                // Wait for the next line without blocking outbound messages.
                incoming = lines.next() => {
                    match incoming {
                        Some(Ok(line)) if line.trim().is_empty() => {

                        }
                        Some(Ok(line)) => server.input(session_id, line).await?,
                        Some(Err(error)) => return Err(error.into()),
                        None => break,
                    }
                }

                // Wait for messages routed back by the server actor without
                // blocking new network input.
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

    // Cleanup is best-effort: if the actor already stopped, there is no
    // routing table left from which this session needs to be removed.
    let _ = server.disconnect(session_id).await;

    result
}
