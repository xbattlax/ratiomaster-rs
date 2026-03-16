/// Represents a single file within a torrent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorrentFile {
    /// The path components of the file (e.g., `["dir", "subdir", "file.txt"]`).
    pub path: Vec<String>,
    /// The size of the file in bytes.
    pub length: u64,
}

/// Parsed representation of a .torrent file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorrentMetainfo {
    /// The primary tracker announce URL.
    pub announce: String,

    /// Optional list of tracker tier lists (BEP 12).
    pub announce_list: Option<Vec<Vec<String>>>,

    /// The name of the torrent (typically the root directory or single file name).
    pub name: String,

    /// The number of bytes per piece.
    pub piece_length: u64,

    /// The concatenated SHA1 hashes of each piece (each hash is 20 bytes).
    pub pieces: Vec<u8>,

    /// If this is a single-file torrent, the file length.
    pub length: Option<u64>,

    /// If this is a multi-file torrent, the list of files.
    pub files: Option<Vec<TorrentFile>>,

    /// Optional comment embedded in the torrent.
    pub comment: Option<String>,

    /// Optional creator string.
    pub created_by: Option<String>,

    /// Optional creation timestamp (Unix epoch seconds).
    pub creation_date: Option<i64>,

    /// The 20-byte SHA1 hash of the bencoded info dictionary.
    pub info_hash: [u8; 20],
}

impl TorrentMetainfo {
    /// Returns the total size of all files in the torrent.
    pub fn total_size(&self) -> u64 {
        if let Some(length) = self.length {
            length
        } else if let Some(ref files) = self.files {
            files.iter().map(|f| f.length).sum()
        } else {
            0
        }
    }

    /// Returns the number of pieces in the torrent.
    pub fn piece_count(&self) -> usize {
        self.pieces.len() / 20
    }

    /// Returns `true` if this is a single-file torrent.
    pub fn is_single_file(&self) -> bool {
        self.length.is_some()
    }

    /// Returns `true` if this is a multi-file torrent.
    pub fn is_multi_file(&self) -> bool {
        self.files.is_some()
    }
}
