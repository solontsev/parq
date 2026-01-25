import pyarrow as pa
import pyarrow.parquet as pq
from datetime import datetime

def nested_schema():
    """
    Create a parquet file with complex nested structures:
    - Struct columns (nested objects)
    - List columns (arrays)
    - Map columns (key-value pairs)
    """

    # Define the schema with nested structures
    schema = pa.schema([
        pa.field('id', pa.int64()),
        pa.field('name', pa.string()),
        pa.field('score', pa.float64()),
        # Struct: nested object with address details
        pa.field('address', pa.struct([
            pa.field('street', pa.string()),
            pa.field('city', pa.string()),
            pa.field('zip_code', pa.string()),
            pa.field('country', pa.string())
        ])),
        # List: array of strings (hobbies)
        pa.field('hobbies', pa.list_(pa.string())),
        # Struct with list inside: contact info
        pa.field('contact', pa.struct([
            pa.field('email', pa.string()),
            pa.field('phone_numbers', pa.list_(pa.string())),
            pa.field('social_media', pa.map_(pa.string(), pa.string()))
        ])),
        # List of structs: course history
        pa.field('courses', pa.list_(pa.struct([
            pa.field('course_name', pa.string()),
            pa.field('grade', pa.float64()),
            pa.field('completion_date', pa.date32())
        ]))),
    ])

    # Create sample data
    data = {
        'id': pa.array([1, 2, 3, 4, 5], type=pa.int64()),
        'name': pa.array(['Alice', 'Bob', 'Charlie', 'Diana', 'Eve'], type=pa.string()),
        'score': pa.array([95.5, 87.3, 92.1, 88.9, 91.4], type=pa.float64()),

        # Address struct data
        'address': pa.StructArray.from_arrays([
            pa.array(['123 Main St', '456 Oak Ave', '789 Pine Rd', '321 Elm St', '654 Maple Dr']),
            pa.array(['New York', 'San Francisco', 'Austin', 'Boston', 'Seattle']),
            pa.array(['10001', '94102', '78701', '02101', '98101']),
            pa.array(['USA', 'USA', 'USA', 'USA', 'USA'])
        ], names=['street', 'city', 'zip_code', 'country']),

        # Hobbies list
        'hobbies': pa.array([
            ['reading', 'hiking', 'coding'],
            ['gaming', 'cooking'],
            ['photography', 'painting', 'music', 'travel'],
            ['yoga', 'swimming'],
            ['gardening', 'writing', 'drawing']
        ]),

        # Contact struct with nested list and struct (for key-value pairs)
        'contact': pa.StructArray.from_arrays([
            pa.array([
                'alice@example.com',
                'bob@example.com',
                'charlie@example.com',
                'diana@example.com',
                'eve@example.com'
            ]),
            pa.array([
                ['555-0001', '555-0101'],
                ['555-0002'],
                ['555-0003', '555-0103', '555-0203'],
                ['555-0004'],
                ['555-0005', '555-0105']
            ]),
            pa.array([
                {'twitter': '@alice', 'linkedin': 'alice-profile'},
                {'github': 'bob-github', 'twitter': '@bob_dev'},
                {'instagram': 'charlie_photo', 'github': 'charlie-code'},
                {'linkedin': 'diana-profile'},
                {'twitter': '@eve', 'github': 'eve-dev', 'instagram': 'eve_art'}
            ], type=pa.map_(pa.string(), pa.string()))
        ], names=['email', 'phone_numbers', 'social_media']),

        # Courses list of structs
        'courses': pa.array([
            [
                {'course_name': 'Python 101', 'grade': 98.0, 'completion_date': datetime(2023, 6, 15).date()},
                {'course_name': 'Data Science', 'grade': 95.5, 'completion_date': datetime(2023, 12, 20).date()}
            ],
            [
                {'course_name': 'Web Dev', 'grade': 87.0, 'completion_date': datetime(2023, 9, 10).date()}
            ],
            [
                {'course_name': 'ML Basics', 'grade': 92.0, 'completion_date': datetime(2023, 8, 5).date()},
                {'course_name': 'Advanced ML', 'grade': 91.0, 'completion_date': datetime(2024, 1, 15).date()},
                {'course_name': 'Python 101', 'grade': 96.0, 'completion_date': datetime(2023, 5, 20).date()}
            ],
            [
                {'course_name': 'Cloud Computing', 'grade': 89.0, 'completion_date': datetime(2023, 11, 30).date()}
            ],
            [
                {'course_name': 'Data Visualization', 'grade': 93.0, 'completion_date': datetime(2023, 10, 25).date()},
                {'course_name': 'Statistics', 'grade': 90.0, 'completion_date': datetime(2024, 1, 10).date()}
            ]
        ]),
    }

    # Create the table
    table = pa.table(data, schema=schema)

    # Write to parquet with row groups
    file_name = '../data/nested.parquet'
    pq.write_table(table, file_name, row_group_size=3)

    print(f"Complex Parquet file created: {file_name}\n")
    print("=" * 70)
    print("FILE STRUCTURE:")
    print("=" * 70)
    print(f"Total rows: {table.num_rows}")
    print(f"\nColumns ({len(table.column_names)}):")
    for i, col_name in enumerate(table.column_names, 1):
        print(f"  {i}. {col_name}: {table.schema[i-1].type}")


def main():
    nested_schema()


if __name__ == "__main__":
    main()