//! The device's SFTP: a `russh-sftp` server handler over a temporary directory, as a device's
//! `sftp-server` would answer. Paths are absolute inside the directory (`/` is its root); `..`
//! is refused; an exclusive open of an existing file fails as POSIX `O_EXCL` does.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::future::ready;
use std::io::{self, Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use russh_sftp::protocol::{
    Attrs, Data, File, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode,
};

/// One open handle.
enum Opened {
    File(fs::File),
    /// A directory's entries, read once.
    Dir(Option<Vec<File>>),
}

/// The SFTP server's side of one channel: the directory and the open handles.
#[derive(Clone)]
pub struct Disk {
    root: PathBuf,
    handles: Arc<Mutex<HashMap<String, Opened>>>,
    opened: Arc<AtomicUsize>,
}

fn ok(id: u32) -> Status {
    Status {
        id,
        status_code: StatusCode::Ok,
        error_message: "Ok".to_owned(),
        language_tag: "en-US".to_owned(),
    }
}

/// The status OpenSSH's `sftp-server` sends for an I/O error.
fn status(error: &io::Error) -> StatusCode {
    match error.kind() {
        io::ErrorKind::NotFound => StatusCode::NoSuchFile,
        io::ErrorKind::PermissionDenied => StatusCode::PermissionDenied,
        _ => StatusCode::Failure,
    }
}

impl Disk {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            handles: Arc::default(),
            opened: Arc::default(),
        }
    }

    /// The real path of `path`, refusing `..`.
    fn real(&self, path: &str) -> Result<PathBuf, StatusCode> {
        let mut real = self.root.clone();
        for component in Path::new(path).components() {
            match component {
                Component::Normal(part) => real.push(part),
                Component::RootDir | Component::CurDir => {}
                Component::ParentDir | Component::Prefix(_) => {
                    return Err(StatusCode::PermissionDenied);
                }
            }
        }
        Ok(real)
    }

    fn hold(&self, opened: Opened) -> String {
        let handle = format!("h{}", self.opened.fetch_add(1, Ordering::SeqCst));
        self.handles
            .lock()
            .expect("the handles")
            .insert(handle.clone(), opened);
        handle
    }

    fn entries(real: &Path) -> io::Result<Vec<File>> {
        let mut files = Vec::new();
        for entry in fs::read_dir(real)? {
            let entry = entry?;
            let metadata = entry.path().symlink_metadata()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            files.push(File::new(name, FileAttributes::from(&metadata)));
        }
        files.sort_by(|a, b| a.filename.cmp(&b.filename));
        Ok(files)
    }

    fn file<T>(
        &self,
        handle: &str,
        use_file: impl FnOnce(&mut fs::File) -> io::Result<T>,
    ) -> Result<T, StatusCode> {
        let mut handles = self.handles.lock().expect("the handles");
        match handles.get_mut(handle) {
            Some(Opened::File(file)) => use_file(file).map_err(|error| status(&error)),
            _ => Err(StatusCode::Failure),
        }
    }

    fn resolve(&self, path: &str) -> Result<Vec<File>, StatusCode> {
        let real = self.real(path)?;
        fs::metadata(&real).map_err(|error| status(&error))?;
        let inside = real
            .strip_prefix(&self.root)
            .map_err(|_| StatusCode::Failure)?;
        Ok(vec![File::dummy(format!("/{}", inside.to_string_lossy()))])
    }

    fn open_dir(&self, path: &str) -> Result<String, StatusCode> {
        let files = Self::entries(&self.real(path)?).map_err(|error| status(&error))?;
        Ok(self.hold(Opened::Dir(Some(files))))
    }

    fn next_entries(&self, handle: &str) -> Result<Vec<File>, StatusCode> {
        match self.handles.lock().expect("the handles").get_mut(handle) {
            Some(Opened::Dir(files)) => files.take().ok_or(StatusCode::Eof),
            _ => Err(StatusCode::Failure),
        }
    }

    fn open_file(&self, path: &str, pflags: OpenFlags) -> Result<String, StatusCode> {
        let real = self.real(path)?;
        let mut options = OpenOptions::new();
        options
            .read(pflags.contains(OpenFlags::READ))
            .write(pflags.contains(OpenFlags::WRITE))
            .truncate(pflags.contains(OpenFlags::TRUNCATE));
        if pflags.contains(OpenFlags::CREATE | OpenFlags::EXCLUDE) {
            options.create_new(true);
        } else {
            options.create(pflags.contains(OpenFlags::CREATE));
        }
        let file = options.open(real).map_err(|error| status(&error))?;
        Ok(self.hold(Opened::File(file)))
    }

    fn read_at(&self, handle: &str, offset: u64, len: u32) -> Result<Vec<u8>, StatusCode> {
        let data = self.file(handle, |file| {
            file.seek(SeekFrom::Start(offset))?;
            let mut data = vec![0; len as usize];
            let read = file.read(&mut data)?;
            data.truncate(read);
            Ok(data)
        })?;
        if data.is_empty() {
            return Err(StatusCode::Eof);
        }
        Ok(data)
    }

    fn write_at(&self, handle: &str, offset: u64, data: &[u8]) -> Result<(), StatusCode> {
        self.file(handle, |file| {
            file.seek(SeekFrom::Start(offset))?;
            file.write_all(data)
        })
    }

    fn attrs(&self, path: &str, follow: bool) -> Result<FileAttributes, StatusCode> {
        let real = self.real(path)?;
        let metadata = if follow {
            fs::metadata(real)
        } else {
            fs::symlink_metadata(real)
        };
        let metadata = metadata.map_err(|error| status(&error))?;
        Ok(FileAttributes::from(&metadata))
    }

    fn close_handle(&self, handle: &str) -> Result<(), StatusCode> {
        let closed = self.handles.lock().expect("the handles").remove(handle);
        closed.map(drop).ok_or(StatusCode::Failure)
    }

    fn remove_file(&self, path: &str) -> Result<(), StatusCode> {
        fs::remove_file(self.real(path)?).map_err(|error| status(&error))
    }
}

