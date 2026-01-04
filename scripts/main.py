import pyarrow as pa
import pyarrow.parquet as pq

def simple_parquet():
    # Create a single table with all 5 rows
    data = {
        'id': pa.array([1, 2, 3, 4, 5], type=pa.int64()),
        'name': pa.array(['Alice', 'Bob', 'Charlie', 'Diana', 'Eve'], type=pa.string()),
        'score': pa.array([95.5, 87.3, 92.1, 88.9, 91.4], type=pa.float64())
    }
    table = pa.table(data)

    # Write to parquet with row_group_size=3 to create 2 row groups (3 rows + 2 rows)
    file_name = 'simple.parquet'
    pq.write_table(table, file_name, row_group_size=3)

    print(f"Parquet file created: {file_name}")
    print("\nFile structure:")
    print(f"Total rows: {table.num_rows}")
    print(f"Columns: {table.column_names}")
    print(f"Column types: id (int64), name (string), score (float64)")

    # Verify the row groups
    parquet_file = pq.ParquetFile(file_name)
    print(f"\nVerification - Number of row groups: {parquet_file.num_row_groups}")
    for i in range(parquet_file.num_row_groups):
        print(f"Row group {i}: {parquet_file.metadata.row_group(i).num_rows} rows")


def main():
    simple_parquet()


if __name__ == "__main__":
    main()
