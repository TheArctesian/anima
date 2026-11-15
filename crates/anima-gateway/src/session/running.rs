//! The open session: the device's updates into the screen copy, frames to the client as
//! fast as it draws them, and the client's input to the device. I/O edge.

use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep_until};

use super::hub::Hub;
use super::input::{Clipboard, Input, copy_to_client};
use super::opening::{Opened, refuse};
use super::{Ended, Settings, status};
use crate::frames::{self, Next, Pacing};
use crate::handshake::Opening;
use crate::screen::Screen;
use crate::view::View;
use crate::vnc::{self, Event, requests};

/// Bytes read from the device at a time.
const READ_BYTES: usize = 64 * 1024;
/// Reads from the device that wait for the session. Small, so a device that sends faster
/// than the session draws is held back by TCP and not buffered.
const WAITING_READS: usize = 4;

struct Running<'a, S> {
    hub: &'a mut Hub<S>,
    to_device: OwnedWriteHalf,
    opening: Opening,
    device: vnc::Device,
    screen: Screen,
    view: View,
    pacing: Pacing,
    clipboard: Clipboard,
    started: Instant,
}

/// Runs the session until one side ends it.
pub(super) async fn run<S>(hub: &mut Hub<S>, opened: Opened, settings: &Settings) -> Ended
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let Opened {
        opening,
        device,
        size,
    } = opened;
    let Ok(screen) = Screen::new(size.0, size.1) else {
        return refuse(hub, status::UPSTREAM_ERROR, Ended::DeviceFault).await;
    };
    let (mut from_device, to_device) = device.into_split();
    let (read, mut reads) = mpsc::channel::<Vec<u8>>(WAITING_READS);
    tokio::spawn(async move {
        let mut buffer = vec![0; READ_BYTES];
        while let Ok(count @ 1..) = from_device.read(&mut buffer).await {
            if read.send(buffer[..count].to_vec()).await.is_err() {
                return;
            }
        }
    });
    let view = View::new(&screen, opening.view);
    let mut running = Running {
        hub,
        to_device,
        opening,
        device: vnc::Device::default(),
        screen,
        view,
        pacing: Pacing::new(settings.pace),
        clipboard: Clipboard::default(),
        started: Instant::now(),
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

impl<S> Running<'_, S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    /// Waits for the client, the device or the time of a held-back frame, handles it, and
    /// sends a frame when one is due.
    async fn turn(&mut self, reads: &mut mpsc::Receiver<Vec<u8>>) -> Result<(), Ended> {
        // A frame held back by the frame rate goes out when its time comes. One held back
        // by the client goes out after the client's next `sync`, which is an instruction.
        let due = match self.pacing.next(self.started.elapsed()) {
            Next::At(time) if self.view.has_changes(&self.screen) => Some(self.started + time),
            _ => None,
        };
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
        self.send_frame().await
    }

    /// The device closed: the client is told, as guacd tells it.
    async fn device_left(&mut self) -> Ended {
        let disconnect = guac_protocol::Instruction::new("disconnect", Vec::<String>::new());
        let _ = self.hub.send(&[disconnect]).await;
        Ended::ByDevice
    }

    async fn write(&mut self, message: &[u8]) -> Result<(), Ended> {
        self.to_device
            .write_all(message)
            .await
            .map_err(|_| Ended::ByDevice)
    }

    async fn ask_for_update(&mut self, incremental: bool) -> Result<(), Ended> {
        let side = |side: usize| u16::try_from(side).unwrap_or(u16::MAX);
        let (width, height) = (side(self.screen.width()), side(self.screen.height()));
        self.write(&requests::update(incremental, width, height))
            .await
    }

    async fn on_device(&mut self, bytes: &[u8]) -> Result<(), Ended> {
        let Ok(events) = self.device.receive(bytes, &mut self.screen) else {
            return Err(refuse(self.hub, status::UPSTREAM_ERROR, Ended::DeviceFault).await);
        };
        for event in events {
            match event {
                Event::Updated => self.ask_for_update(true).await?,
                Event::Clipboard(text) if self.opening.copy => {
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
                let (x, y) = self.view.to_device((x, y), &self.screen);
                let on_wire = |at: usize| u16::try_from(at).unwrap_or(u16::MAX);
                self.write(&requests::pointer(buttons, on_wire(x), on_wire(y)))
                    .await
            }
            Input::View(width, height) => {
                self.view.set_client((width, height), &mut self.screen);
                Ok(())
            }
            Input::Paste(text) if self.opening.paste => {
                self.write(&requests::cut_text(&text)).await
            }
            Input::Drawn(timestamp) => {
                self.pacing.answered(&timestamp);
                Ok(())
            }
            Input::Disconnect => Err(Ended::ByHub),
            Input::Paste(_) | Input::Nothing => Ok(()),
        }
    }

    /// Sends a frame when the screen changed and one may go.
    async fn send_frame(&mut self) -> Result<(), Ended> {
        if !self.view.has_changes(&self.screen) {
            return Ok(());
        }
        let Some(timestamp) = self.pacing.send(self.started.elapsed()) else {
            return Ok(());
        };
        let (shown, changes) = self.view.take_changes(&mut self.screen);
        let Ok(frame) = frames::frame(shown, &changes, self.opening.formats, timestamp) else {
            return Err(refuse(self.hub, status::SERVER_ERROR, Ended::Fault).await);
        };
        self.hub.send(&frame).await.map_err(|_| Ended::ByHub)
    }
}
