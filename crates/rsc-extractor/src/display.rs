use bytesize::ByteSize;

use crate::types::RscEntry;
use crate::{oprint, oprintln};

/// Print information about an RSC entry
pub fn print_rsc_entry(entry_num: usize, rsc_entry: &RscEntry) {
    let encryption_status = if rsc_entry.is_encrypted() {
        " [ENCRYPTED]"
    } else {
        ""
    };

    oprintln!(
        "Entry {}: {} (0x{:02X}){}",
        entry_num,
        rsc_entry.type_str(),
        rsc_entry.base_type(),
        encryption_status
    );
    oprintln!("  Filename:    \"{}\"", rsc_entry.filename);
    oprintln!("  Unique ID:   0x{:08X}", rsc_entry.unique_id);
    oprintln!(
        "  Data length: {}",
        ByteSize::b(rsc_entry.data_length as u64)
    );
    oprintln!(
        "  Timestamp:   {} ({})",
        rsc_entry.timestamp,
        rsc_entry.timestamp_str()
    );
    oprintln!(
        "  Original:    {} ({})",
        rsc_entry.original_timestamp,
        rsc_entry.original_timestamp_str()
    );

    // Display inferred MIME type
    if let Some(ref mime) = rsc_entry.inferred_mime {
        oprint!("  Inferred:    {}", mime);
        if rsc_entry.has_type_mismatch() {
            oprintln!(" ⚠️  [TYPE MISMATCH]");
        } else {
            oprintln!();
        }
    } else {
        oprintln!("  Inferred:    <unknown>");
    }

    oprintln!();
}

/// Print summary statistics
pub fn print_summary(
    total: usize,
    valid: usize,
    invalid: usize,
    encrypted: usize,
    invalid_entry_numbers: &[usize],
) {
    oprintln!();
    oprintln!("Summary:");
    oprintln!("  Total entries: {}", total);
    oprintln!("  Valid entries:     {}", valid);
    oprintln!("  Invalid entries:   {}", invalid);
    oprintln!("  Encrypted entries: {}", encrypted);

    // Display invalid entry numbers if there are any
    if !invalid_entry_numbers.is_empty() {
        oprintln!();
        oprintln!("Invalid entry numbers:");
        for &entry_num in invalid_entry_numbers {
            oprintln!("  • Entry {}", entry_num);
        }
    }
}
