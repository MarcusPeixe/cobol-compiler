#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    Numeric,
    Alphabetic,
    Alphanumeric,
}

pub fn parse_picture(picture: &str) -> DataType {
    let normalized = picture.to_ascii_uppercase();
    if normalized
        .chars()
        .any(|ch| matches!(ch, '9' | 'S' | 'V' | 'P'))
    {
        DataType::Numeric
    } else if normalized.contains('A') && !normalized.contains('X') && !normalized.contains('N') {
        DataType::Alphabetic
    } else {
        DataType::Alphanumeric
    }
}

pub fn is_valid_picture(picture: &str) -> bool {
    if picture.is_empty() {
        return false;
    }

    let mut has_picture_symbol = false;
    let mut chars = picture.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch.to_ascii_uppercase() {
            'A' | 'X' | '9' | 'S' | 'V' | 'P' => has_picture_symbol = true,
            '(' => {
                let mut digits = String::new();
                while let Some(next) = chars.peek().copied() {
                    if next.is_ascii_digit() {
                        digits.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if digits.is_empty()
                    || digits.parse::<usize>().is_err()
                    || digits.parse::<usize>().is_ok_and(|count| count == 0)
                    || chars.next() != Some(')')
                {
                    return false;
                }
            }
            ')' | '0'..='9' => return false,
            _ => return false,
        }
    }

    has_picture_symbol
}

pub fn is_move_compatible(source: DataType, target: DataType) -> bool {
    !matches!(
        (source, target),
        (DataType::Alphabetic, DataType::Numeric) | (DataType::Alphanumeric, DataType::Numeric)
    )
}

pub fn can_operate_arithmetic(data_type: DataType) -> bool {
    data_type == DataType::Numeric
}

#[cfg(test)]
mod tests {
    use super::{
        DataType, can_operate_arithmetic, is_move_compatible, is_valid_picture, parse_picture,
    };

    #[test]
    fn infers_types_from_picture_symbols() {
        assert_eq!(parse_picture("S9(3)V99"), DataType::Numeric);
        assert_eq!(parse_picture("A(10)"), DataType::Alphabetic);
        assert_eq!(parse_picture("X(20)"), DataType::Alphanumeric);
    }

    #[test]
    fn validates_picture_repetition_syntax() {
        assert!(is_valid_picture("S9(3)V99"));
        assert!(!is_valid_picture("X(0)"));
        assert!(!is_valid_picture("9(2"));
    }

    #[test]
    fn checks_move_and_arithmetic_compatibility() {
        assert!(is_move_compatible(
            DataType::Numeric,
            DataType::Alphanumeric
        ));
        assert!(!is_move_compatible(DataType::Alphabetic, DataType::Numeric));
        assert!(can_operate_arithmetic(DataType::Numeric));
        assert!(!can_operate_arithmetic(DataType::Alphabetic));
    }
}
