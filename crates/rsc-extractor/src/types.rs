use binrw::{NullString, binrw};
use chrono::{DateTime, Utc};

// Constants from the RSC/RAD format specification
/// Valid entry flag (RAD format)
const VALID_ENTRY: u8 = 0x01;

/// Maximum allowed entry size in bytes (256 MiB)
///
/// This limit prevents DoS attacks from crafted files with huge entry_length
/// or data_length values that would cause unbounded memory allocations.
pub const MAX_ENTRY_SIZE: u32 = 256 * 1024 * 1024;

/// Encryption flag bit (RSC format)
const ENCRYPTION_FLAG: u8 = 0x80;

// RSC entry type constants
const TYPE_MIDI: u8 = 0x01;
const TYPE_OGG_WAV: u8 = 0x02;
const TYPE_DMI_PNG: u8 = 0x03;
const TYPE_PNG: u8 = 0x06;
const TYPE_JPG: u8 = 0x0B;

/// RSC entry type enum representing the different file types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    Midi,
    OggWav,
    DmiPng,
    Png,
    Jpg,
    Unknown(u8),
}

impl std::fmt::Display for EntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryType::Midi => write!(f, "MIDI"),
            EntryType::OggWav => write!(f, "OGG/WAV"),
            EntryType::DmiPng => write!(f, "DMI_PNG"),
            EntryType::Png => write!(f, "PNG"),
            EntryType::Jpg => write!(f, "JPG"),
            EntryType::Unknown(v) => write!(f, "Unknown(0x{:02X})", v),
        }
    }
}

impl EntryType {
    /// Convert a u8 entry type to an EntryType enum
    pub fn from_u8(value: u8) -> Self {
        let base_type = value & !ENCRYPTION_FLAG;
        match base_type {
            TYPE_MIDI => EntryType::Midi,
            TYPE_OGG_WAV => EntryType::OggWav,
            TYPE_DMI_PNG => EntryType::DmiPng,
            TYPE_PNG => EntryType::Png,
            TYPE_JPG => EntryType::Jpg,
            _ => EntryType::Unknown(base_type),
        }
    }
}

/// RAD (Random Access Data) outer structure wrapping each entry.
///
/// This is the format behind RSC and savefile formats, designed for
/// arbitrary-access writing needed for maintaining `byond.rsc` or modifying savegames.
///
/// Structure (as per RAD.md):
/// 1. Uint32 entryLength
/// 2. Uint8 valid (0x01 for valid entries, 0x00 for invalid entries)
/// 3. Array of entryLength Uint8s entryContent
#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
pub struct RadEntry {
    /// Length of the entry content in bytes
    #[br(assert(entry_length <= MAX_ENTRY_SIZE, "Entry too large: {} bytes (max: {} bytes)", entry_length, MAX_ENTRY_SIZE))]
    pub entry_length: u32,
    /// Valid flag (0x01 = valid, 0x00 = invalid)
    ///
    /// Note: Invalid entries will have nonsensical content that can crash your reader.
    pub valid: u8,
    /// The entry content (RSC entry data)
    #[br(count = entry_length)]
    pub content: Vec<u8>,
}

impl RadEntry {
    /// Check if this RAD entry is valid (valid == 0x01)
    ///
    /// Invalid entries should be skipped as they contain nonsensical content.
    pub fn is_valid(&self) -> bool {
        self.valid == VALID_ENTRY
    }
}

/// RSC (*.rsc) entry structure representing a cached resource file.
///
/// This is the inner structure within a RAD entry that identifies entry components.
///
/// Structure (as per RSC.md):
/// 1. Uint8 typeOrSomething (corresponds to Cache File typeOrSomething in DMB)
/// 2. Uint32 uniqueID (corresponds to Cache File uniqueID in DMB)
/// 3. Uint32 timestamp (seconds since Unix epoch, UTC)
/// 4. Uint32 originalTimestamp (modification time of the imported file)
/// 5. Uint32 dataLength
/// 6. Zero-terminated string filename
/// 7. Array of dataLength Uint8s data
#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
pub struct RscEntry {
    /// Entry type (includes encryption flag 0x80 in high bit)
    ///
    /// Corresponds to Cache File typeOrSomething in DMB.
    /// Base types: 0x01 (MIDI), 0x02 (OGG/WAV), 0x03 (DMI PNG), 0x06 (PNG), 0x0B (JPG)
    pub entry_type: u8,

    /// Unique identifier (probably a checksum/hash of the data/entry, details unknown)
    ///
    /// Corresponds to Cache File uniqueID in DMB.
    pub unique_id: u32,

    /// Timestamp in seconds since Unix epoch (UTC)
    pub timestamp: u32,

    /// Original timestamp - modification time of the imported file
    ///
    /// In `byond.rsc`, this is 0. Used to determine which files to update.
    pub original_timestamp: u32,

