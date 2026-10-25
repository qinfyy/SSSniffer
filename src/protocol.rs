use anyhow::{Context, Result, bail, ensure};
use mlua::{ChunkMode, HookTriggers, Lua, LuaOptions, StdLib, Table, Value as LuaValue};
use prost::{
    Message,
    encoding::{
        DecodeContext, WireType, decode_key, decode_varint, encode_key, encode_varint, skip_field,
    },
};
use prost_reflect::{DescriptorPool, DynamicMessage, Kind, MessageDescriptor};
use prost_types::{FileDescriptorProto, FileDescriptorSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub cmdId: PathBuf,
    pub proto: PathBuf,
    pub port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cmdId: PathBuf::from(r".\data\NetMsgId.lua"),
            proto: PathBuf::from(r".\data\proto"),
            port: 1984,
        }
    }
}

impl Config {
    pub fn read(path: &Path) -> Result<Self> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                let config = Self::default();
                match fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)
                {
                    Ok(mut file) => {
                        serde_json::to_writer_pretty(&mut file, &config)?;
                        file.write_all(b"\n")?;
                        crate::DebugPrintLockln!(
                            "[INFO] Created default configuration: {}",
                            path.display()
                        );
                        return Ok(config);
                    }
                    Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                        fs::read_to_string(path)
                            .with_context(|| format!("Unable to read {}", path.display()))?
                    }
                    Err(error) => {
                        return Err(error)
                            .with_context(|| format!("Unable to create {}", path.display()));
                    }
                }
            }
            Err(error) => {
                return Err(error).with_context(|| format!("Unable to read {}", path.display()));
            }
        };
        let config: Self = serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .context("Invalid SSSniffer.json")?;
        ensure!(config.port != 0, "port must be between 1 and 65535");
        Ok(config)
    }
}

pub struct Protocol {
    names: HashMap<i16, String>,
    messages: HashMap<i16, MessageDescriptor>,
}

impl Protocol {
    pub fn load(config: &Config, base: &Path) -> Result<Self> {
        ensure!(
            !config.cmdId.as_os_str().is_empty() && !config.proto.as_os_str().is_empty(),
            "cmdId and proto must not be empty"
        );
        let cmdId = base.join(&config.cmdId);
        let proto = base.join(&config.proto);
        let (names, types) = load_commands(&cmdId)?;
        let pool = if proto.is_dir() {
            compile_directory(&proto)?
        } else {
            load_pb(&proto)?
        };
        let mut messages = HashMap::with_capacity(names.len());
        for (id, name) in &names {
            let type_name = types
                .get(id)
                .with_context(|| format!("Missing MsgName entry for {id} ({name})"))?;
            let descriptor = pool
                .get_message_by_name(type_name)
                .or_else(|| {
                    type_name
                        .strip_prefix("proto.")
                        .and_then(|name| pool.get_message_by_name(name))
                })
                .with_context(|| {
                    format!("Descriptor not found: {type_name}, command {id} ({name})")
                })?;
            messages.insert(*id, descriptor);
        }
        crate::DebugPrintLockln!(
            "[INFO] Loaded {} command IDs and {} proto descriptors",
            names.len(),
            pool.files().count()
        );
        Ok(Self { names, messages })
    }

    pub fn name(&self, id: i16) -> &str {
        self.names
            .get(&id)
            .map(String::as_str)
            .unwrap_or("UnknownPacket")
    }

    pub fn decode(&self, id: i16, payload: &[u8]) -> Result<Value> {
        let descriptor = self
            .messages
            .get(&id)
            .with_context(|| format!("Unknown packet ID: {id}"))?;
        let message = match DynamicMessage::decode(descriptor.clone(), payload) {
            Ok(message) => message,
            Err(original) => {
                let mut input = payload;
                let mut skipped = 0;
                let compatible = compatible_payload(descriptor, &mut input, None, 0, &mut skipped)
                    .with_context(|| {
                        format!(
                            "Unable to decode {} ({id}): {original}",
                            descriptor.full_name()
                        )
                    })?;
                ensure!(
                    skipped > 0,
                    "Unable to decode {} ({id}): {original}",
                    descriptor.full_name()
                );
                let message = DynamicMessage::decode(descriptor.clone(), compatible.as_slice())
                    .with_context(|| {
                        format!("Unable to decode {} ({id})", descriptor.full_name())
                    })?;
                crate::DebugPrintLockln!(
                    "[WARN] Decoded {} ({id}): Skipped {skipped} fields due to wire type mismatches; please update the proto definition to decode these fields.",
                    descriptor.full_name()
                );
                message
            }
        };
        serde_json::to_value(message).context("Unable to serialize protobuf to JSON")
    }
}

