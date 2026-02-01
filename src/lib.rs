pub mod app;
pub mod format;

use chrono::{DateTime, Local};
use parquet::basic::{ConvertedType, LogicalType, TimeUnit};
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::file::statistics::Statistics;
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
    pub num_rows: u64,
    pub num_columns: usize,
    pub version: i32,
    pub created_by: Option<String>,
    pub num_row_groups: usize,
    pub schema: String,
    pub schema_tree: SchemaField,
    pub key_value_metadata: Vec<(String, String)>,
    pub column_orders: String,
}

#[derive(Debug, Clone)]
pub struct RowGroupData {
    pub index: usize,
    pub num_rows: i64,
    pub total_byte_size: i64,
    pub compressed_size: i64,
    pub sorting_columns: Vec<SortingColumnInfo>,
    pub columns: Vec<ColumnChunkInfo>,
}

#[derive(Debug, Clone)]
pub struct SortingColumnInfo {
    pub column_path: String,
    pub descending: bool,
    pub nulls_first: bool,
}

#[derive(Debug, Clone)]
pub struct ColumnChunkInfo {
    pub name: String,
    pub physical_type: String,
    pub logical_type: Option<String>,
    pub converted_type: Option<String>,
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
                num_rows: pq_file_meta.num_rows() as u64,
                num_columns: schema.num_columns(),
                version: pq_file_meta.version(),
                created_by: pq_file_meta.created_by().map(|s| s.to_string()),
                num_row_groups: pq_meta.num_row_groups(),
                schema: format!("{:?}", pq_file_meta.schema()),
                schema_tree: SchemaField::from_type(schema.root_schema()),
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
                            let stats = col.statistics().map(|s| {
                                let (min, max) = format_statistics_min_max(s);
                                ColumnStatistics {
                                    min,
                                    max,
                                    null_count: s.null_count_opt(),
                                    distinct_count: s.distinct_count_opt(),
                                }
                            });

                            let converted_type = match col.column_descr().converted_type() {
                                ConvertedType::NONE => None,
                                t => Some(t.to_string()),
                            };

