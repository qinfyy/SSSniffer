use crate::{DebugPrintLockln, server::AppState};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, SyncSender},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use windows::Win32::System::SystemInformation::GetLocalTime;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Packet {
    pub from_server: bool,
    #[serde(default)]
    pub object: Option<Value>,
    pub packet_id: i16,
    pub packet_name: String,
    pub raw: String,
    pub time: u64,
}

struct RawPacket {
    time: u64,
    id: i16,
    from_server: bool,
    data: Vec<u8>,
}
static QUEUE: OnceLock<SyncSender<RawPacket>> = OnceLock::new();
pub static DROPPED: AtomicU64 = AtomicU64::new(0);

pub fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn enqueue(id: i16, from_server: bool, data: &[u8]) {
    if let Some(queue) = QUEUE.get() {
        let packet = RawPacket {
            time: timestamp(),
            id,
            from_server,
            data: data.to_vec(),
        };
        if queue.try_send(packet).is_err() {
            DROPPED.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub fn start(state: Arc<AppState>) -> Result<()> {
    let (sender, receiver) = mpsc::sync_channel::<RawPacket>(2048);
    QUEUE
        .set(sender)
        .map_err(|_| anyhow::anyhow!("Capture queue already initialized"))?;
    std::thread::Builder::new()
        .name("sssniffer-decode".into())
        .spawn(move || {
            let mut log: Option<BufWriter<File>> = None;
            let mut log_failed = false;
            for raw in receiver {
                let name = state.protocol.name(raw.id);
                let object = if raw.data.is_empty() {
                    None
                } else {
                    match state.protocol.decode(raw.id, &raw.data) {
                        Ok(object) => Some(object),
                        Err(error) => {
                            DebugPrintLockln!("[WARN] {error:#}");
                            None
                        }
                    }
                };
                let packet = Packet {
                    time: raw.time,
                    from_server: raw.from_server,
                    packet_id: raw.id,
                    packet_name: name.to_owned(),
                    object,
                    raw: rbase64::encode(&raw.data),
                };
                if !log_failed {
                    if log.is_none() {
                        match open_log(&state) {
                            Ok(file) => log = Some(BufWriter::new(file)),
                            Err(error) => {
                                DebugPrintLockln!("[ERROR] {error:#}");
                                log_failed = true;
                            }
                        }
                    }
                    if let Some(writer) = &mut log {
                        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
                        let result = packet
                            .serialize(&mut serde_json::Serializer::with_formatter(
                                &mut *writer,
                                formatter,
                            ))
                            .map_err(std::io::Error::other)
                            .and_then(|_| writer.write_all(b"\n"))
                            .and_then(|_| writer.flush());
                        if let Err(error) = result {
                            DebugPrintLockln!("[ERROR] Capture log write failed: {error}");
                            log_failed = true;
                        }
                    }
                }
                state.record(Arc::new(packet));
            }
        })
        .context("Unable to start capture decoder")?;
    Ok(())
}

fn open_log(state: &AppState) -> Result<File> {
    let local = unsafe { GetLocalTime() };
    let path = state.directory.join(format!(
        "PacketCapture_{:02}-{:02}-{:02}_{:02}-{:02}-{:02}.json",
        local.wYear % 100,
        local.wMonth,
        local.wDay,
        local.wHour,
        local.wMinute,
        local.wSecond
    ));
    File::create(&path).with_context(|| format!("Unable to create {}", path.display()))
}

pub fn import(bytes: &[u8]) -> Result<Vec<Packet>> {
    let content = std::str::from_utf8(bytes).context("Invalid capture text")?;
    let mut packets = Vec::new();
    let mut block = String::new();
    let mut in_block = false;
    // Match the old ReadJsonBlocks parser: only column-zero braces delimit records.
    for line in content.trim_start_matches('\u{feff}').lines() {
        if line.is_empty() {
            continue;
        }
        if line.starts_with('{') {
            in_block = true;
            block.clear();
        }
        if in_block {
            block.push_str(line);
            block.push('\n');
        }
        if line.starts_with('}') && in_block {
            in_block = false;
            let packet: Packet = serde_json::from_str(&block).context("Invalid capture block")?;
            rbase64::decode(&packet.raw).context("Invalid base64 packet data")?;
            packets.push(packet);
        }
    }
    Ok(packets)
}
