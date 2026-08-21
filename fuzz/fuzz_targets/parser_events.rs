#![no_main]

use fastaguard::parser::{for_each_fasta_event_from_reader, FastaEvent};
use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    let mut event_count = 0usize;
    let result = for_each_fasta_event_from_reader(Cursor::new(data), |event| {
        match event {
            FastaEvent::StartRecord { id, header, .. } => {
                let _ = (id.len(), header.len());
            }
            FastaEvent::SequenceLine { bytes, .. } => {
                let _ = bytes.len();
            }
            FastaEvent::EndRecord => {}
        }
        event_count = event_count.saturating_add(1);
        Ok(())
    });

    match result {
        Ok(()) => {
            let _ = event_count;
        }
        Err(error) => {
            let _ = (error.to_string(), fastaguard::parser::is_structural_fasta_error(&error));
        }
    }
});