                            ColumnChunkInfo {
                                name: col.column_descr().name().to_string(),
                                physical_type: col.column_descr().physical_type().to_string(),
                                logical_type: col.column_descr().logical_type_ref().map(pq_logical_type_to_string),
                                converted_type,
                                encodings: col
                                    .encodings()
                                    .map(|e| e.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                compression: format!("{:?}", col.compression()),
                                total_compressed_size: col.compressed_size(),
                                total_uncompressed_size: col.uncompressed_size(),
                                num_values: col.num_values(),
                                statistics: stats,
                                sort_order: col.column_descr().sort_order().to_string(),
                            }
                        })
                        .collect();

                    let sorting_columns = rg
                        .sorting_columns()
                        .map(|cols| {
                            cols.iter()
                                .map(|sc| {
                                    let column_path =
                                        schema.column(sc.column_idx as usize).path().string();
                                    SortingColumnInfo {
                                        column_path,
                                        descending: sc.descending,
                                        nulls_first: sc.nulls_first,
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    RowGroupData {
                        index,
                        num_rows: rg.num_rows(),
                        total_byte_size: rg.total_byte_size(),
                        compressed_size: rg.compressed_size(),
                        sorting_columns,
                        columns,
                    }
                })
                .collect(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SchemaField {
    pub id: Option<i32>,
    pub name: String,
    pub _type: SchemaType,
    pub repetition: Option<String>,
    pub converted_type: Option<String>,
    pub logical_type: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SchemaType {
    Primitive(String),
    Group(Vec<SchemaField>),
}

impl SchemaField {
    fn from_type(field: &Type) -> Self {
        let name = field.name().to_string();
        let basic_info = field.get_basic_info();
        let id = if basic_info.has_id() {
            Some(basic_info.id())
        } else {
            None
        };

        let converted_type = match basic_info.converted_type() {
            ConvertedType::NONE => None,
            t => Some(t.to_string()),
        };

        let logical_type = basic_info
            .logical_type_ref()
            .map(|lt| pq_logical_type_to_string(lt));

        let repetition = if basic_info.has_repetition() {
            Some(basic_info.repetition().to_string())
        } else {
            None
        };

        let _type = match field {
            Type::PrimitiveType {
                physical_type,
                type_length,
                scale,
                precision,
                ..
            } => {
                let mut parts = vec![];
                if *type_length >= 0 {
                    parts.push(format!("length = {type_length}"));
                }
                if *precision >= 0 {
                    parts.push(format!("precision = {precision}"));
                }
                if *scale >= 0 {
                    parts.push(format!("scale = {scale}"));
                }
                let description = if parts.is_empty() {
                    format!("{physical_type}")
                } else {
                    format!("{physical_type}({})", parts.join(", "))
                };
                SchemaType::Primitive(description)
            }
            Type::GroupType { fields, .. } => {
                SchemaType::Group(fields.iter().map(|t| Self::from_type(t)).collect())
            }
        };

        SchemaField {
            id,
            name,
            _type,
            repetition,
            converted_type,
            logical_type,
        }
    }
}

fn pq_logical_type_to_string(lt: &LogicalType) -> String {
    match lt {
        LogicalType::String => "string".into(),
        LogicalType::Map => "map".into(),
        LogicalType::List => "list".into(),
        LogicalType::Enum => "enum".into(),
        LogicalType::Decimal { scale, precision } => format!("decimal({},{})", precision, scale),
        LogicalType::Date => "date".into(),
        LogicalType::Time {
            is_adjusted_to_u_t_c,
            unit,
        } => logical_type_time_to_string("time", *is_adjusted_to_u_t_c, unit),
        LogicalType::Timestamp {
            is_adjusted_to_u_t_c,
            unit,
        } => logical_type_time_to_string("timestamp", *is_adjusted_to_u_t_c, unit),
        LogicalType::Integer {
            bit_width,
            is_signed,
        } => {
            let signed = if *is_signed { "signed" } else { "unsigned" };
            format!("int({}bit, {})", bit_width, signed)
        }
        LogicalType::Unknown => "unknown".into(),
        LogicalType::Json => "json".into(),
        LogicalType::Bson => "bson".into(),
        LogicalType::Uuid => "uuid".into(),
        LogicalType::Float16 => "float16".into(),
        LogicalType::Variant {
            specification_version,
        } => {
            let specification = specification_version
                .map(|v| format!("(specification_version = {})", v))
                .unwrap_or_default();
            format!("variant{}", specification)
        }
        LogicalType::Geometry { crs } => {
            let crs = crs
                .as_ref()
                .map(|c| format!("(crs = {})", c))
                .unwrap_or_default();
            format!("geometry{}", crs)
        }
        LogicalType::Geography { crs, algorithm } => {
            let crs = crs.as_ref().map(|c| format!("crs = {}", c));
            let algorithm = algorithm.map(|a| format!("algorithm = {}", a));
            let info = crs
                .into_iter()
                .chain(algorithm)
                .collect::<Vec<_>>()
                .join(", ");
            let info = if info.is_empty() {
                ""
            } else {
                &format!("({info})")
            };
            format!("geography{info}")
        }
        LogicalType::_Unknown { field_id } => {
            format!("unknown(filed_id = {})", field_id)
        }
    }
}

fn logical_type_time_to_string(
    base_type: &str,
    is_adjusted_to_utc: bool,
    unit: &TimeUnit,
) -> String {
    let unit = match unit {
        TimeUnit::MILLIS => "millis",
        TimeUnit::MICROS => "micros",
        TimeUnit::NANOS => "nanos",
    };
    format!(
        "{base_type}({unit}, {}adjusted to UTC)",
        if is_adjusted_to_utc { "" } else { "not " }
    )
}

fn format_statistics_min_max(stats: &Statistics) -> (Option<String>, Option<String>) {
    match stats {
        Statistics::Boolean(s) => (
            s.min_opt().map(|v| v.to_string()),
            s.max_opt().map(|v| v.to_string()),
        ),
        Statistics::Int32(s) => (
            s.min_opt().map(|v| v.to_string()),
            s.max_opt().map(|v| v.to_string()),
        ),
        Statistics::Int64(s) => (
            s.min_opt().map(|v| v.to_string()),
            s.max_opt().map(|v| v.to_string()),
        ),
        Statistics::Int96(s) => (
            s.min_opt().map(|v| format!("{:?}", v)),
            s.max_opt().map(|v| format!("{:?}", v)),
        ),
        Statistics::Float(s) => (
            s.min_opt().map(|v| v.to_string()),
            s.max_opt().map(|v| v.to_string()),
        ),
        Statistics::Double(s) => (
            s.min_opt().map(|v| v.to_string()),
            s.max_opt().map(|v| v.to_string()),
        ),
        Statistics::ByteArray(s) => (
            s.min_opt().map(|v| {
                String::from_utf8(v.data().to_vec()).unwrap_or_else(|_| format!("{:?}", v.data()))
            }),
            s.max_opt().map(|v| {
                String::from_utf8(v.data().to_vec()).unwrap_or_else(|_| format!("{:?}", v.data()))
            }),
        ),
        Statistics::FixedLenByteArray(s) => (
            s.min_opt().map(|v| {
                String::from_utf8(v.data().to_vec()).unwrap_or_else(|_| format!("{:?}", v.data()))
            }),
            s.max_opt().map(|v| {
                String::from_utf8(v.data().to_vec()).unwrap_or_else(|_| format!("{:?}", v.data()))
            }),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::pq_logical_type_to_string;
    use parquet::basic::LogicalType;

    #[test]
    fn logical_type_to_string() {
        assert_eq!(
            pq_logical_type_to_string(&LogicalType::Date),
            "date".to_string()
        );
    }
}
