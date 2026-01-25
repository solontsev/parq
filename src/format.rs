use byte_unit::{Byte, Unit, UnitType};
use num_format::{Locale, ToFormattedString};

pub fn format_file_size(bytes: u64) -> String {
    let byte = Byte::from_u64(bytes);
    let adjusted = byte.get_appropriate_unit(UnitType::Binary);

    let bytes_str = if adjusted.get_unit() != Unit::B {
        format!(" ({} bytes)", format_number(bytes))
    } else {
        "".to_string()
    };

    if adjusted.get_value() % 1.0 == 0.0 {
        format!("{adjusted:.0}{bytes_str}")
    } else {
        format!("{adjusted:.2}{bytes_str}")
    }
}

pub fn format_number(n: u64) -> String {
    n.to_formatted_string(&Locale::en)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(1024), "1 KiB (1,024 bytes)");
        assert_eq!(format_file_size(1), "1 B");
        assert_eq!(format_file_size(1000), "1000 B");
        assert_eq!(format_file_size(1024000), "1000 KiB (1,024,000 bytes)");
        assert_eq!(
            format_file_size(1024000000),
            "976.56 MiB (1,024,000,000 bytes)"
        );
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(1024), "1,024");
        assert_eq!(format_number(123), "123");
    }
}
