//! A VNC device on a loopback port for the session tests: it signs a client in, answers
//! the first update request with one colour, notes what the client sends, and does what
//! the test tells it.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use anima_gateway::screen::Area;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

use crate::vnc::rfb;

pub(crate) const SIZE: (usize, usize) = (64, 48);
pub(crate) const ALL: Area = Area {
    x: 0,
    y: 0,
    width: SIZE.0,
    height: SIZE.1,
};

/// What the test tells the device to send.
pub(crate) enum Tell {
    Bytes(Vec<u8>),
    Close,
}

/// How the device treats a sign-in.
#[derive(Clone, Copy)]
pub(crate) enum SignIn {
    None,
    /// A VNC password; `accepted` is the device's verdict on it.
    Password {
        accepted: bool,
    },
}

pub(crate) struct Device {
    pub(crate) address: SocketAddr,
    pub(crate) tell: mpsc::Sender<Tell>,
    /// What the client sent after sign-in, one line for each message.
    pub(crate) heard: Arc<Mutex<Vec<String>>>,
}

impl Device {
    pub(crate) async fn start(sign_in: SignIn) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
        let address = listener.local_addr().expect("an address");
        let (tell, told) = mpsc::channel(16);
        let heard = Arc::new(Mutex::new(Vec::new()));
        let notes = heard.clone();
        tokio::spawn(async move {
            let (client, _) = listener.accept().await.expect("a client");
            // The session ending first is not the device's fault.
            let _ = serve(client, sign_in, told, notes).await;
        });
        Self {
            address,
            tell,
            heard,
        }
    }

    pub(crate) fn heard(&self) -> Vec<String> {
        self.heard.lock().expect("notes").clone()
    }
}

async fn serve(
    mut client: TcpStream,
    sign_in: SignIn,
    mut told: mpsc::Receiver<Tell>,
    heard: Arc<Mutex<Vec<String>>>,
) -> std::io::Result<()> {
    if !signed_in(&mut client, sign_in).await? {
        return Ok(());
    }
    loop {
        tokio::select! {
            kind = client.read_u8() => hear(&mut client, kind?, &heard).await?,
            tell = told.recv() => match tell {
                Some(Tell::Bytes(bytes)) => client.write_all(&bytes).await?,
                Some(Tell::Close) | None => return Ok(()),
            },
        }
    }
}

/// The opening up to `ClientInit`; `false` when the device refused the sign-in.
async fn signed_in(client: &mut TcpStream, sign_in: SignIn) -> std::io::Result<bool> {
    client.write_all(b"RFB 003.008\n").await?;
    client.read_exact(&mut [0; 12]).await?;
    let (offered, accepted) = match sign_in {
        SignIn::None => (1, true),
        SignIn::Password { accepted } => (2, accepted),
    };
    client.write_all(&[1, offered]).await?;
    client.read_u8().await?;
    if offered == 2 {
        client.write_all(&[7; 16]).await?;
        client.read_exact(&mut [0; 16]).await?;
    }
    if !accepted {
        client.write_all(&[0, 0, 0, 1, 0, 0, 0, 0]).await?;
        return Ok(false);
    }
    client.write_all(&[0, 0, 0, 0, 0, 64, 0, 48]).await?;
    client.write_all(&[0; 16]).await?;
    client.write_all(&[0, 0, 0, 0]).await?;
    client.read_u8().await?; // ClientInit
    Ok(true)
}

/// Reads one client message of type `kind` and notes it. The first full update request is
/// answered with a red screen.
async fn hear(client: &mut TcpStream, kind: u8, heard: &Mutex<Vec<String>>) -> std::io::Result<()> {
    let mut body = vec![
        0;
        match kind {
            0 => 19,
            2 => 3,
            3 => 9,
            4 | 6 => 7,
            5 => 5,
            _ => 0,
        }
    ];
    client.read_exact(&mut body).await?;
    let note = match kind {
        2 => {
            let count = usize::from(u16::from_be_bytes([body[1], body[2]]));
            client.read_exact(&mut vec![0; count * 4]).await?;
            return Ok(());
        }
        3 if body[0] == 0 => {
            let red = [200, 0, 0].repeat(SIZE.0 * SIZE.1);
            client
                .write_all(&rfb::update(&[rfb::raw(ALL, &red)]))
                .await?;
            return Ok(());
        }
        4 => format!(
            "key {} {}",
            u32::from_be_bytes([body[3], body[4], body[5], body[6]]),
            body[0]
        ),
        5 => format!(
            "pointer {} {} {}",
            u16::from_be_bytes([body[1], body[2]]),
            u16::from_be_bytes([body[3], body[4]]),
            body[0]
        ),
        6 => {
            let mut text =
                vec![
                    0;
                    usize::try_from(u32::from_be_bytes([body[3], body[4], body[5], body[6]]))
                        .expect("short")
                ];
            client.read_exact(&mut text).await?;
            format!("cut {}", String::from_utf8_lossy(&text))
        }
        _ => return Ok(()),
    };
    heard.lock().expect("notes").push(note);
    Ok(())
}
