use anyhow::{Context, Result};
use flate2::read::MultiGzDecoder;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastaRecord {
    pub id: String,
    pub header: String,
    pub sequence: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FastaEvent<'a> {
    StartRecord {
        id: String,
        header: String,
        line_number: usize,
    },
    SequenceLine {
        bytes: &'a [u8],
        line_number: usize,
    },
    EndRecord,
}

#[derive(Debug, Error)]
pub enum FastaParseError {
    #[error("sequence before first header at line {line_number}")]
    SequenceBeforeFirstHeader { line_number: usize },
    #[error("empty FASTA header at line {line_number}")]
    EmptyHeader { line_number: usize },
    #[error("empty FASTA record for {id} at line {line_number}")]
    EmptyRecord { id: String, line_number: usize },
    #[error("input contains no FASTA records")]
    NoRecords,
}

pub fn is_structural_fasta_error(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| cause.is::<FastaParseError>())
}

// Convenience collector for tests and early internal use. Production orchestration should prefer
// `for_each_fasta_record`, or `for_each_fasta_event` when it must avoid per-record sequence bodies.
pub fn read_fasta(path: &Path) -> Result<Vec<FastaRecord>> {
    let mut records = Vec::new();
    for_each_fasta_record(path, |record| {
        records.push(record);
        Ok(())
    })?;
    Ok(records)
}

pub fn for_each_fasta_record<F>(path: &Path, visitor: F) -> Result<()>
where
    F: FnMut(FastaRecord) -> Result<()>,
{
    let reader = open_fasta_reader(path)?;
    parse_records_from_events(reader, visitor)
}

pub fn for_each_fasta_event<F>(path: &Path, visitor: F) -> Result<()>
where
    F: for<'a> FnMut(FastaEvent<'a>) -> Result<()>,
{
    let reader = open_fasta_reader(path)?;
    parse_events(reader, visitor)
}

fn open_fasta_reader(path: &Path) -> Result<BufReader<Box<dyn Read>>> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let reader: Box<dyn Read> = if path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("gz"))
        .unwrap_or(false)
    {
        Box::new(MultiGzDecoder::new(file))
    } else {
        Box::new(file)
    };

    Ok(BufReader::new(reader))
}

fn parse_records_from_events<R, F>(reader: R, mut visitor: F) -> Result<()>
where
    R: BufRead,
    F: FnMut(FastaRecord) -> Result<()>,
{
    let mut current_id = String::new();
    let mut current_header = String::new();
    let mut current_sequence: Vec<u8> = Vec::new();

    parse_events(reader, |event| {
        match event {
            FastaEvent::StartRecord { id, header, .. } => {
                current_id = id;
                current_header = header;
            }
            FastaEvent::SequenceLine { bytes, .. } => current_sequence.extend(bytes),
            FastaEvent::EndRecord => {
                visitor(FastaRecord {
                    id: std::mem::take(&mut current_id),
                    header: std::mem::take(&mut current_header),
                    sequence: std::mem::take(&mut current_sequence),
                })?;
            }
        }

        Ok(())
    })
}

fn parse_events<R, F>(reader: R, mut visitor: F) -> Result<()>
where
    R: BufRead,
    F: for<'a> FnMut(FastaEvent<'a>) -> Result<()>,
{
    let mut current_id: Option<String> = None;
    let mut current_header_line: Option<usize> = None;
    let mut current_has_sequence = false;
    let mut record_count = 0usize;

    for (line_index, line_result) in reader.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line_result.with_context(|| format!("failed to read line {line_number}"))?;
        let trimmed = line.trim_end_matches('\r');

        if let Some(header_text) = trimmed.strip_prefix('>') {
            close_current_event_record(
                &mut current_id,
                current_header_line.take(),
                &mut current_has_sequence,
                &mut record_count,
                &mut visitor,
            )?;

            let header = header_text.trim().to_string();
            if header.is_empty() {
                return Err(FastaParseError::EmptyHeader { line_number }.into());
            }
            let id = header
                .split_whitespace()
                .next()
                .ok_or(FastaParseError::EmptyHeader { line_number })?
                .to_string();
            current_id = Some(id);
            current_header_line = Some(line_number);
            visitor(FastaEvent::StartRecord {
                id: current_id.as_ref().unwrap().clone(),
                header,
                line_number,
            })?;
        } else if trimmed.is_empty() {
            continue;
        } else {
            if current_id.is_none() {
                return Err(FastaParseError::SequenceBeforeFirstHeader { line_number }.into());
            }
            current_has_sequence = true;
            visitor(FastaEvent::SequenceLine {
                bytes: trimmed.as_bytes(),
                line_number,
            })?;
        }
    }

    close_current_event_record(
        &mut current_id,
        current_header_line.take(),
        &mut current_has_sequence,
        &mut record_count,
        &mut visitor,
    )?;

    if record_count == 0 {
        return Err(FastaParseError::NoRecords.into());
    }

    Ok(())
}