    /// Length of the data field in bytes
    #[br(assert(data_length <= MAX_ENTRY_SIZE, "Data too large: {} bytes (max: {} bytes)", data_length, MAX_ENTRY_SIZE))]
    pub data_length: u32,

    /// Null-terminated filename
    pub filename: NullString,

    /// The actual file data
    ///
    /// Usually unencrypted unless in `byond.rsc`.
    /// DreamDaemon does not understand encrypted entries.
    #[br(count = data_length)]
    pub data: Vec<u8>,

    /// Inferred MIME type from the actual data
    ///
    /// This field is populated after reading the entry by analyzing the data bytes.
    /// It may not match the declared entry_type.
    #[br(default)]
    #[bw(ignore)]
    pub inferred_mime: Option<String>,
}

impl RscEntry {
    /// Check if this entry is encrypted (0x80 flag set)
    pub fn is_encrypted(&self) -> bool {
        (self.entry_type & ENCRYPTION_FLAG) != 0
    }

    /// Get the base entry type without the encryption flag
    pub fn base_type(&self) -> u8 {
        self.entry_type & !ENCRYPTION_FLAG
    }

    /// Get the entry type as an enum
    pub fn get_entry_type(&self) -> EntryType {
        EntryType::from_u8(self.entry_type)
    }

    /// Get a human-readable string representation of the entry type
    pub fn type_str(&self) -> &'static str {
        entry_type_to_str(self.entry_type)
    }

    /// Get the timestamp as a formatted UTC date string
    pub fn timestamp_str(&self) -> String {
        format_unix_timestamp(self.timestamp)
    }

    /// Get the original timestamp as a formatted UTC date string
    pub fn original_timestamp_str(&self) -> String {
        if self.original_timestamp == 0 {
            "N/A (byond.rsc)".to_string()
        } else {
            format_unix_timestamp(self.original_timestamp)
        }
    }

    /// Infer the MIME type from the actual data bytes
    pub fn infer_mime_type(&mut self) {
        self.inferred_mime = infer::get(&self.data).map(|kind| kind.mime_type().to_string());
    }

    /// Get file extension from inferred MIME type
    ///
    /// Returns the appropriate file extension (with dot) for the inferred MIME type,
    /// or None if no MIME type was inferred.
    ///
    /// Special case: If the entry type is TYPE_DMI_PNG and no MIME type could be inferred,
    /// returns ".dmi" extension.
    pub fn extension_from_mime(&self) -> Option<&'static str> {
        match self.inferred_mime.as_deref() {
            Some("audio/midi") => Some(".midi"),
            Some("audio/ogg") => Some(".ogg"),
            Some("audio/wav") | Some("audio/x-wav") => Some(".wav"),
            Some("image/png") => Some(".png"),
            Some("image/jpeg") | Some("image/jpg") => Some(".jpg"),
            Some("image/gif") => Some(".gif"),
            Some("image/bmp") => Some(".bmp"),
            Some("image/webp") => Some(".webp"),
            Some("video/mp4") => Some(".mp4"),
            Some("video/webm") => Some(".webm"),
            Some("application/zip") => Some(".zip"),
            Some("text/plain") => Some(".txt"),
            _ => {
                // Special case: DMI files are PNGs but may not be detected by infer crate
                // If entry type is DMI_PNG and we couldn't infer the MIME type, assume .dmi
                if self.base_type() == TYPE_DMI_PNG {
                    Some(".dmi")
                } else {
                    None
                }
            }
        }
    }

    /// Check if the inferred MIME type matches the declared entry type
    ///
    /// Returns true if there's a mismatch, false if they match or if MIME type couldn't be inferred.
    pub fn has_type_mismatch(&self) -> bool {
        if let Some(ref mime) = self.inferred_mime {
            let base_type = self.base_type();
            match base_type {
                TYPE_MIDI => mime != "audio/midi",
                TYPE_OGG_WAV => !matches!(mime.as_str(), "audio/ogg" | "audio/wav" | "audio/x-wav"),
                TYPE_DMI_PNG | TYPE_PNG => mime != "image/png",
                TYPE_JPG => !matches!(mime.as_str(), "image/jpeg" | "image/jpg"),
                _ => false, // Unknown types don't mismatch
            }
        } else {
            false
        }
    }
}

/// Convert an RSC entry type to a human-readable string.
///
/// Automatically strips the encryption flag (0x80) before matching.
pub fn entry_type_to_str(entry_type: u8) -> &'static str {
    match entry_type & !ENCRYPTION_FLAG {
        TYPE_MIDI => "MIDI",
        TYPE_OGG_WAV => "OGG/WAV",
        TYPE_DMI_PNG => "DMI PNG",
        TYPE_PNG => "PNG",
        TYPE_JPG => "JPG",
        _ => "Unknown",
    }
}