fn compatible_payload(
    descriptor: &MessageDescriptor,
    input: &mut &[u8],
    end_group: Option<u32>,
    depth: usize,
    skipped: &mut usize,
) -> Result<Vec<u8>> {
    ensure!(depth < 100, "Protobuf recursion limit reached");
    let mut output = Vec::new();
    while !input.is_empty() {
        let start = *input;
        let (number, wire) = decode_key(input)?;
        if wire == WireType::EndGroup {
            ensure!(end_group == Some(number), "Unexpected protobuf end group");
            return Ok(output);
        }
        let field = descriptor
            .get_field(number)
            .map(|field| (field.kind(), field.is_list(), field.is_group()))
            .or_else(|| {
                descriptor
                    .get_extension(number)
                    .map(|field| (field.kind(), field.is_list(), field.is_group()))
            });
        if let Some((kind, list, group)) = field {
            let expected = match &kind {
                Kind::Double | Kind::Fixed64 | Kind::Sfixed64 => WireType::SixtyFourBit,
                Kind::Float | Kind::Fixed32 | Kind::Sfixed32 => WireType::ThirtyTwoBit,
                Kind::Message(_) if group => WireType::StartGroup,
                Kind::String | Kind::Bytes | Kind::Message(_) => WireType::LengthDelimited,
                _ => WireType::Varint,
            };
            let packed = list
                && wire == WireType::LengthDelimited
                && matches!(
                    expected,
                    WireType::Varint | WireType::ThirtyTwoBit | WireType::SixtyFourBit
                );
            if wire != expected && !packed {
                skip_field(wire, number, input, DecodeContext::default())?;
                *skipped += 1;
                continue;
            }
            if let Kind::Message(nested) = kind {
                let value = if group {
                    compatible_payload(&nested, input, Some(number), depth + 1, skipped)?
                } else {
                    let length = decode_varint(input)?;
                    ensure!(length <= input.len() as u64, "Truncated protobuf message");
                    let (mut value, rest) = input.split_at(length as usize);
                    *input = rest;
                    compatible_payload(&nested, &mut value, None, depth + 1, skipped)?
                };
                encode_key(number, wire, &mut output);
                if !group {
                    encode_varint(value.len() as u64, &mut output);
                }
                output.extend_from_slice(&value);
                if group {
                    encode_key(number, WireType::EndGroup, &mut output);
                }
                continue;
            }
        }
        skip_field(wire, number, input, DecodeContext::default())?;
        output.extend_from_slice(&start[..start.len() - input.len()]);
    }
    ensure!(end_group.is_none(), "Unterminated protobuf group");
    Ok(output)
}

