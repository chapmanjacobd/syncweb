use std::sync::Arc;

use n0_future::{SinkExt, StreamExt, split};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

use crate::error::{Result, SyncwebError};

use super::{encoding, service::BridgeService};

const FRAME_HEADER_LEN: usize = 9;

pub struct WsSession {
    service: Arc<BridgeService>,
    sender: split::SplitSink<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, Message>,
    shutdown: broadcast::Receiver<()>,
}

impl WsSession {
    #[must_use]
    pub const fn new(
        service: Arc<BridgeService>,
        sender: split::SplitSink<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, Message>,
        shutdown: broadcast::Receiver<()>,
    ) -> Self {
        Self {
            service,
            sender,
            shutdown,
        }
    }

    /// Run the read loop until the connection or shutdown is signalled.
    ///
    /// # Errors
    ///
    /// Returns an error if the WebSocket stream produces a fatal error.
    pub async fn run(
        mut self,
        receiver: &mut split::SplitStream<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>>,
    ) -> Result<()> {
        self.send_node_status("connected").await;

        loop {
            tokio::select! {
                _ = self.shutdown.recv() => {
                    self.send_node_status("disconnected").await;
                    break;
                }
                msg = receiver.next() => {
                    match msg {
                        Some(Ok(Message::Binary(data))) => {
                            if let Err(e) = self.handle_frame(data.to_vec()).await {
                                tracing::warn!("frame error (continuing): {e}");
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            break;
                        }
                        Some(Ok(Message::Ping(payload))) => {
                            let _ = self.sender.send(Message::Pong(payload)).await;
                        }
                        Some(Err(e)) => {
                            tracing::warn!("websocket error: {e}");
                            break;
                        }
                        None => break,
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_frame(&mut self, bytes: Vec<u8>) -> Result<()> {
        let header = Self::parse_frame_header(&bytes)?;
        let payload = Self::extract_payload(&bytes, header.payload_len)?;

        let result = match header.tag {
            0x01 => Self::handle_dial_peer(payload),
            0x02 => Box::pin(self.handle_append_event(payload)).await,
            0x03 => Box::pin(self.handle_get_events(payload)).await,
            0x04 => Box::pin(self.handle_get_events_paged(payload)).await,
            0x05 => Box::pin(self.handle_share_collection(payload)).await,
            0x06 => Box::pin(self.handle_import_collection(payload)).await,
            0x07 => Box::pin(self.handle_join_gossip_topic(payload, false)).await,
            0x08 => Box::pin(self.handle_join_gossip_topic(payload, true)).await,
            0x09 => Box::pin(self.handle_leave_gossip_topic(payload)).await,
            0x0A => Box::pin(self.handle_send_gossip_message(payload)).await,
            0x0B => Box::pin(self.handle_get_connected_peers(payload)).await,
            0x0C => self.handle_block_peer(payload),
            0x0D => self.handle_unblock_peer(payload),
            0x0E => Ok(self.handle_get_blocked_peers(payload)),
            0x0F => Ok(self.handle_get_node_id(payload)),
            _ => {
                let msg = format!("unknown tag: 0x{:02X}", header.tag);
                self.send_error(header.seq, &msg).await;
                return Err(SyncwebError::operation("unknown tag", format!("0x{:02X}", header.tag)));
            }
        };

        match result {
            Ok(response_bytes) => {
                self.send_ok(header.seq, &response_bytes).await?;
            }
            Err(e) => {
                self.send_error(header.seq, &e.to_string()).await;
            }
        }

        Ok(())
    }

    fn parse_frame_header(bytes: &[u8]) -> Result<FrameHeader> {
        if bytes.len() < FRAME_HEADER_LEN {
            return Err(SyncwebError::operation("frame parse error", "frame too short"));
        }
        let tag = *bytes
            .first()
            .ok_or_else(|| SyncwebError::operation("frame parse error", "empty frame"))?;
        let seq_arr: [u8; 4] = bytes
            .get(1..5)
            .ok_or_else(|| SyncwebError::operation("frame parse error", "missing seq"))?
            .try_into()
            .map_err(|error: std::array::TryFromSliceError| SyncwebError::operation("frame parse error", error))?;
        let len_arr: [u8; 4] = bytes
            .get(5..9)
            .ok_or_else(|| SyncwebError::operation("frame parse error", "missing payload_len"))?
            .try_into()
            .map_err(|error: std::array::TryFromSliceError| SyncwebError::operation("frame parse error", error))?;
        Ok(FrameHeader {
            tag,
            seq: u32::from_be_bytes(seq_arr),
            payload_len: u32::from_be_bytes(len_arr),
        })
    }

    fn extract_payload(bytes: &[u8], payload_len: u32) -> Result<&[u8]> {
        let len = usize::try_from(payload_len).unwrap_or(usize::MAX);
        let end = FRAME_HEADER_LEN.saturating_add(len);
        bytes
            .get(FRAME_HEADER_LEN..end)
            .ok_or_else(|| SyncwebError::operation("frame parse error", "truncated payload"))
    }

    fn handle_dial_peer(payload: &[u8]) -> Result<Vec<u8>> {
        let mut offset = 0;
        let node_id = encoding::read_string(payload, &mut offset)?;
        let _: iroh::PublicKey = node_id
            .parse()
            .map_err(|e| SyncwebError::operation("invalid peer id", e))?;
        Ok(Vec::new())
    }

    async fn handle_append_event(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut offset = 0;
        let collection_id = encoding::read_string(payload, &mut offset)?;
        let event_payload = encoding::read_bytes(payload, &mut offset)?;

        let namespace = self.service.get_or_create_collection(&collection_id).await?;
        let doc_opt = self.service.node().docs_engine().open(namespace).await?;
        let doc = doc_opt
            .ok_or_else(|| SyncwebError::operation("collection namespace not available", namespace.to_string()))?;
        let author = self.service.node().docs_engine().author().await?;

        let key = format!("evt/{}", uuid::Uuid::new_v4());
        self.service
            .node()
            .docs_engine()
            .set(&doc, author, &key, &event_payload)
            .await?;

        let node_id = self.service.node().endpoint().id().to_string();
        let mut response = Vec::new();
        encoding::write_string(&mut response, &node_id);
        encoding::write_u64(&mut response, 0);
        Ok(response)
    }

    async fn handle_get_events(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut offset = 0;
        let collection_id = encoding::read_string(payload, &mut offset)?;
        let entries = self.list_entries(&collection_id).await?;

        let node_id = self.service.node().endpoint().id().to_string();
        let mut response = Vec::new();
        let count = u32::try_from(entries.len()).unwrap_or(0);
        encoding::write_u32(&mut response, count);
        for content in entries {
            encoding::write_string(&mut response, &node_id);
            encoding::write_bytes(&mut response, &content);
            encoding::write_u64(&mut response, 0);
        }
        Ok(response)
    }

    async fn handle_get_events_paged(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut offset = 0;
        let collection_id = encoding::read_string(payload, &mut offset)?;
        let page_raw = encoding::read_u64(payload, &mut offset)?;
        let page_size_raw = encoding::read_u64(payload, &mut offset)?;
        let entries = self.list_entries(&collection_id).await?;

        let page_idx = usize::try_from(page_raw).unwrap_or(0);
        let limit = usize::try_from(page_size_raw).unwrap_or(0);
        let start = page_idx.saturating_sub(1).saturating_mul(limit);
        let paged: Vec<Vec<u8>> = entries.into_iter().skip(start).take(limit).collect();

        let node_id = self.service.node().endpoint().id().to_string();
        let mut response = Vec::new();
        let count = u32::try_from(paged.len()).unwrap_or(0);
        encoding::write_u32(&mut response, count);
        for content in paged {
            encoding::write_string(&mut response, &node_id);
            encoding::write_bytes(&mut response, &content);
            encoding::write_u64(&mut response, 0);
        }
        Ok(response)
    }

    async fn list_entries(&self, collection_id: &str) -> Result<Vec<Vec<u8>>> {
        let namespace = self.service.get_collection(collection_id).await?;
        let doc_opt = self.service.node().docs_engine().open(namespace).await?;
        let doc = doc_opt.ok_or_else(|| SyncwebError::operation("collection not found", collection_id.to_owned()))?;
        let entries = self.service.node().docs_engine().list_latest(&doc).await?;
        let mut result = Vec::with_capacity(entries.len());
        for entry in entries {
            let content = self.service.node().blob_store().get(entry.content_hash()).await?;
            result.push(content.to_vec());
        }
        Ok(result)
    }

    async fn handle_share_collection(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut offset = 0;
        let collection_id = encoding::read_string(payload, &mut offset)?;

        let namespace = self.service.get_collection(&collection_id).await?;
        let doc_opt = self.service.node().docs_engine().open(namespace).await?;
        let doc = doc_opt.ok_or_else(|| SyncwebError::operation("collection not found", collection_id))?;
        let ticket = self
            .service
            .node()
            .docs_engine()
            .share(
                &doc,
                iroh_docs::api::protocol::ShareMode::Read,
                self.service.node().endpoint().addr(),
            )
            .await?;

        let mut response = Vec::new();
        encoding::write_string(&mut response, &ticket.to_string());
        Ok(response)
    }

    async fn handle_import_collection(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut offset = 0;
        let collection_id = encoding::read_string(payload, &mut offset)?;
        let ticket_str = encoding::read_string(payload, &mut offset)?;

        let ticket: iroh_docs::DocTicket = ticket_str
            .parse()
            .map_err(|e| SyncwebError::operation("invalid doc ticket", e))?;
        let doc = self.service.node().docs_engine().import_ticket(ticket).await?;
        let namespace = self.service.node().docs_engine().namespace_id(&doc);
        self.service.node().docs_engine().start_sync(&doc, Vec::new()).await?;

        self.service.insert_collection(&collection_id, namespace).await;

        Ok(Vec::new())
    }

    async fn handle_join_gossip_topic(&self, payload: &[u8], with_discovery: bool) -> Result<Vec<u8>> {
        let mut offset = 0;
        let topic_str = encoding::read_string(payload, &mut offset)?;

        let _sender = self.service.subscribe_gossip(&topic_str, with_discovery).await?;
        Ok(Vec::new())
    }

    async fn handle_leave_gossip_topic(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut offset = 0;
        let topic_str = encoding::read_string(payload, &mut offset)?;

        self.service.leave_gossip(&topic_str).await;
        Ok(Vec::new())
    }

    async fn handle_send_gossip_message(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut offset = 0;
        let topic_str = encoding::read_string(payload, &mut offset)?;
        let message = encoding::read_bytes(payload, &mut offset)?;

        self.service.send_gossip(&topic_str, &message).await?;
        Ok(Vec::new())
    }

    async fn handle_get_connected_peers(&self, _payload: &[u8]) -> Result<Vec<u8>> {
        let peers = self
            .service
            .connected_peers
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let mut response = Vec::new();
        encoding::write_peer_list(&mut response, &peers);
        Ok(response)
    }

    fn handle_block_peer(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut offset = 0;
        let node_id = encoding::read_string(payload, &mut offset)?;
        self.service.block_peer(&node_id)?;
        Ok(Vec::new())
    }

    fn handle_unblock_peer(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut offset = 0;
        let node_id = encoding::read_string(payload, &mut offset)?;
        self.service.unblock_peer(&node_id)?;
        Ok(Vec::new())
    }

    fn handle_get_blocked_peers(&self, _payload: &[u8]) -> Vec<u8> {
        let blocked = self.service.get_blocked_peers();
        let mut response = Vec::new();
        encoding::write_string_list(&mut response, &blocked);
        response
    }

    fn handle_get_node_id(&self, _payload: &[u8]) -> Vec<u8> {
        let node_id = self.service.node().endpoint().id().to_string();
        let mut response = Vec::new();
        encoding::write_string(&mut response, &node_id);
        response
    }

    async fn send_ok(&mut self, seq: u32, payload: &[u8]) -> Result<()> {
        let frame = build_response(0x80, seq, payload);
        self.sender
            .send(Message::Binary(frame.into()))
            .await
            .map_err(|e| SyncwebError::operation("websocket send failed", e))
    }

    async fn send_error(&mut self, seq: u32, message: &str) {
        let mut payload = Vec::new();
        encoding::write_string(&mut payload, message);
        let frame = build_response(0x81, seq, &payload);
        if let Err(e) = self.sender.send(Message::Binary(frame.into())).await {
            tracing::warn!("failed to send error frame: {e}");
        }
    }

    async fn send_node_status(&mut self, status: &str) {
        let node_id = self.service.node().endpoint().id().to_string();
        let mut payload = Vec::new();
        encoding::write_string(&mut payload, status);
        encoding::write_string(&mut payload, &node_id);
        let frame = build_response(0x83, 0, &payload);
        let _ = self.sender.send(Message::Binary(frame.into())).await;
    }
}

struct FrameHeader {
    tag: u8,
    seq: u32,
    payload_len: u32,
}

fn build_response(tag: u8, seq: u32, payload: &[u8]) -> Vec<u8> {
    let payload_len = u32::try_from(payload.len()).unwrap_or(0);
    let len = FRAME_HEADER_LEN
        .checked_add(usize::try_from(payload_len).unwrap_or(0))
        .unwrap_or(FRAME_HEADER_LEN);
    let mut frame = Vec::with_capacity(len);
    frame.push(tag);
    frame.extend_from_slice(&seq.to_be_bytes());
    frame.extend_from_slice(&payload_len.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}
