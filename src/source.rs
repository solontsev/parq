use chrono::{DateTime, Local};
use object_store::ObjectStoreExt;
use object_store::aws::AmazonS3Builder;
use parquet::arrow::arrow_reader::ArrowReaderMetadata;
use parquet::arrow::async_reader::ParquetObjectReader;
use parquet::file::metadata::ParquetMetaData;
use parquet::file::reader::{FileReader, SerializedFileReader};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum SourceError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ParquetError(#[from] parquet::errors::ParquetError),
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
    #[error(transparent)]
    ObjectStoreError(#[from] object_store::Error),
    #[error("{0}")]
    InvalidUrl(String),
}

pub enum FileSource<'a> {
    Local(&'a Path),
    ObjectStore(Url),
}

impl<'a> FileSource<'a> {
    pub fn parse(path: &'a str) -> Result<Self, SourceError> {
        if path.starts_with("s3://") {
            let url = Url::parse(path)?;
            Ok(Self::ObjectStore(url))
        } else if let Some(stripped) = path.strip_prefix("file://") {
            let path = Path::new(stripped);
            Ok(Self::Local(path))
        } else {
            let path = Path::new(path);
            Ok(Self::Local(path))
        }
    }

    pub fn load(self) -> Result<SourceFile, SourceError> {
        match self {
            FileSource::Local(path) => load_from_local_disk(path),
            FileSource::ObjectStore(url) => load_from_object_store(&url),
        }
    }
}

fn load_from_local_disk(path: &Path) -> Result<SourceFile, SourceError> {
    let file = File::open(path)?;
    let file_meta = file.metadata()?;
    let size = file_meta.len();
    let created = file_meta.created().ok().map(Into::into);
    let modified = file_meta.modified().ok().map(Into::into);

    let reader = SerializedFileReader::new(file)?;
    let pq_meta = reader.metadata().clone(); // TODO: remove clone

    Ok(SourceFile {
        name: path.display().to_string(),
        size,
        created,
        modified,
        etag: None,
        pq_meta: Arc::new(pq_meta),
    })
}

fn load_from_object_store(url: &Url) -> Result<SourceFile, SourceError> {
    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(async {
        let bucket = url
            .host_str()
            .filter(|_| !url.path().is_empty() && url.path() != "/") // In case you pass some wrong url like s3://data.parquet
            .ok_or_else(|| SourceError::InvalidUrl(format!("no bucket in URL: {}", url)))?;
        let path = object_store::path::Path::from(url.path().trim_start_matches('/'));

        let store = AmazonS3Builder::from_env()
            .with_bucket_name(bucket)
            .with_allow_http(true)
            .with_virtual_hosted_style_request(false)
            .build()?;

        let meta = store.head(&path).await?;
        let store = Arc::new(store);
        let size = &meta.size;

        let mut reader = ParquetObjectReader::new(store, meta.location).with_file_size(*size);
        let arrow_metadata =
            ArrowReaderMetadata::load_async(&mut reader, Default::default()).await?;
        let pq_meta = arrow_metadata.metadata().clone();

        let modified: DateTime<Local> = meta.last_modified.into();

        Ok(SourceFile {
            name: url.to_string(),
            size: *size,
            created: None,
            modified: Some(modified),
            etag: meta.e_tag,
            pq_meta,
        })
    })
}

pub struct SourceFile {
    pub name: String,
    pub size: u64,
    pub created: Option<DateTime<Local>>,
    pub modified: Option<DateTime<Local>>,
    pub etag: Option<String>,
    pub pq_meta: Arc<ParquetMetaData>,
}