fn load_commands(path: &Path) -> Result<(HashMap<i16, String>, HashMap<i16, String>)> {
    let bytes = fs::read(path).with_context(|| format!("Unable to read {}", path.display()))?;
    let (source, mode, lua) = if bytes.starts_with(b"\x1bLua") {
        let version = *bytes.get(4).context("Truncated Lua bytecode header")?;
        ensure!(
            version == 0x53,
            "Unsupported Lua bytecode version 0x{version:02X}; expected Lua 5.3 (0x53)"
        );
        // Binary chunks must come from trusted files; mlua's safe state rejects bytecode.
        let lua = unsafe { Lua::unsafe_new_with(StdLib::NONE, LuaOptions::default()) };
        (bytes.as_slice(), ChunkMode::Binary, lua)
    } else {
        let source = std::str::from_utf8(&bytes).context("NetMsgId.lua text must be UTF-8")?;
        (
            source.trim_start_matches('\u{feff}').as_bytes(),
            ChunkMode::Text,
            Lua::new_with(StdLib::NONE, LuaOptions::default())?,
        )
    };
    lua.set_memory_limit(16 * 1024 * 1024)?;
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(1_000_000),
        |_, _| {
            Err(mlua::Error::RuntimeError(
                "NetMsgId.lua exceeded its instruction budget".into(),
            ))
        },
    )?;
    let table: Table = lua
        .load(source)
        .set_mode(mode)
        .set_name(path.to_string_lossy())
        .set_environment(lua.create_table()?)
        .eval()
        .with_context(|| format!("{} must return a NetMsgId table", path.display()))?;
    let ids: Table = table.get("Id").context("Missing NetMsgId.Id table")?;
    let types: Table = table
        .get("MsgName")
        .context("Missing NetMsgId.MsgName table")?;
    let mut names = HashMap::new();
    let mut messages = HashMap::new();
    for pair in ids.pairs::<LuaValue, LuaValue>() {
        let (name, id) = pair?;
        let id = command_id(id)?;
        ensure!(
            names.insert(id, lua_string(name)?).is_none(),
            "Duplicate command ID: {id}"
        );
    }
    for pair in types.pairs::<LuaValue, LuaValue>() {
        let (id, name) = pair?;
        messages.insert(command_id(id)?, lua_string(name)?);
    }
    ensure!(
        !names.is_empty() && !messages.is_empty(),
        "NetMsgId tables must not be empty"
    );
    Ok((names, messages))
}

fn command_id(value: LuaValue) -> Result<i16> {
    let value = match value {
        LuaValue::Integer(value) => value,
        LuaValue::Number(value)
            if value.is_finite()
                && value.fract() == 0.0
                && value >= i16::MIN as f64
                && value <= i16::MAX as f64 =>
        {
            value as i64
        }
        _ => bail!("Invalid command ID in NetMsgId.lua"),
    };
    i16::try_from(value).context("Command ID is outside int16 range")
}

fn lua_string(value: LuaValue) -> Result<String> {
    match value {
        LuaValue::String(value) => {
            let value = value.to_str()?.to_string();
            ensure!(!value.is_empty(), "Empty name in NetMsgId.lua");
            Ok(value)
        }
        _ => bail!("Invalid name in NetMsgId.lua"),
    }
}

fn compile_directory(path: &Path) -> Result<DescriptorPool> {
    let path = fs::canonicalize(path)?;
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(&path) {
        let entry = entry?;
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "proto")
        {
            files.push(entry.into_path());
        }
    }
    ensure!(
        !files.is_empty(),
        "No .proto files found in {}",
        path.display()
    );
    files.sort();
    let descriptors = protox::Compiler::new([&path])?
        .include_imports(true)
        .include_source_info(false)
        .open_files(&files)
        .context("Unable to compile .proto directory")?
        .file_descriptor_set();
    crate::DebugPrintLockln!("[INFO] Protox compiled {} .proto files", files.len());
    DescriptorPool::from_file_descriptor_set(descriptors).context("Invalid compiled descriptors")
}

fn load_pb(path: &Path) -> Result<DescriptorPool> {
    ensure!(
        path.extension().is_some_and(|extension| extension == "pb"),
        "proto must point to a .proto directory or a .pb file"
    );
    let bytes = fs::read(path).with_context(|| format!("Unable to read {}", path.display()))?;
    ensure!(!bytes.is_empty(), "Empty .pb file");
    let mut pool = DescriptorPool::global();
    match FileDescriptorSet::decode(bytes.as_slice()) {
        Ok(set) if !set.file.is_empty() => {
            let mut names = std::collections::HashSet::new();
            for file in &set.file {
                ensure!(
                    !file.name().is_empty() && names.insert(file.name()),
                    "Empty or duplicate descriptor filename"
                );
            }
            pool.decode_file_descriptor_set(bytes.as_slice())
                .context("Invalid .pb descriptor set or missing imports")?;
        }
        _ => {
            let file = FileDescriptorProto::decode(bytes.as_slice())
                .context("Invalid .pb descriptor file")?;
            ensure!(!file.name().is_empty(), "Missing descriptor filename");
            pool.decode_file_descriptor_proto(bytes.as_slice())
                .context("Invalid single .pb descriptor or missing imports")?;
        }
    }
    Ok(pool)
}
