use std::{
    io,
    path::{Path, PathBuf},
};
pub(crate) mod real;

#[cfg(any(test, fuzzing))]
pub(crate) mod sim;
#[cfg(test)]
mod tests;

/// Why the log makes a call: a label a simulator can aim a fault at. A minor release may add a
/// purpose, so treat an unknown one like any other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Purpose {
    Open,
    Group,
    Recovery,
    Lookup,
    Roll,
    Summary,
    KeyFilter,
    KeyIndex,
    Seal,
    /// The log key filter's builder reads key index frames after open (ADR 0051).
    LogFilter,
    /// The operator repair tool's backup and truncation (ADR 0065).
    Repair,
    /// A clean drop's close frame (ADR 0071).
    Close,
}
/// One file or directory operation. Each returns `Value::Unit` unless it says otherwise; a wrong
/// value is an I/O error. The contract is in api.md (ADR 0072).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Op {
    /// Fails with `AlreadyExists` if the path exists.
    CreateDir,
    /// Creates an empty file. Fails with `AlreadyExists` if the path exists.
    Create,
    /// Returns the file's length as `Value::Length`.
    Open,
    /// Writes at the end of the file.
    Append(Vec<u8>),
    /// Writes at a byte position, growing the file if needed.
    Write(u64, Vec<u8>),
    /// Returns `Value::Bytes` of at most this many bytes from a position, short only at the end
    /// of the file.
    Read(u64, usize),
    SyncData,
    SyncAll,
    /// Makes the directory's entries durable.
    SyncDir,
    Truncate(u64),
    Remove,
    /// Returns the directory's file names, sorted, as `Value::Names`.
    List,
    /// Returns this many random bytes as `Value::Bytes`, for the log id.
    Random(usize),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    pub path: PathBuf,
    pub purpose: Purpose,
    pub op: Op,
}
impl Call {
    pub fn new(path: impl AsRef<Path>, purpose: Purpose, op: Op) -> Self {
        Self {
            path: path.as_ref().to_owned(),
            purpose,
            op,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Value {
    Unit,
    Length(u64),
    Bytes(Vec<u8>),
    Names(Vec<PathBuf>),
}
/// The directory lock. Dropping it releases the lock.
pub trait Lock: Send {}
/// Every file and directory operation the log performs (ADR 0072). The log never retries a failed
/// `SyncData`, `SyncAll` or `SyncDir` (ADR 0004). A storage must not call back into the log.
// One command seam keeps recorded calls identical to the operations being simulated.
pub trait Storage: Send + Sync {
    fn call(&self, call: Call) -> io::Result<Value>;
    /// Fails with `WouldBlock` while another guard holds the lock.
    fn lock(&self, path: &Path) -> io::Result<Box<dyn Lock>>;
}
