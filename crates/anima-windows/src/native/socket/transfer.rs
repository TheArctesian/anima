//! Transfer one connected socket to a named child without reading its network payload.
use std::{
    io,
    net::TcpStream,
    os::windows::io::{AsRawSocket, FromRawSocket},
    sync::OnceLock,
};
use windows::Win32::Networking::WinSock::{
    SOCKET, WSA_FLAG_NO_HANDLE_INHERIT, WSA_FLAG_OVERLAPPED, WSADATA, WSADuplicateSocketW,
    WSAGetLastError, WSAPROTOCOL_INFOW, WSASocketW, WSAStartup,
};

/// Length of the opaque, process-specific transfer record. Never persist this record.
pub const RECORD_BYTES: usize = size_of::<WSAPROTOCOL_INFOW>();

/// Prepare a socket for exactly one import by `process`.
/// Keep the original socket open until the receiver acknowledges import.
/// # Errors
/// Winsock cannot duplicate the connected socket for that process.
pub fn export(connection: &TcpStream, process: u32) -> io::Result<Vec<u8>> {
    let socket = usize::try_from(connection.as_raw_socket()).map_err(io::Error::other)?;
    // A u32 buffer provides the structure's alignment and initializes padding as well.
    let mut record = vec![0_u32; RECORD_BYTES.div_ceil(size_of::<u32>())];
    // SAFETY: the live socket is borrowed; the aligned, initialized buffer is large enough.
    if unsafe { WSADuplicateSocketW(SOCKET(socket), process, record.as_mut_ptr().cast()) } != 0 {
        return Err(last_error());
    }
    // SAFETY: all bytes, including padding, were initialized before the OS call.
    Ok(unsafe { std::slice::from_raw_parts(record.as_ptr().cast::<u8>(), RECORD_BYTES) }.to_vec())
}

/// Import a record received over a private pipe from the parent process.
/// # Errors
/// The record is malformed, belongs to another process, or Winsock cannot import it.
pub fn import(record: &[u8]) -> io::Result<TcpStream> {
    if record.len() != RECORD_BYTES {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    initialize()?;
    let mut protocol = WSAPROTOCOL_INFOW::default();
    // SAFETY: the destination is aligned and initialized, and both regions have RECORD_BYTES.
    unsafe {
        std::ptr::copy_nonoverlapping(
            record.as_ptr(),
            (&raw mut protocol).cast::<u8>(),
            RECORD_BYTES,
        );
    }
    // SAFETY: protocol outlives the call; Winsock validates the process-specific record.
    let socket = unsafe {
        WSASocketW(
            -1,
            -1,
            -1,
            Some(&raw const protocol),
            0,
            WSA_FLAG_OVERLAPPED | WSA_FLAG_NO_HANDLE_INHERIT,
        )
    }
    .map_err(|_| last_error())?;
    // SAFETY: successful WSASocketW returned a new owned socket, released by TcpStream.
    Ok(unsafe { TcpStream::from_raw_socket(socket.0 as _) })
}

fn initialize() -> io::Result<()> {
    static STARTED: OnceLock<Result<(), i32>> = OnceLock::new();
    STARTED
        .get_or_init(|| {
            let mut state = WSADATA::default();
            // SAFETY: writable state outlives the call. This process-lifetime reference keeps
            // Winsock available until all imported sockets have been closed at process exit.
            let result = unsafe { WSAStartup(0x0202, &raw mut state) };
            if result == 0 { Ok(()) } else { Err(result) }
        })
        .map_err(io::Error::from_raw_os_error)
}

fn last_error() -> io::Error {
    // SAFETY: reads only this thread's Winsock error.
    io::Error::from_raw_os_error(unsafe { WSAGetLastError() }.0)
}
