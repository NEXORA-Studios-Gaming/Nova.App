use crate::{Error, Result, decode, encode};
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

const VERSION: u8 = 2;
#[derive(Debug, Serialize, Deserialize)]
struct Envelope {
    version: u8,
    message: Message,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum Message {
    Subscribe {
        id: u64,
        topic: String,
    },
    Unsubscribe {
        id: u64,
        topic: String,
    },
    Publish {
        id: u64,
        topic: String,
        payload: Vec<u8>,
    },
    Request {
        id: u64,
        method: String,
        payload: Vec<u8>,
    },
    RegisterService {
        id: u64,
        name: String,
        methods: Vec<String>,
    },
    Invoke {
        id: u64,
        method: String,
        payload: Vec<u8>,
    },
    ServiceReply {
        id: u64,
        result: std::result::Result<Vec<u8>, String>,
    },
    Reply {
        id: u64,
        result: std::result::Result<Vec<u8>, String>,
    },
    Event {
        topic: String,
        payload: Vec<u8>,
    },
}
type Framing = Framed<TcpStream, LengthDelimitedCodec>;
pub(crate) type Writer = SplitSink<Framing, bytes::Bytes>;
pub(crate) type Reader = SplitStream<Framing>;
pub(crate) fn split(stream: TcpStream, max: usize) -> (Writer, Reader) {
    LengthDelimitedCodec::builder()
        .max_frame_length(max)
        .new_framed(stream)
        .split()
}
pub(crate) async fn send(writer: &mut Writer, message: Message, max: usize) -> Result<()> {
    let bytes = encode(&Envelope {
        version: VERSION,
        message,
    })?;
    if bytes.len() > max {
        return Err(Error::Protocol(
            "encoded frame exceeds max_frame_bytes".into(),
        ));
    }
    writer.send(bytes.into()).await?;
    Ok(())
}
pub(crate) async fn receive(reader: &mut Reader) -> Result<Message> {
    let bytes = reader.next().await.ok_or(Error::Disconnected)??;
    let envelope: Envelope = decode(&bytes)?;
    if envelope.version != VERSION {
        return Err(Error::Protocol("unsupported protocol version".into()));
    }
    Ok(envelope.message)
}
pub(crate) fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 256 {
        return Err(Error::Protocol(
            "topic/method must contain 1..=256 UTF-8 bytes".into(),
        ));
    }
    Ok(())
}
