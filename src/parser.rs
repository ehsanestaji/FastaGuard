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

// Convenience collector for tests and early internal use. Production orchestration should
// prefer `for_each_fasta_record` to avoid retaining every record in memory.
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

    parse_reader(BufReader::new(reader), visitor)
}

fn parse_reader<R, F>(reader: R, mut visitor: F) -> Result<()>
where
    R: BufRead,
    F: FnMut(FastaRecord) -> Result<()>,
{
    let mut current_header: Option<String> = None;
    let mut current_id: Option<String> = None;
    let mut current_sequence: Vec<u8> = Vec::new();
    let mut current_header_line: Option<usize> = None;
    let mut record_count = 0usize;

    for (line_index, line_result) in reader.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line_result.with_context(|| format!("failed to read line {line_number}"))?;
        let trimmed = line.trim_end_matches('\r');

        if let Some(header_text) = trimmed.strip_prefix('>') {
            if let Some(header) = current_header.take() {
                emit_record(
                    current_id.take().unwrap(),
                    header,
                    std::mem::take(&mut current_sequence),
                    current_header_line.take().unwrap(),
                    &mut record_count,
                    &mut visitor,
                )?;
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
            current_header_line = Some(line_number);
        } else if trimmed.is_empty() {
            continue;
        } else {
            if current_header.is_none() {
                return Err(anyhow!(
                    "sequence before first header at line {line_number}"
                ));
            }
            current_sequence.extend(trimmed.as_bytes());
        }
    }

    if let Some(header) = current_header.take() {
        emit_record(
            current_id.take().unwrap(),
            header,
            current_sequence,
            current_header_line.take().unwrap(),
            &mut record_count,
            &mut visitor,
        )?;
    }

    if record_count == 0 {
        return Err(anyhow!("input contains no FASTA records"));
    }

    Ok(())
}

fn emit_record<F>(
    id: String,
    header: String,
    sequence: Vec<u8>,
    header_line: usize,
    record_count: &mut usize,
    visitor: &mut F,
) -> Result<()>
where
    F: FnMut(FastaRecord) -> Result<()>,
{
    if sequence.is_empty() {
        return Err(anyhow!("empty FASTA record for {id} at line {header_line}"));
    }

    visitor(FastaRecord {
        id,
        header,
        sequence,
    })?;
    *record_count += 1;

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
