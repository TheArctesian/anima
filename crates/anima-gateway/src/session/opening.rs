//! A session from the hub's first instruction to a signed-in device: the handshake, the
//! connection to the device, a prompt for what the sign-in still needs (GW-015), and the
//! sign-in. Each way it can fail is told to the hub with the status that gives the session
//! its end reason (K4 Section 5). I/O edge.

use guac_protocol::Instruction;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::time::timeout;
use zeroize::{Zeroize, Zeroizing};

use super::hub::Hub;
use super::{Ended, Settings, status};
use crate::handshake::{
    self, Credential, Handshake, Opening, Prompt, PromptStep, Refused, Secret, Step,
};
use crate::vnc::{self, Account, SignInError};

/// The most instructions a hub sends before `connect`: seven, and room for `nop`.
const MOST_BEFORE_CONNECT: usize = 32;
/// Random bytes for one sign-in: the longest secret number and the filler of Apple's method.
const RANDOM_BYTES: usize = 512 + 128;

/// A session with its device signed in.
pub(super) struct Opened {
    pub(super) opening: Opening,
    pub(super) device: TcpStream,
    /// The device's screen size.
    pub(super) size: (usize, usize),
}

/// How the opening goes on after the first instruction.
pub(super) enum First {
    Vnc(Opening),
    /// `select` of a protocol the gateway does not serve: for another gateway, if any.
    Other(Instruction),
}

/// Reads the hub's handshake up to `connect` and answers it with `ready`.
pub(super) async fn handshake<S>(hub: &mut Hub<S>, connection_id: &str) -> Result<First, Ended>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let mut handshake = Handshake::default();
    for _ in 0..MOST_BEFORE_CONNECT {
        let instruction = hub.next().await.ok_or(Ended::ByHub)?;
        let select = instruction.opcode == "select";
        let kept = select.then(|| instruction.clone());
        match handshake.on_instruction(instruction) {
            Ok(Step::Send(args)) => hub.send(&[args]).await.map_err(|_| Ended::ByHub)?,
            Ok(Step::Read) => {}
            Ok(Step::Opened(opening)) => {
                let ready = handshake::ready(connection_id);
                hub.send(&[ready]).await.map_err(|_| Ended::ByHub)?;
                return Ok(First::Vnc(*opening));
            }
            Err(Refused::NotServed) => return kept.map(First::Other).ok_or(Ended::Refused),
            Err(refused) => return Err(refuse(hub, refused.status(), Ended::Refused).await),
        }
    }
    Err(refuse(hub, Refused::Unexpected.status(), Ended::Refused).await)
}

/// Tells the hub the session ends with `status`, and gives `ended` back.
pub(super) async fn refuse<S>(hub: &mut Hub<S>, status: u16, ended: Ended) -> Ended
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    // The end stands whether or not the hub still hears of it.
    let _ = hub.send(&[handshake::error(status)]).await;
    ended
}

/// Connects to the device and signs in, asking the operator for what is missing.
pub(super) async fn sign_in<S>(
    hub: &mut Hub<S>,
    mut opening: Opening,
    settings: &Settings,
) -> Result<Opened, Ended>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let mut random = Zeroizing::new([0u8; RANDOM_BYTES]);
    if getrandom::fill(&mut *random).is_err() {
        return Err(refuse(hub, status::SERVER_ERROR, Ended::NoRandomNumbers).await);
    }
    let connecting = timeout(settings.device_wait, TcpStream::connect(opening.device)).await;
    let Ok(Ok(mut device)) = connecting else {
        return Err(refuse(hub, status::UPSTREAM_NOT_FOUND, Ended::DeviceUnreachable).await);
    };
    // Keys and pointer moves are small: do not hold them back (Nagle).
    let _ = device.set_nodelay(true);
    let Ok(Ok(offered)) = timeout(settings.device_wait, vnc::offered(&mut device)).await else {
        return Err(refuse(hub, status::UPSTREAM_ERROR, Ended::NotVnc).await);
    };
    let Ok(method) = offered.method(opening.username.is_some(), opening.password.is_some()) else {
        return Err(refuse(hub, status::UPSTREAM_ERROR, Ended::NoSignInMethod).await);
    };
    ask_for_missing(hub, &mut opening, method.needs(), settings).await?;
    let text = |secret: &Option<Secret>| secret.as_ref().map_or("", Secret::expose).to_owned();
    let (username, password) = (
        Zeroizing::new(text(&opening.username)),
        Zeroizing::new(text(&opening.password)),
    );
    let account = Account {
        username: &username,
        password: &password,
    };
    let mut left = &random[..];
    let mut hand_out = |buffer: &mut [u8]| {
        let (taken, rest) = left.split_at(buffer.len().min(left.len()));
        buffer[..taken.len()].copy_from_slice(taken);
        left = rest;
    };
    let signing = vnc::sign_in(&mut device, method, account, &mut hand_out);
    match timeout(settings.device_wait, signing).await {
        Ok(Ok(size)) => Ok(Opened {
            opening,
            device,
            size,
        }),
        Ok(Err(SignInError::Refused)) => {
            Err(refuse(hub, status::CLIENT_UNAUTHORIZED, Ended::SignInRefused).await)
        }
        _ => Err(refuse(hub, status::UPSTREAM_ERROR, Ended::SignInFailed).await),
    }
}

/// Asks the operator for each credential in `needs` that the hub did not send, and puts
/// the answers into `opening`.
async fn ask_for_missing<S>(
    hub: &mut Hub<S>,
    opening: &mut Opening,
    needs: &[Credential],
    settings: &Settings,
) -> Result<(), Ended>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let has = |credential: &Credential| match credential {
        Credential::Username => opening.username.is_some(),
        Credential::Password => opening.password.is_some(),
    };
    let missing: Vec<Credential> = needs
        .iter()
        .copied()
        .filter(|credential| !has(credential))
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    let (mut prompt, required) = Prompt::ask(&missing);
    hub.send(&[required]).await.map_err(|_| Ended::ByHub)?;
    let answers = async {
        while !prompt.is_answered() {
            let mut instruction = hub.next().await.ok_or(Ended::ByHub)?;
            let step = prompt.on_instruction(&instruction);
            let disconnect = instruction.opcode == "disconnect";
            // An answer's `blob` holds a part of the credential.
            instruction.args.iter_mut().for_each(Zeroize::zeroize);
            let reply = match step {
                PromptStep::NotAnAnswer if disconnect => return Err(Ended::ByHub),
                PromptStep::NotAnAnswer => continue,
                // A refused answer: the hub lets the client answer only a new `required`.
                PromptStep::Acknowledge(ack)
                    if ack.args.get(2).is_some_and(|status| status != "0") =>
                {
                    vec![ack, prompt.required()]
                }
                PromptStep::Acknowledge(ack) => vec![ack],
                PromptStep::Answered {
                    credential,
                    value,
                    ack,
                } => {
                    match credential {
                        Credential::Username => opening.username = Some(value),
                        Credential::Password => opening.password = Some(value),
                    }
                    vec![ack]
                }
            };
            hub.send(&reply).await.map_err(|_| Ended::ByHub)?;
        }
        Ok(())
    };
    match timeout(settings.answer_wait, answers).await {
        Ok(answered) => answered,
        Err(_) => Err(refuse(hub, status::CLIENT_TIMEOUT, Ended::NoAnswer).await),
    }
}
