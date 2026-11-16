//! One signed-in RDP device: its messages into the screen copy, and the client's input
//! and clipboard as messages for it. `IronRDP` decodes and encodes; this file joins it to
//! the gateway's screen, keys, pointer and clipboard.
//!
//! No I/O: the session reads the device's messages and writes what this gives back.

use ironrdp_cliprdr::CliprdrClient;
use ironrdp_cliprdr::pdu::{ClipboardFormat, FormatDataResponse};
use ironrdp_connector::ConnectionResult;
use ironrdp_connector::connection_activation::ConnectionActivationSequence;
use ironrdp_core::IntoOwned as _;
use ironrdp_displaycontrol::pdu::MonitorLayoutEntry;
use ironrdp_dvc::DrdynvcClient;
use ironrdp_graphics::image_processing::PixelFormat;
use ironrdp_pdu::Action;
use ironrdp_pdu::input::fast_path::FastPathInputEvent;
use ironrdp_session::image::DecodedImage;
use ironrdp_session::{ActiveStage, ActiveStageBuilder, ActiveStageOutput};

use super::channels::ChannelGuard;
use super::clipboard::{Clipboard, Said, TEXT};
use super::keys::Keyboard;
use super::pointer::Pointer;
use super::{drawing, input, resizing};
use crate::screen::Screen;

/// The device sent what an RDP server may not send, or what the gateway cannot use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the device broke the RDP protocol")]
pub struct Malformed;

/// What a message of the device means for the session.
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    /// Write these bytes to the device.
    Send(Vec<u8>),
    /// The screen copy has a new size.
    Resized,
    /// The device said this on the clipboard channel.
    Clipboard(Said),
    /// The device ended the session.
    Ended,
}

/// The device's session, after the opening.
pub struct Device {
    pub(super) stage: ActiveStage,
    /// `IronRDP`'s picture of the screen, which it decodes into.
    pub(super) image: DecodedImage,
    activation: ironrdp_connector::connection_activation::ConnectionActivationFactory,
    /// The exchange of capabilities after the device changed its screen, while it runs.
    pub(super) reactivating: Option<ConnectionActivationSequence>,
    keyboard: Keyboard,
    pointer: Pointer,
    guard: ChannelGuard,
}

impl Device {
    /// The device from the result of its opening. The caller has checked the size of its
    /// screen.
    #[must_use]
    pub fn new(opened: ConnectionResult) -> Self {
        let size = opened.desktop_size;
        let channels = &opened.static_channels;
        let guard = ChannelGuard::new(
            channels
                .get_channel_id_by_type::<CliprdrClient>()
                .into_iter()
                .chain(channels.get_channel_id_by_type::<DrdynvcClient>()),
        );
        let stage = ActiveStageBuilder {
            static_channels: opened.static_channels,
            user_channel_id: opened.user_channel_id,
            io_channel_id: opened.io_channel_id,
            message_channel_id: opened.message_channel_id,
            share_id: opened.share_id,
            compression_type: opened.compression_type,
            enable_server_pointer: opened.enable_server_pointer,
            pointer_software_rendering: opened.pointer_software_rendering,
        }
        .build();
        Self {
            stage,
            image: DecodedImage::new(PixelFormat::RgbA32, size.width, size.height),
            activation: opened.activation_factory,
            reactivating: None,
            keyboard: Keyboard::default(),
            pointer: Pointer::default(),
            guard,
        }
    }

    /// Takes one message of the device, draws what it shows into `screen`, and gives what
    /// follows from it, in order.
    ///
    /// # Errors
    /// [`Malformed`]: end the session.
    pub fn receive(
        &mut self,
        action: Action,
        message: &[u8],
        screen: &mut Screen,
    ) -> Result<Vec<Event>, Malformed> {
        if action == Action::X224 {
            self.guard.check(message)?;
        }
        if self.reactivating.is_some() {
            return resizing::go_on(self, Some((action, message)), screen);
        }
        let outputs = self
            .stage
            .process(&mut self.image, action, message, None)
            .map_err(|_| Malformed)?;
        self.follow(outputs, screen)
    }

    /// What `IronRDP` made of a message or of the client's input.
    fn follow(
        &mut self,
        outputs: Vec<ActiveStageOutput>,
        screen: &mut Screen,
    ) -> Result<Vec<Event>, Malformed> {
        let mut events = Vec::new();
        for output in outputs {
            match output {
                ActiveStageOutput::ResponseFrame(bytes) => events.push(Event::Send(bytes)),
                ActiveStageOutput::GraphicsUpdate(area) => {
                    drawing::draw(&self.image, &area, screen)?;
                }
                // A device that sends the session to another host: the hub checked this one
                // only, so the session ends.
                ActiveStageOutput::Terminate(_) | ActiveStageOutput::ServerRedirection(_) => {
                    events.push(Event::Ended);
                }
                ActiveStageOutput::DeactivateAll => {
                    self.reactivating = Some(self.activation.create());
                    events.extend(resizing::go_on(self, None, screen)?);
                }
                _ => {}
            }
        }
        if let Some(clipboard) = self.clipboard() {
            events.extend(clipboard.take().into_iter().map(Event::Clipboard));
        }
        Ok(events)
    }

