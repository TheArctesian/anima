//! The open RDP session: the device's messages into the screen copy, frames to the client
//! as fast as it draws them, the client's input to the device, the clipboard under the
//! device's switches (RA-033), and the client's view as the device's screen size
//! (RA-004). I/O edge.

use std::time::Duration;

use ironrdp_pdu::Action;
use ironrdp_tokio::split_tokio_framed;
use tokio::io::{AsyncRead, AsyncWrite, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep_until};
use tokio_rustls::client::TlsStream;

use super::super::hub::{Hub, write_unless_stalled};
use super::super::input::{Clipboard, Input, copy_to_client};
use super::super::opening::{Allowed, refuse};
use super::super::picture::Picture;
use super::super::{Ended, Settings, status};
use super::Terms;
use crate::rdp::{Device, Event, Opened, Reply, Said, Sharing, ViewRequests, reply};

/// Messages read from the device that wait for the session. Small, so a device that
/// sends faster than the session draws is held back by TCP and not buffered.
const WAITING_MESSAGES: usize = 16;
struct Running<'a, S> {
    hub: &'a mut Hub<S>,
    to_device: WriteHalf<TlsStream<TcpStream>>,
    allowed: Allowed,
    write_wait: Duration,
    /// Reads the device; stopped when the session ends.
    reader: JoinHandle<()>,
    device: Device,
    picture: Picture,
    clipboard: Clipboard,
    /// The client's clipboard text, for the device when it pastes.
    text: Option<String>,
    /// Whether the device is asked for a screen of the size of the client's view.
    resizes: bool,
    view_requests: ViewRequests,
    started: Instant,
}

/// Runs the session until one side ends it.
pub(super) async fn run<S>(
    hub: &mut Hub<S>,
    opened: Opened,
    terms: Terms,
    settings: &Settings,
) -> Ended
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let Terms { allowed, resizes } = terms;
    // The size is checked before anything of that size is made.
    let size = opened.result.desktop_size;
    let size = (usize::from(size.width), usize::from(size.height));
    let Ok(picture) = Picture::new(size, &allowed, settings.pace) else {
        return refuse(hub, status::UPSTREAM_ERROR, Ended::DeviceFault).await;
    };
    let device = Device::new(opened.result);
    let (mut from_device, to_device) = split_tokio_framed(opened.connection);
    let (to_device, _) = to_device.into_inner();
    let (read, mut reads) = mpsc::channel::<(Action, Vec<u8>)>(WAITING_MESSAGES);
    let reader = tokio::spawn(async move {
        while let Ok((action, message)) = from_device.read_pdu().await {
            if read.send((action, message.to_vec())).await.is_err() {
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
        device,
        picture,
        clipboard: Clipboard::default(),
        text: None,
        resizes,
        view_requests: ViewRequests::default(),
        started: Instant::now(),
    };
    let locks = running.device.set_locks(&mut running.picture.screen);
    if let Err(ended) = running.follow(locks).await {
        return ended;
    }
    // The client learns the size of the screen at once, before the device draws.
    if let Err(ended) = running.picture.send_frame(running.hub).await {
        return ended;
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
    /// Waits for the client, the device, the time of a held-back frame or the time to ask
    /// for a new size, handles it, and sends a frame when one is due.
    async fn turn(&mut self, reads: &mut mpsc::Receiver<(Action, Vec<u8>)>) -> Result<(), Ended> {
        let frame_due = self.picture.due();
        let ask_due = self.view_requests.ask_at().map(|at| self.started + at);
        let or_now = |due: Option<Instant>| due.unwrap_or_else(Instant::now);
        tokio::select! {
            instruction = self.hub.next() => {
                let instruction = instruction.ok_or(Ended::ByHub)?;
                self.on_client(&instruction).await?;
            }
            message = reads.recv() => match message {
                Some((action, message)) => {
                    let events = self.device.receive(action, &message, &mut self.picture.screen);
                    self.follow(events).await?;
                }
                None => return Err(self.device_left().await),
            },
            () = sleep_until(or_now(frame_due)), if frame_due.is_some() => {}
            () = sleep_until(or_now(ask_due)), if ask_due.is_some() => self.ask_for_view().await?,
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

    /// Does what follows from a message of the device or from the client's input.
    async fn follow(
        &mut self,
        events: Result<Vec<Event>, crate::rdp::Malformed>,
    ) -> Result<(), Ended> {
        let Ok(events) = events else {
            return Err(refuse(self.hub, status::UPSTREAM_ERROR, Ended::DeviceFault).await);
        };
        for event in events {
            match event {
                Event::Send(message) => self.write(&message).await?,
                Event::Clipboard(said) => self.on_clipboard(said).await?,
                Event::Ended => return Err(self.device_left().await),
                // The picture follows the screen copy, which has the new size.
                Event::Resized => {}
            }
        }
        Ok(())
    }

    /// The clipboard under the device's switches (RA-033); `reply` holds the rule.
    async fn on_clipboard(&mut self, said: Said) -> Result<(), Ended> {
        let sharing = Sharing {
            copy: self.allowed.copy,
            paste: self.allowed.paste,
        };
        let message = match reply(said, sharing, self.text.as_deref()) {
            Reply::Offer(has_text) => self.device.offer_text(has_text),
            Reply::AskForText => self.device.ask_for_text(),
            Reply::GiveText(text) => self.device.give_text(text),
            Reply::ToClient(text) => {
                let stream = copy_to_client(&text);
                return self.hub.send(&stream).await.map_err(|_| Ended::ByHub);
            }
            Reply::Nothing => None,
        };
        match message {
            Some(message) => self.write(&message).await,
            None => Ok(()),
        }
    }

    async fn on_client(&mut self, instruction: &guac_protocol::Instruction) -> Result<(), Ended> {
        let screen = &mut self.picture.screen;
        match self.clipboard.read(instruction) {
            Input::Key { keysym, pressed } => {
                let events = self.device.key(keysym, pressed, screen);
                self.follow(events).await
            }
            Input::Pointer { x, y, buttons } => {
                let at = self.picture.to_device((x, y));
                let events = self.device.pointer(at, buttons, &mut self.picture.screen);
                self.follow(events).await
            }
            Input::View(width, height) => {
                self.picture.set_view((width, height));
                if self.resizes {
                    self.view_requests
                        .view_changed((width, height), self.started.elapsed());
                }
                Ok(())
            }
            // The device learns that there is new text, when it may have it. With paste
            // off it learns nothing, not even that the client's clipboard changed.
            Input::Paste(_) if !self.allowed.paste => Ok(()),
            Input::Paste(text) => {
                self.text = Some(text);
                self.on_clipboard(Said::WhatIsThere).await
            }
            Input::Drawn(timestamp) => {
                self.picture.drawn(&timestamp);
                Ok(())
            }
            Input::Disconnect => Err(Ended::ByHub),
            Input::Nothing => Ok(()),
        }
    }

    /// Asks the device for a screen of the size of the client's view, when one is due.
    async fn ask_for_view(&mut self) -> Result<(), Ended> {
        let screen = &self.picture.screen;
        let (screen, now) = ((screen.width(), screen.height()), self.started.elapsed());
        let Some(size) = self.view_requests.due(screen, now) else {
            return Ok(());
        };
        if let Some(message) = self.device.resize(size) {
            self.view_requests.asked();
            return self.write(&message).await;
        }
        self.view_requests.not_asked(now);
        Ok(())
    }
}
