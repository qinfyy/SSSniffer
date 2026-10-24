use anyhow::{Context, Result, bail, ensure};
use mlua::{ChunkMode, HookTriggers, Lua, LuaOptions, StdLib, Table, Value as LuaValue};
use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage, MessageDescriptor};
use prost_types::{FileDescriptorProto, FileDescriptorSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
    pub cmdid: PathBuf,
    pub proto: PathBuf,
}

impl Config {
    pub fn read(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("Unable to read {}", path.display()))?;
        serde_json::from_str(text.trim_start_matches('\u{feff}')).context("Invalid SSSniffer.json")
    }
}

pub struct Protocol {
    names: HashMap<i32, String>,
    messages: HashMap<i32, MessageDescriptor>,
}

impl Protocol {
    pub fn load(config: &Config, base: &Path) -> Result<Self> {
        ensure!(
            !config.cmdid.as_os_str().is_empty() && !config.proto.as_os_str().is_empty(),
            "cmdid and proto must not be empty"
        );
        let cmdid = base.join(&config.cmdid);
        let proto = base.join(&config.proto);
        let (names, types) = load_commands(&cmdid)?;
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

    pub fn name(&self, id: i32) -> &str {
        self.names
            .get(&id)
            .map(String::as_str)
            .unwrap_or("UnknownPacket")
    }

    pub fn decode(&self, id: i32, payload: &[u8]) -> Result<Value> {
        let descriptor = self
            .messages
            .get(&id)
            .with_context(|| format!("Unknown packet ID: {id}"))?;
        let message = DynamicMessage::decode(descriptor.clone(), payload)
            .with_context(|| format!("Unable to decode {} ({id})", descriptor.full_name()))?;
        serde_json::to_value(message).context("Unable to serialize protobuf to JSON")
    }
}

fn load_commands(path: &Path) -> Result<(HashMap<i32, String>, HashMap<i32, String>)> {
    let source =
        fs::read_to_string(path).with_context(|| format!("Unable to read {}", path.display()))?;
    let lua = Lua::new_with(StdLib::NONE, LuaOptions::default())?;
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
        .load(source.trim_start_matches('\u{feff}'))
        .set_mode(ChunkMode::Text)
        .set_name(path.to_string_lossy())
        .set_environment(lua.create_table()?)
        .eval()
        .context("NetMsgId.lua must return a table")?;
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

fn command_id(value: LuaValue) -> Result<i32> {
    let value = match value {
        LuaValue::Integer(value) => value,
        LuaValue::Number(value)
            if value.is_finite()
                && value.fract() == 0.0
                && value >= i32::MIN as f64
                && value <= i32::MAX as f64 =>
        {
            value as i64
        }
        _ => bail!("Invalid command ID in NetMsgId.lua"),
    };
    i32::try_from(value).context("Command ID is outside int32 range")
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