    fn clipboard(&mut self) -> Option<&mut Clipboard> {
        self.stage
            .get_svc_processor_mut::<CliprdrClient>()?
            .downcast_backend_mut::<Clipboard>()
    }

    fn send_input(
        &mut self,
        events: &[FastPathInputEvent],
        screen: &mut Screen,
    ) -> Result<Vec<Event>, Malformed> {
        // The device is not listening while it exchanges capabilities again.
        if self.reactivating.is_some() {
            return Ok(Vec::new());
        }
        let outputs = self
            .stage
            .process_fastpath_input(&mut self.image, events)
            .map_err(|_| Malformed)?;
        self.follow(outputs, screen)
    }

    /// Sets the device's lock keys as the key table needs them: once, at the start.
    ///
    /// # Errors
    /// [`Malformed`]: end the session.
    pub fn set_locks(&mut self, screen: &mut Screen) -> Result<Vec<Event>, Malformed> {
        self.send_input(&[input::locks()], screen)
    }

    /// The client's `key` (K4 Section 4).
    ///
    /// # Errors
    /// [`Malformed`]: end the session.
    pub fn key(
        &mut self,
        keysym: u32,
        pressed: bool,
        screen: &mut Screen,
    ) -> Result<Vec<Event>, Malformed> {
        let events: Vec<_> = self
            .keyboard
            .key(keysym, pressed)
            .into_iter()
            .map(input::key)
            .collect();
        self.send_input(&events, screen)
    }

    /// The client's `mouse`, at a position on the device's screen. The pointer the device
    /// draws moves in `screen`.
    ///
    /// # Errors
    /// [`Malformed`]: end the session.
    pub fn pointer(
        &mut self,
        at: (u16, u16),
        buttons: u8,
        screen: &mut Screen,
    ) -> Result<Vec<Event>, Malformed> {
        let events: Vec<_> = self
            .pointer
            .moved(buttons)
            .into_iter()
            .map(|event| input::pointer(event, at))
            .collect();
        self.send_input(&events, screen)
    }

    /// Asks the device for a screen of this size, the client's view. `None` when the
    /// device cannot be asked now: it has no channel for it yet, or is changing its
    /// screen.
    pub fn resize(&mut self, (width, height): (usize, usize)) -> Option<Vec<u8>> {
        if self.reactivating.is_some() {
            return None;
        }
        let side = |side: usize| u32::try_from(side).unwrap_or(u32::MAX);
        let (width, height) = MonitorLayoutEntry::adjust_display_size(side(width), side(height));
        // xrdp 0.10 stops when a request has no physical size: always send one, in
        // millimetres at 96 pixels for an inch.
        let millimetres = |pixels: u32| pixels * 254 / 960;
        let physical = Some((millimetres(width), millimetres(height)));
        self.stage
            .encode_resize(width, height, Some(100), physical)?
            .ok()
    }

    /// Tells the device whether the client's clipboard holds text. `None` when the
    /// channel is not open.
    pub fn offer_text(&mut self, has_text: bool) -> Option<Vec<u8>> {
        let formats: Vec<_> = has_text
            .then(|| ClipboardFormat::new(TEXT))
            .into_iter()
            .collect();
        let channel = self.stage.get_svc_processor_mut::<CliprdrClient>()?;
        let messages = channel.initiate_copy(&formats).ok()?;
        self.stage.process_svc_processor_messages(messages).ok()
    }

    /// Asks the device for the text that was copied on it.
    pub fn ask_for_text(&mut self) -> Option<Vec<u8>> {
        let channel = self.stage.get_svc_processor_mut::<CliprdrClient>()?;
        let messages = channel.initiate_paste(TEXT).ok()?;
        self.stage.process_svc_processor_messages(messages).ok()
    }

    /// Answers the device's request for the client's text; with `None` there is none
    /// for it.
    pub fn give_text(&mut self, text: Option<&str>) -> Option<Vec<u8>> {
        let response = text.map_or_else(
            FormatDataResponse::new_error,
            FormatDataResponse::new_unicode_string,
        );
        let channel = self.stage.get_svc_processor_mut::<CliprdrClient>()?;
        let messages = channel.submit_format_data(response.into_owned()).ok()?;
        self.stage.process_svc_processor_messages(messages).ok()
    }
}
