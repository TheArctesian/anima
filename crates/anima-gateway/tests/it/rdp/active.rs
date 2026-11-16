//! The test device carries an active session: what it paints reaches `IronRDP`'s session,
//! the client's keys and pointer reach it, and it changes its size or closes when told.

use anima_gateway::handshake::{Certificate, Security};
use anima_gateway::rdp::{Button, Connection, KeyEvent, PointerEvent};
use anima_gateway::screen::Area;
use ironrdp_connector::connection_activation::{
    ConnectionActivationFactory, ConnectionActivationState,
};
use ironrdp_core::WriteBuf;
use ironrdp_graphics::image_processing::PixelFormat;
use ironrdp_pdu::input::MousePdu;
use ironrdp_pdu::input::fast_path::{FastPathInputEvent, KeyboardFlags};
use ironrdp_pdu::input::mouse::PointerFlags;
use ironrdp_session::image::DecodedImage;
use ironrdp_session::{ActiveStage, ActiveStageBuilder, ActiveStageOutput};
use ironrdp_tokio::{FramedWrite as _, single_sequence_step};

use super::device::{Device, Seen, WAIT};
use super::opening::{TLS, opened, wanted};

/// A client's side of an active session, driven by the test as the gateway will drive it.
struct Client {
    connection: Connection,
    stage: ActiveStage,
    image: DecodedImage,
    activation: ConnectionActivationFactory,
}

impl Client {
    async fn signed_in(device: &mut Device) -> Self {
        let accepted = Certificate::Accepted(device.fingerprint);
        let opened = opened(device.address, wanted(Security::Tls, accepted))
            .await
            .expect("opened");
        assert!(matches!(device.next().await, Seen::SignIn { .. }));
        let result = opened.result;
        let size = result.desktop_size;
        let stage = ActiveStageBuilder {
            static_channels: result.static_channels,
            user_channel_id: result.user_channel_id,
            io_channel_id: result.io_channel_id,
            message_channel_id: result.message_channel_id,
            share_id: result.share_id,
            compression_type: result.compression_type,
            enable_server_pointer: result.enable_server_pointer,
            pointer_software_rendering: result.pointer_software_rendering,
        }
        .build();
        Self {
            connection: opened.connection,
            stage,
            image: DecodedImage::new(PixelFormat::RgbA32, size.width, size.height),
            activation: result.activation_factory,
        }
    }

    /// Processes what the device sends, answering it, until an output `ends` the wait.
    async fn until(&mut self, ends: impl Fn(&ActiveStageOutput) -> bool) -> ActiveStageOutput {
        loop {
            let (action, frame) = tokio::time::timeout(WAIT, self.connection.read_pdu())
                .await
                .expect("the device sends in time")
                .expect("a frame");
            let outputs = self
                .stage
                .process(&mut self.image, action, &frame, None)
                .expect("processed");
            for output in outputs {
                self.answer(&output).await;
                if ends(&output) {
                    return output;
                }
            }
        }
    }

    async fn answer(&mut self, output: &ActiveStageOutput) {
        if let ActiveStageOutput::ResponseFrame(frame) = output
            && !frame.is_empty()
        {
            self.connection.write_all(frame).await.expect("answered");
        }
    }

    /// Sends input events as the session encodes them.
    async fn send(&mut self, events: &[FastPathInputEvent]) {
        let outputs = self
            .stage
            .process_fastpath_input(&mut self.image, events)
            .expect("encoded");
        for output in outputs {
            self.answer(&output).await;
        }
    }

    /// The colour (red, green, blue) of one pixel of the session's picture.
    fn pixel(&self, x: usize, y: usize) -> [u8; 3] {
        let at = y * self.image.stride() + x * 4;
        let data = self.image.data();
        [data[at], data[at + 1], data[at + 2]]
    }

    /// Runs the Deactivation-Reactivation Sequence after a Deactivate All, and returns the
    /// new size.
    async fn reactivated(&mut self) -> (u16, u16) {
        let mut sequence = self.activation.create();
        let mut buf = WriteBuf::new();
        loop {
            let step = single_sequence_step(&mut self.connection, &mut sequence, &mut buf);
            tokio::time::timeout(WAIT, step)
                .await
                .expect("in time")
                .expect("a step");
            if let ConnectionActivationState::Finalized {
                desktop_size,
                share_id,
                static_channel_chunk_size,
                enable_server_pointer,
                pointer_software_rendering,
                ..
            } = sequence.connection_activation_state()
            {
                let ids = (sequence.io_channel_id(), sequence.user_channel_id());
                assert!(self.stage.reactivate(
                    ids.0,
                    ids.1,
                    share_id,
                    enable_server_pointer,
                    pointer_software_rendering,
                    static_channel_chunk_size,
                ));
                let (width, height) = (desktop_size.width, desktop_size.height);
                self.image = DecodedImage::new(PixelFormat::RgbA32, width, height);
                return (width, height);
            }
        }
    }
}