/// Format a Unix timestamp as a UTC date string
fn format_unix_timestamp(timestamp: u32) -> String {
    DateTime::from_timestamp(timestamp as i64, 0)
        .map(|dt: DateTime<Utc>| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| format!("{} (invalid)", timestamp))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use binrw::BinRead;

    use super::*;

    #[test]
    fn test_rad_entry_size_limit_enforced() {
        // Create a RAD entry that exceeds MAX_ENTRY_SIZE
        // Format: u32 entry_length, u8 valid, then content
        let oversized_length = MAX_ENTRY_SIZE + 1;
        let mut data = Vec::new();

        // Write entry_length (little-endian u32)
        data.extend_from_slice(&oversized_length.to_le_bytes());
        // Write valid flag
        data.push(0x01);
        // Note: We don't need to write actual content since parsing should fail before reading it

        let mut cursor = Cursor::new(data);
        let result = RadEntry::read(&mut cursor);

        // Should fail with assertion error
        assert!(result.is_err());
        let error_msg = format!("{:?}", result.unwrap_err());
        assert!(error_msg.contains("Entry too large") || error_msg.contains("assertion"));
    }

    #[test]
    fn test_rad_entry_size_limit_at_boundary() {
        // Create a RAD entry at exactly MAX_ENTRY_SIZE (should succeed)
        let max_length = MAX_ENTRY_SIZE;
        let mut data = Vec::new();

        // Write entry_length (little-endian u32)
        data.extend_from_slice(&max_length.to_le_bytes());
        // Write valid flag
        data.push(0x01);
        // Write content (all zeros for simplicity)
        data.extend(vec![0u8; max_length as usize]);

        let mut cursor = Cursor::new(data);
        let result = RadEntry::read(&mut cursor);

        // Should succeed
        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(entry.entry_length, MAX_ENTRY_SIZE);
    }

    #[test]
    fn test_rsc_entry_size_limit_enforced() {
        // Create an RSC entry with data_length exceeding MAX_ENTRY_SIZE
        let oversized_length = MAX_ENTRY_SIZE + 1;
        let mut data = Vec::new();

        // Write entry_type
        data.push(0x03); // DMI PNG
        // Write unique_id (u32)
        data.extend_from_slice(&1234u32.to_le_bytes());
        // Write timestamp (u32)
        data.extend_from_slice(&1234567890u32.to_le_bytes());
        // Write original_timestamp (u32)
        data.extend_from_slice(&0u32.to_le_bytes());
        // Write data_length (u32) - oversized
        data.extend_from_slice(&oversized_length.to_le_bytes());

        let mut cursor = Cursor::new(data);
        let result = RscEntry::read(&mut cursor);

        // Should fail with assertion error
        assert!(result.is_err());
        let error_msg = format!("{:?}", result.unwrap_err());
        assert!(error_msg.contains("Data too large") || error_msg.contains("assertion"));
    }

    #[test]
    fn test_extension_from_mime() {
        // Test common MIME types
        let mut entry = RscEntry {
            entry_type: TYPE_OGG_WAV,
            unique_id: 123,
            timestamp: 0,
            original_timestamp: 0,
            data_length: 0,
            filename: NullString::default(),
            data: vec![],
            inferred_mime: None,
        };

        // No MIME type inferred
        assert_eq!(entry.extension_from_mime(), None);

        // OGG
        entry.inferred_mime = Some("audio/ogg".to_string());
        assert_eq!(entry.extension_from_mime(), Some(".ogg"));

        // WAV
        entry.inferred_mime = Some("audio/wav".to_string());
        assert_eq!(entry.extension_from_mime(), Some(".wav"));

        // PNG
        entry.inferred_mime = Some("image/png".to_string());
        assert_eq!(entry.extension_from_mime(), Some(".png"));

        // JPEG
        entry.inferred_mime = Some("image/jpeg".to_string());
        assert_eq!(entry.extension_from_mime(), Some(".jpg"));

        // MIDI
        entry.inferred_mime = Some("audio/midi".to_string());
        assert_eq!(entry.extension_from_mime(), Some(".midi"));

        // Unknown MIME type
        entry.inferred_mime = Some("application/unknown".to_string());
        assert_eq!(entry.extension_from_mime(), None);

        // Special case: DMI PNG with no inferred MIME type should return .dmi
        entry.entry_type = TYPE_DMI_PNG;
        entry.inferred_mime = None;
        assert_eq!(entry.extension_from_mime(), Some(".dmi"));

        // DMI PNG with unknown MIME type should also return .dmi
        entry.inferred_mime = Some("application/unknown".to_string());
        assert_eq!(entry.extension_from_mime(), Some(".dmi"));

        // DMI PNG with PNG MIME type should return .png (inferred type takes precedence)
        entry.inferred_mime = Some("image/png".to_string());
        assert_eq!(entry.extension_from_mime(), Some(".png"));
    }
}
