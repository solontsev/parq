pub mod app;
pub mod format;

use chrono::{DateTime, Local};
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::schema::types::Type;
use std::fs::File;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    ParquetError(#[from] parquet::errors::ParquetError),
}

#[derive(Debug, Clone)]
pub struct ParquetFileData {
    pub file_meta: FileMetadata,
    pub metadata: ParquetFileMetadata,
    pub row_groups_data: Vec<RowGroupData>,
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub name: String,
    pub created: DateTime<Local>,
    pub modified: DateTime<Local>,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct ParquetFileMetadata {
    pub num_rows: i64,
    pub version: i32,
    pub created_by: Option<String>,
    pub num_row_groups: usize,
    pub schema: String,
    pub schema_tree: SchemaNode,
    pub key_value_metadata: Vec<(String, String)>,
    pub column_orders: String,
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
pub struct RowGroupData {
    pub index: usize,
    pub num_rows: i64,
    pub total_byte_size: i64,
    pub compressed_size: i64,
    pub sorting_columns: String,
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
    pub sort_order: String,
}

#[derive(Debug, Clone)]
pub struct ColumnStatistics {
    pub min: Option<String>,
    pub max: Option<String>,
    pub null_count: Option<u64>,
    pub distinct_count: Option<u64>,
}

impl ParquetFileData {
    pub fn new(file_path: &str) -> Result<Self, AppError> {
        let file = File::open(file_path)?;

        // extract basic file metadata
        let file_meta = file.metadata()?;
        let created: DateTime<Local> = file_meta.created()?.into();
        let modified: DateTime<Local> = file_meta.modified()?.into();
        let size = file_meta.len();

        let reader = SerializedFileReader::new(file)?;

        let pq_meta = reader.metadata();
        let pq_file_meta = pq_meta.file_metadata();
        let schema = pq_file_meta.schema_descr();

        Ok(Self {
            file_meta: FileMetadata {
                name: file_path.to_owned(),
                created,
                modified,
                size,
            },
            metadata: ParquetFileMetadata {
                num_rows: pq_file_meta.num_rows(),
                version: pq_file_meta.version(),
                created_by: pq_file_meta.created_by().map(|s| s.to_string()),
                num_row_groups: pq_meta.num_row_groups(),
                schema: format!("{:?}", pq_file_meta.schema()),
                schema_tree: SchemaNode::from_type(schema.root_schema()),
                key_value_metadata: pq_file_meta
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
                column_orders: format!("{:#?}", pq_file_meta.column_orders()),
            },
            row_groups_data: pq_meta
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
                                sort_order: format!("{:?}", col.column_descr().sort_order()),
                            }
                        })
                        .collect();

                    RowGroupData {
                        index,
                        num_rows: rg.num_rows(),
                        total_byte_size: rg.total_byte_size(),
                        compressed_size: rg.compressed_size(),
                        sorting_columns: format!("{:#?}", rg.sorting_columns()),
                        columns,
                    }
                })
                .collect(),
        })
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