fn is_graphics(output: &ActiveStageOutput) -> bool {
    matches!(output, ActiveStageOutput::GraphicsUpdate(_))
}

#[tokio::test]
async fn what_the_device_paints_reaches_the_sessions_picture() {
    let mut device = Device::start(TLS);
    let mut client = Client::signed_in(&mut device).await;
    // Taller than one update holds, so it comes in bands.
    let area = Area {
        x: 10,
        y: 5,
        width: 300,
        height: 200,
    };

    device.paint(area, [200, 30, 0]).await;
    let mut painted = 0;
    while painted < area.width * area.height {
        let output = client.until(is_graphics).await;
        let ActiveStageOutput::GraphicsUpdate(rectangle) = output else {
            unreachable!("waited for graphics");
        };
        let width = usize::from(rectangle.right - rectangle.left + 1);
        painted += width * usize::from(rectangle.bottom - rectangle.top + 1);
    }

    for (x, y) in [(10, 5), (309, 204), (150, 100)] {
        assert_eq!(client.pixel(x, y), [200, 30, 0], "inside at {x}, {y}");
    }
    for (x, y) in [(9, 5), (310, 204), (150, 205), (0, 0)] {
        assert_eq!(client.pixel(x, y), [0, 0, 0], "outside at {x}, {y}");
    }
}

/// A mouse PDU at `x`, `y`.
fn mouse(flags: PointerFlags, units: i16, x: u16, y: u16) -> FastPathInputEvent {
    FastPathInputEvent::MouseEvent(MousePdu {
        flags,
        number_of_wheel_rotation_units: units,
        x_position: x,
        y_position: y,
    })
}

/// A button the device saw at `x`, `y`.
fn button(x: u16, y: u16, button: Button, pressed: bool) -> Seen {
    let event = PointerEvent::Button { button, pressed };
    Seen::Pointer { x, y, event }
}

#[tokio::test]
async fn the_device_sees_the_clients_keys_and_pointer_in_order() {
    let mut device = Device::start(TLS);
    let mut client = Client::signed_in(&mut device).await;
    client
        .send(&[
            FastPathInputEvent::KeyboardEvent(KeyboardFlags::empty(), 0x1e),
            FastPathInputEvent::KeyboardEvent(
                KeyboardFlags::RELEASE | KeyboardFlags::EXTENDED,
                0x48,
            ),
            FastPathInputEvent::UnicodeKeyboardEvent(KeyboardFlags::empty(), 0xe9),
            mouse(PointerFlags::MOVE, 0, 10, 20),
            mouse(PointerFlags::DOWN | PointerFlags::LEFT_BUTTON, 0, 10, 20),
            mouse(PointerFlags::RIGHT_BUTTON, 0, 11, 21),
            mouse(PointerFlags::VERTICAL_WHEEL, -120, 11, 21),
        ])
        .await;

    let key = |code, extended, pressed| {
        Seen::Key(KeyEvent::Scancode {
            code,
            extended,
            pressed,
        })
    };
    let expected = [
        key(0x1e, false, true),
        key(0x48, true, false),
        Seen::Key(KeyEvent::Unicode {
            unit: 0xe9,
            pressed: true,
        }),
        Seen::Pointer {
            x: 10,
            y: 20,
            event: PointerEvent::Move,
        },
        button(10, 20, Button::Left, true),
        button(11, 21, Button::Right, false),
        Seen::Pointer {
            x: 11,
            y: 21,
            event: PointerEvent::Wheel(-120),
        },
    ];
    for seen in expected {
        assert_eq!(device.next().await, seen);
    }
}

#[tokio::test]
async fn a_resize_deactivates_the_session_and_activates_it_again_at_the_new_size() {
    let mut device = Device::start(TLS);
    let mut client = Client::signed_in(&mut device).await;

    device.resize(400, 300).await;
    client
        .until(|output| matches!(output, ActiveStageOutput::DeactivateAll))
        .await;

    assert_eq!(client.reactivated().await, (400, 300));
    device
        .paint(
            Area {
                x: 390,
                y: 290,
                width: 10,
                height: 10,
            },
            [0, 0, 255],
        )
        .await;
    client.until(is_graphics).await;
    assert_eq!(client.pixel(399, 299), [0, 0, 255]);
}

#[tokio::test]
async fn the_device_closes_the_session_when_told() {
    let mut device = Device::start(TLS);
    let mut client = Client::signed_in(&mut device).await;

    device.close().await;

    assert_eq!(device.next().await, Seen::Ended);
    let read = tokio::time::timeout(WAIT, client.connection.read_pdu()).await;
    assert!(read.expect("the end in time").is_err());
}
