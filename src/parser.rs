use anyhow::{anyhow, Context, Result};
use flate2::read::MultiGzDecoder;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastaRecord {
    pub id: String,
    pub header: String,
    pub sequence: Vec<u8>,
}

pub fn read_fasta(path: &Path) -> Result<Vec<FastaRecord>> {
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

    parse_reader(BufReader::new(reader))
}

fn parse_reader<R: BufRead>(reader: R) -> Result<Vec<FastaRecord>> {
    let mut records = Vec::new();
    let mut current_header: Option<String> = None;
    let mut current_id: Option<String> = None;
    let mut current_sequence: Vec<u8> = Vec::new();

    for (line_index, line_result) in reader.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line_result.with_context(|| format!("failed to read line {line_number}"))?;
        let trimmed = line.trim_end_matches('\r');

        if let Some(header_text) = trimmed.strip_prefix('>') {
            if let Some(header) = current_header.take() {
                records.push(FastaRecord {
                    id: current_id.take().unwrap(),
                    header,
                    sequence: std::mem::take(&mut current_sequence),
                });
            }

            let header = header_text.trim().to_string();
            if header.is_empty() {
                return Err(anyhow!("empty FASTA header at line {line_number}"));
            }
            let id = header
                .split_whitespace()
                .next()
                .ok_or_else(|| anyhow!("empty FASTA header at line {line_number}"))?
                .to_string();
            current_header = Some(header);
            current_id = Some(id);
        } else if trimmed.trim().is_empty() {
            continue;
        } else {
            if current_header.is_none() {
                return Err(anyhow!(
                    "sequence before first header at line {line_number}"
                ));
            }
            current_sequence.extend(trimmed.trim().as_bytes());
        }
    }

    if let Some(header) = current_header.take() {
        records.push(FastaRecord {
            id: current_id.take().unwrap(),
            header,
            sequence: current_sequence,
        });
    }

    if records.is_empty() {
        return Err(anyhow!("input contains no FASTA records"));
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_multirecord_fasta() {
        let records = read_fasta(Path::new("testdata/valid_assembly.fa")).unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].id, "contig_1");
        assert_eq!(records[1].id, "contig_2");
        assert_eq!(records[0].sequence, b"ACGTACGTACGTAAAA");
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
}
