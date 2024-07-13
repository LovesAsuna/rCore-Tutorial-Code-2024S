//! File trait & inode(dir, file, pipe, stdin, stdout)

pub use inode::{link_file, unlink_file, list_apps, open_file, OpenFlags};
pub use stdio::{Stdin, Stdout};

use crate::mm::UserBuffer;

mod inode;
mod stdio;

/// trait File for all file types
pub trait File: Send + Sync {
    /// the file readable?
    fn readable(&self) -> bool;
    /// the file writable?
    fn writable(&self) -> bool;
    /// read from the file to buf, return the number of bytes read
    fn read(&self, buf: UserBuffer) -> usize;
    /// write to the file from buf, return the number of bytes written
    fn write(&self, buf: UserBuffer) -> usize;
    /// get inode id
    fn inode_id(&self) -> Option<u32>;
    /// get name
    fn link_count(&self) -> Option<u32>;
}

/// The stat of a inode
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Stat {
    /// ID of device containing file
    pub dev: u64,
    /// inode number
    pub ino: u64,
    /// file type and mode
    pub mode: StatMode,
    /// number of hard links
    pub nlink: u32,
    /// unused pad
    pad: [u64; 7],
}

impl Stat {
    /// new a file stat
    pub fn new(ino: u64, mode: StatMode, nlink: u32) -> Stat {
        Self {
            dev: 0,
            ino,
            mode,
            nlink,
            pad: [0; 7]
        }
    }
}

bitflags! {
    /// The mode of a inode
    /// whether a directory or a file
    pub struct StatMode: u32 {
        /// null
        const NULL  = 0;
        /// directory
        const DIR   = 0o040000;
        /// ordinary regular file
        const FILE  = 0o100000;
    }
}

