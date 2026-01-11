pub mod app;

use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::schema::types::Type;
use std::fs::File;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to extract parquet metadata: {0}")]
    ParquetError(#[from] parquet::errors::ParquetError),
}

#[derive(Debug, Clone)]
pub struct ParquetFileInfo {
    pub meta_info: ParquetFileMetaInfo,
    pub row_groups_info: Vec<RowGroupInfo>,
}

#[derive(Debug, Clone)]
pub struct ParquetFileMetaInfo {
    pub num_rows: i64,
    pub version: i32,
    pub created_by: Option<String>,
    pub num_row_groups: usize,
    pub schema: String,
    pub schema_tree: SchemaNode,
    pub key_value_metadata: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct SchemaNode {
    pub name: String,
    pub type_name: String,
    pub repetition: String,
    pub converted_type: Option<String>,
    pub logical_type: Option<String>,
    pub children: Vec<SchemaNode>,
}

#[derive(Debug, Clone)]
pub struct RowGroupInfo {
    pub index: usize,
    pub num_rows: i64,
    pub total_byte_size: i64,
    pub columns: Vec<ColumnChunkInfo>,
}

#[derive(Debug, Clone)]
pub struct ColumnChunkInfo {
    pub name: String,
    pub column_type: String,
    pub encodings: String,
    pub compression: String,
    pub total_compressed_size: i64,
    pub total_uncompressed_size: i64,
    pub num_values: i64,
    pub statistics: Option<ColumnStatistics>,
}

#[derive(Debug, Clone)]
pub struct ColumnStatistics {
    pub min: Option<String>,
    pub max: Option<String>,
    pub null_count: Option<u64>,
    pub distinct_count: Option<u64>,
}

impl ParquetFileInfo {
    pub fn new(file_path: &str) -> Result<Self, AppError> {
        let file = File::open(file_path)?;
        let reader = SerializedFileReader::new(file)?;
        Ok(Self::from(&reader))
    }
}

impl SchemaNode {
    fn from_type(field: &Type) -> Self {
        let name = field.name().to_string();
        let basic_info = field.get_basic_info();

        let (type_name, repetition) = match field {
            Type::PrimitiveType { physical_type, .. } => {
                let rep = if basic_info.has_repetition() {
                    format!("{:?}", basic_info.repetition())
                } else {
                    "REQUIRED".to_string()
                };
                (format!("{:?}", physical_type), rep)
            }
            Type::GroupType { .. } => {
                let rep = if basic_info.has_repetition() {
                    format!("{:?}", basic_info.repetition())
                } else {
                    "".to_string()
                };
                ("GROUP".to_string(), rep)
            }
        };

        let converted_type_val = basic_info.converted_type();
        let converted_type = if format!("{:?}", converted_type_val) == "NONE" {
            None
        } else {
            Some(format!("{:?}", converted_type_val))
        };
        let logical_type = basic_info.logical_type_ref().map(|lt| format!("{:?}", lt));

        let children = if let Type::GroupType { fields, .. } = field {
            fields.iter().map(|f| SchemaNode::from_type(f)).collect()
        } else {
            vec![]
        };

        SchemaNode {
            name,
            type_name,
            repetition,
            converted_type,
            logical_type,
            children,
        }
    }
}

impl std::fmt::Display for ParquetFileMetaInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  Total rows: {}", self.num_rows)?;
        writeln!(f, "  Version: {}", self.version)?;
        if let Some(created_by) = &self.created_by {
            writeln!(f, "  Created by: {}", created_by)?;
        }
        if !self.key_value_metadata.is_empty() {
            writeln!(f, "  K/V metadata: {:?}", self.key_value_metadata)?;
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
        let schema = file_meta.schema_descr();

        ParquetFileInfo {
            meta_info: ParquetFileMetaInfo {
                num_rows: file_meta.num_rows(),
                version: file_meta.version(),
                created_by: file_meta.created_by().map(|s| s.to_string()),
                num_row_groups: meta.num_row_groups(),
                schema: format!("{:?}", file_meta.schema()),
                schema_tree: SchemaNode::from_type(schema.root_schema()),
                key_value_metadata: file_meta
                    .key_value_metadata()
                    .map(|kv| {
                        kv.iter()
                            .map(|kv_pair| {
                                (
                                    kv_pair.key.clone(),
                                    kv_pair.value.clone().unwrap_or_default(),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            },
            row_groups_info: meta
                .row_groups()
                .iter()
                .enumerate()
                .map(|(index, rg)| {
                    let columns = rg
                        .columns()
                        .iter()
                        .map(|col| {
                            let stats = col.statistics().map(|s| ColumnStatistics {
                                min: s.min_bytes_opt().map(|b| format!("{:?}", b)),
                                max: s.max_bytes_opt().map(|b| format!("{:?}", b)),
                                null_count: s.null_count_opt(),
                                distinct_count: s.distinct_count_opt(),
                            });

                            ColumnChunkInfo {
                                name: col.column_descr().name().to_string(),
                                column_type: format!("{:?}", col.column_descr().physical_type()),
                                encodings: col
                                    .encodings()
                                    .map(|e| format!("{:?}", e))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                compression: format!("{:?}", col.compression()),
                                total_compressed_size: col.compressed_size(),
                                total_uncompressed_size: col.uncompressed_size(),
                                num_values: col.num_values(),
                                statistics: stats,
                            }
                        })
                        .collect();

                    RowGroupInfo {
                        index,
                        num_rows: rg.num_rows(),
                        total_byte_size: rg.total_byte_size(),
                        columns,
                    }
                })
                .collect(),
        }
    }
}