type Answer<T> = Result<T, StatusCode>;

impl russh_sftp::server::Handler for Disk {
    type Error = StatusCode;

    fn unimplemented(&self) -> Self::Error {
        StatusCode::OpUnsupported
    }

    fn realpath(&mut self, id: u32, path: String) -> impl Future<Output = Answer<Name>> + Send {
        ready(self.resolve(&path).map(|files| Name { id, files }))
    }

    fn opendir(&mut self, id: u32, path: String) -> impl Future<Output = Answer<Handle>> + Send {
        ready(self.open_dir(&path).map(|handle| Handle { id, handle }))
    }

    fn readdir(&mut self, id: u32, handle: String) -> impl Future<Output = Answer<Name>> + Send {
        ready(self.next_entries(&handle).map(|files| Name { id, files }))
    }

    fn open(
        &mut self,
        id: u32,
        filename: String,
        pflags: OpenFlags,
        _attrs: FileAttributes,
    ) -> impl Future<Output = Answer<Handle>> + Send {
        ready(
            self.open_file(&filename, pflags)
                .map(|handle| Handle { id, handle }),
        )
    }

    fn read(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        len: u32,
    ) -> impl Future<Output = Answer<Data>> + Send {
        ready(
            self.read_at(&handle, offset, len)
                .map(|data| Data { id, data }),
        )
    }

    fn write(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> impl Future<Output = Answer<Status>> + Send {
        ready(self.write_at(&handle, offset, &data).map(|()| ok(id)))
    }

    fn fstat(&mut self, id: u32, handle: String) -> impl Future<Output = Answer<Attrs>> + Send {
        let attrs = self.file(&handle, |file| Ok(FileAttributes::from(&file.metadata()?)));
        ready(attrs.map(|attrs| Attrs { id, attrs }))
    }

    fn stat(&mut self, id: u32, path: String) -> impl Future<Output = Answer<Attrs>> + Send {
        ready(self.attrs(&path, true).map(|attrs| Attrs { id, attrs }))
    }

    fn lstat(&mut self, id: u32, path: String) -> impl Future<Output = Answer<Attrs>> + Send {
        ready(self.attrs(&path, false).map(|attrs| Attrs { id, attrs }))
    }

    fn close(&mut self, id: u32, handle: String) -> impl Future<Output = Answer<Status>> + Send {
        ready(self.close_handle(&handle).map(|()| ok(id)))
    }

    fn remove(&mut self, id: u32, filename: String) -> impl Future<Output = Answer<Status>> + Send {
        ready(self.remove_file(&filename).map(|()| ok(id)))
    }
}