fn close_current_event_record<F>(
    current_id: &mut Option<String>,
    current_header_line: Option<usize>,
    current_has_sequence: &mut bool,
    record_count: &mut usize,
    visitor: &mut F,
) -> Result<()>
where
    F: for<'a> FnMut(FastaEvent<'a>) -> Result<()>,
{
    let Some(id) = current_id.take() else {
        return Ok(());
    };

    if !*current_has_sequence {
        let header_line = current_header_line.unwrap_or_default();
        return Err(FastaParseError::EmptyRecord {
            id,
            line_number: header_line,
        }
        .into());
    }

    visitor(FastaEvent::EndRecord)?;
    *record_count += 1;
    *current_has_sequence = false;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    #[test]
    fn reads_multirecord_fasta() {
        let records = read_fasta(Path::new("testdata/valid_assembly.fa")).unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].id, "contig_1");
        assert_eq!(records[1].id, "contig_2");
        assert_eq!(records[0].sequence, b"ACGTACGTACGTAAAA");
    }

    #[test]
    fn streams_records_to_visitor() {
        let mut ids = Vec::new();
        for_each_fasta_record(Path::new("testdata/valid_assembly.fa"), |record| {
            ids.push(record.id);
            Ok(())
        })
        .unwrap();

        assert_eq!(ids, ["contig_1", "contig_2", "contig_3"]);
    }

    #[test]
    fn streams_fasta_events_without_collecting_sequence_records() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.fa");
        std::fs::write(&path, ">first description\n ACGT\t\r\n\nNN\n>second\nUU\n").unwrap();

        let mut events = Vec::new();
        for_each_fasta_event(&path, |event| {
            match event {
                FastaEvent::StartRecord {
                    id,
                    header,
                    line_number,
                } => events.push(format!("start:{id}:{header}:{line_number}")),
                FastaEvent::SequenceLine { bytes, line_number } => {
                    events.push(format!(
                        "seq:{}:{line_number}",
                        String::from_utf8_lossy(bytes)
                    ));
                }
                FastaEvent::EndRecord => events.push("end".to_string()),
            }
            Ok(())
        })
        .unwrap();

        assert_eq!(
            events,
            [
                "start:first:first description:1",
                "seq: ACGT\t:2",
                "seq:NN:4",
                "end",
                "start:second:second:5",
                "seq:UU:6",
                "end",
            ]
        );
    }

    #[test]
    fn event_parser_rejects_empty_records() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.fa");
        std::fs::write(&path, ">first\n>second\nACGT\n").unwrap();

        let error = for_each_fasta_event(&path, |_| Ok(()))
            .unwrap_err()
            .to_string();

        assert!(error.contains("empty FASTA record"));
        assert!(error.contains("first"));
    }

    #[test]
    fn rejects_sequence_before_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.fa");
        std::fs::write(&path, "ACGT\n>later\nACGT\n").unwrap();
        let error = read_fasta(&path).unwrap_err().to_string();
        assert!(error.contains("sequence before first header"));
    }

    #[test]
    fn rejects_empty_header_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.fa");
        std::fs::write(&path, ">\nACGT\n").unwrap();
        let error = read_fasta(&path).unwrap_err().to_string();
        assert!(error.contains("empty FASTA header"));
    }

    #[test]
    fn rejects_consecutive_headers_as_empty_record() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.fa");
        std::fs::write(&path, ">first\n>second\nACGT\n").unwrap();
        let error = read_fasta(&path).unwrap_err().to_string();
        assert!(error.contains("empty FASTA record"));
        assert!(error.contains("first"));
    }

    #[test]
    fn rejects_final_header_only_record() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.fa");
        std::fs::write(&path, ">first\nACGT\n>empty\n").unwrap();
        let error = read_fasta(&path).unwrap_err().to_string();
        assert!(error.contains("empty FASTA record"));
        assert!(error.contains("empty"));
    }

    #[test]
    fn preserves_sequence_whitespace_for_downstream_validation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("spaces.fa");
        std::fs::write(&path, ">spaces\n ACGT\t\n\nNNNN\n").unwrap();

        let records = read_fasta(&path).unwrap();

        assert_eq!(records[0].sequence, b" ACGT\tNNNN");
    }

    #[test]
    fn reads_gzip_fasta() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("assembly.fa.gz");
        let file = std::fs::File::create(&path).unwrap();
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder.write_all(b">gz_contig\nACGT\n").unwrap();
        encoder.finish().unwrap();

        let mut records = Vec::new();
        for_each_fasta_record(&path, |record| {
            records.push(record);
            Ok(())
        })
        .unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "gz_contig");
        assert_eq!(records[0].sequence, b"ACGT");
    }
}
