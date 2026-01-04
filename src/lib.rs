use parquet::file::reader::{FileReader, SerializedFileReader};
use std::fs::File;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to extract parquet metadata: {0}")]
    ParquetError(#[from] parquet::errors::ParquetError),
}

#[derive(Debug)]
pub struct ParquetFileInfo {
    meta_info: ParquetFileMetaInfo,
    row_groups_info: Vec<RowGroupInfo>,
}

#[derive(Debug)]
pub struct ParquetFileMetaInfo {
    num_rows: i64,
    version: i32,
    created_by: Option<String>,
    num_row_groups: usize,
    schema: String,
    key_value_metadata: Option<String>,
}

#[derive(Debug)]
pub struct RowGroupInfo {
    index: usize,
    num_rows: i64,
}

impl ParquetFileInfo {
    pub fn new(file_path: &str) -> Result<Self, AppError> {
        let file = File::open(file_path)?;
        let reader = SerializedFileReader::new(file)?;
        Ok(Self::from(&reader))
    }
}

impl std::fmt::Display for ParquetFileMetaInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  Total rows: {}", self.num_rows)?;
        writeln!(f, "  Version: {}", self.version)?;
        if let Some(created_by) = &self.created_by {
            writeln!(f, "  Created by: {}", created_by)?;
        }
        if let Some(kv_meta) = &self.key_value_metadata {
            writeln!(f, "  K/V metadata: {}", kv_meta)?;
        }
        writeln!(f, "  Schema: {}", self.schema)?;
        writeln!(f, "  Number of row groups: {}", self.num_row_groups)?;

        Ok(())
    }
}

impl std::fmt::Display for RowGroupInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  Row group {}", self.index)?;
        writeln!(f, "    Rows: {}", self.num_rows)?;
        Ok(())
    }
}

impl std::fmt::Display for ParquetFileInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "📄 FILE METADATA:")?;
        self.meta_info.fmt(f)?;
        writeln!(f)?;
        writeln!(f, "📦 ROW GROUPS:")?;
        self.row_groups_info.iter().try_for_each(|rg| {
            writeln!(f)?;
            rg.fmt(f)
        })?;

        Ok(())
    }
}

impl From<&SerializedFileReader<File>> for ParquetFileInfo {
    fn from(reader: &SerializedFileReader<File>) -> Self {
        let meta = reader.metadata();
        let file_meta = meta.file_metadata();

        ParquetFileInfo {
            meta_info: ParquetFileMetaInfo {
                num_rows: file_meta.num_rows(),
                version: file_meta.version(),
                created_by: file_meta.created_by().map(|s| s.to_string()),
                num_row_groups: meta.num_row_groups(),
                schema: format!("{:?}", file_meta.schema()),
                key_value_metadata: file_meta.key_value_metadata().map(|kv| format!("{:?}", kv)),
            },
            row_groups_info: meta
                .row_groups()
                .iter()
                .enumerate()
                .map(|(index, rg)| RowGroupInfo {
                    index,
                    num_rows: rg.num_rows(),
                })
                .collect(),
        }
    }
}
