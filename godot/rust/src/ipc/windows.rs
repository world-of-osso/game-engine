//! Local Windows named pipes, retaining IPC's little-endian MessagePack framing.

use std::{io, path::Path};

use serde::{Serialize, de::DeserializeOwned};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::{NamedPipeServer, ServerOptions},
    sync::Mutex,
};

// Match the peercred-ipc framed limit used by Unix diagnostics.
const MAX_MESSAGE_SIZE: usize = 64 * 1024 * 1024;

pub struct Server {
    path: String,
    listener: Mutex<NamedPipeServer>,
}

impl Server {
    pub fn bind(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "pipe name is not Unicode")
        })?;
        let listener = ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .create(path)?;
        Ok(Self {
            path: path.to_string(),
            listener: Mutex::new(listener),
        })
    }

    pub async fn accept(&self) -> io::Result<(Connection, ())> {
        let mut listener = self.listener.lock().await;
        listener.connect().await?;
        let next = ServerOptions::new()
            .reject_remote_clients(true)
            .create(&self.path)?;
        let stream = std::mem::replace(&mut *listener, next);
        Ok((Connection { stream }, ()))
    }
}

pub struct Connection {
    stream: NamedPipeServer,
}

impl Connection {
    pub async fn read<T: DeserializeOwned>(&mut self) -> io::Result<T> {
        let length = self.stream.read_u32_le().await? as usize;
        if length > MAX_MESSAGE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "message too large",
            ));
        }
        let mut bytes = vec![0; length];
        self.stream.read_exact(&mut bytes).await?;
        rmp_serde::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    pub async fn write<T: Serialize>(&mut self, message: &T) -> io::Result<()> {
        let bytes = rmp_serde::to_vec(message)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if bytes.len() > MAX_MESSAGE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "message too large",
            ));
        }
        self.stream.write_u32_le(bytes.len() as u32).await?;
        self.stream.write_all(&bytes).await
    }
}
