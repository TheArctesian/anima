//! The open session: the device's updates into the screen copy, frames to the client as
//! fast as it draws them, and the client's input to the device. I/O edge.

use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep_until};

use super::hub::{Hub, write_unless_stalled};
use super::input::{Clipboard, Input, copy_to_client};
use super::opening::{Allowed, Opened, refuse};
use super::picture::Picture;
use super::{Ended, Settings, status};
use crate::vnc::{self, Event, requests};

/// Bytes read from the device at a time.
const READ_BYTES: usize = 64 * 1024;
/// Reads from the device that wait for the session. Small, so a device that sends faster
/// than the session draws is held back by TCP and not buffered.
const WAITING_READS: usize = 4;

struct Running<'a, S> {
    hub: &'a mut Hub<S>,
    to_device: OwnedWriteHalf,
    allowed: Allowed,
    write_wait: std::time::Duration,
    /// Reads the device; stopped when the session ends.
    reader: JoinHandle<()>,
    device: vnc::Device,
    picture: Picture,
    clipboard: Clipboard,
    /// The device's screen has a new size: the next request asks for all of it.
    resized: bool,
}

/// Runs the session until one side ends it.
pub(super) async fn run<S>(hub: &mut Hub<S>, opened: Opened, settings: &Settings) -> Ended
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let Opened {
        allowed,
        device,
        size,
    } = opened;
    let Ok(picture) = Picture::new(size, &allowed, settings.pace) else {
        return refuse(hub, status::UPSTREAM_ERROR, Ended::DeviceFault).await;
    };
    let (mut from_device, to_device) = device.into_split();
    let (read, mut reads) = mpsc::channel::<Vec<u8>>(WAITING_READS);
    let reader = tokio::spawn(async move {
        let mut buffer = vec![0; READ_BYTES];
        while let Ok(count @ 1..) = from_device.read(&mut buffer).await {
            if read.send(buffer[..count].to_vec()).await.is_err() {
                return;
            }
        }
    });
    let mut running = Running {
        hub,
        to_device,
        allowed,
        write_wait: settings.write_wait,
        reader,
        device: vnc::Device::default(),
        picture,
        clipboard: Clipboard::default(),
        resized: false,
    };
    if running.ask_for_update(false).await.is_err() {
        return Ended::ByDevice;
    }
    loop {
        if let Err(ended) = running.turn(&mut reads).await {
            return ended;
        }
    }
}

impl<S> Drop for Running<'_, S> {
    fn drop(&mut self) {
        self.reader.abort();
    }
}

impl<S> Running<'_, S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    /// Waits for the client, the device or the time of a held-back frame, handles it, and
    /// sends a frame when one is due.
    async fn turn(&mut self, reads: &mut mpsc::Receiver<Vec<u8>>) -> Result<(), Ended> {
        let due = self.picture.due();
        tokio::select! {
            instruction = self.hub.next() => {
                let instruction = instruction.ok_or(Ended::ByHub)?;
                self.on_client(&instruction).await?;
            }
            bytes = reads.recv() => match bytes {
                Some(bytes) => self.on_device(&bytes).await?,
                None => return Err(self.device_left().await),
            },
            () = sleep_until(due.unwrap_or_else(Instant::now)), if due.is_some() => {}
        }
        self.picture.send_frame(self.hub).await
    }

    /// The device closed: the client is told, as guacd tells it.
    async fn device_left(&mut self) -> Ended {
        let disconnect = guac_protocol::Instruction::new("disconnect", Vec::<String>::new());
        let _ = self.hub.send(&[disconnect]).await;
        Ended::ByDevice
    }

    /// Writes to the device. A device that takes no byte within the write limit has stopped
    /// reading: the session ends, so it cannot be held open.
    async fn write(&mut self, message: &[u8]) -> Result<(), Ended> {
        write_unless_stalled(&mut self.to_device, message, self.write_wait)
            .await
            .map_err(|_| Ended::ByDevice)
    }

    async fn ask_for_update(&mut self, incremental: bool) -> Result<(), Ended> {
        let side = |side: usize| u16::try_from(side).unwrap_or(u16::MAX);
        let screen = &self.picture.screen;
        let (width, height) = (side(screen.width()), side(screen.height()));
        self.write(&requests::update(incremental, width, height))
            .await
    }

    async fn on_device(&mut self, bytes: &[u8]) -> Result<(), Ended> {
        let Ok(events) = self.device.receive(bytes, &mut self.picture.screen) else {
            return Err(refuse(self.hub, status::UPSTREAM_ERROR, Ended::DeviceFault).await);
        };
        for event in events {
            match event {
                Event::Resized => self.resized = true,
                Event::Updated => {
                    let all = std::mem::take(&mut self.resized);
                    self.ask_for_update(!all).await?;
                }
                Event::Clipboard(text) if self.allowed.copy => {
                    self.hub
                        .send(&copy_to_client(&text))
                        .await
                        .map_err(|_| Ended::ByHub)?;
                }
                Event::Clipboard(_) | Event::Bell => {}
            }
        }
        Ok(())
    }

    async fn on_client(&mut self, instruction: &guac_protocol::Instruction) -> Result<(), Ended> {
        match self.clipboard.read(instruction) {
            Input::Key { keysym, pressed } => self.write(&requests::key(keysym, pressed)).await,
            Input::Pointer { x, y, buttons } => {
                let (x, y) = self.picture.to_device((x, y));
                self.write(&requests::pointer(buttons, x, y)).await
            }
            Input::View(width, height) => {
                self.picture.set_view((width, height));
                Ok(())
            }
            Input::Paste(text) if self.allowed.paste => {
                self.write(&requests::cut_text(&text)).await
            }
            Input::Drawn(timestamp) => {
                self.picture.drawn(&timestamp);
                Ok(())
            }
            Input::Disconnect => Err(Ended::ByHub),
            Input::Paste(_) | Input::Nothing => Ok(()),
        }
    }
}
