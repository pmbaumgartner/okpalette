use crate::color::Rgb8;
use crate::error::{GlasbeyError, Result};

pub fn parse_hex_color(input: &str) -> Result<Rgb8> {
    let hex = input.strip_prefix('#').unwrap_or(input);

    match hex.len() {
        3 => parse_shorthand_hex(hex),
        6 => parse_full_hex(hex),
        length => Err(GlasbeyError::InvalidHexLength { length }),
    }
}

fn parse_shorthand_hex(hex: &str) -> Result<Rgb8> {
    let bytes = hex.as_bytes();
    let r = parse_hex_byte(bytes[0], 0)?;
    let g = parse_hex_byte(bytes[1], 1)?;
    let b = parse_hex_byte(bytes[2], 2)?;

    Ok(Rgb8 {
        r: r * 17,
        g: g * 17,
        b: b * 17,
    })
}

fn parse_full_hex(hex: &str) -> Result<Rgb8> {
    let bytes = hex.as_bytes();
    Ok(Rgb8 {
        r: parse_hex_pair(bytes[0], bytes[1], 0)?,
        g: parse_hex_pair(bytes[2], bytes[3], 2)?,
        b: parse_hex_pair(bytes[4], bytes[5], 4)?,
    })
}

fn parse_hex_pair(high: u8, low: u8, byte_index: usize) -> Result<u8> {
    Ok(parse_hex_byte(high, byte_index)? * 16 + parse_hex_byte(low, byte_index + 1)?)
}

fn parse_hex_byte(byte: u8, index: usize) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(GlasbeyError::InvalidHexDigit {
            index,
            ch: char::from(byte),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::rgb;

    #[test]
    fn parses_valid_hex_colors() {
        let cases = [
            ("#0fA", rgb(0, 255, 170)),
            ("#00ffaa", rgb(0, 255, 170)),
            ("0fA", rgb(0, 255, 170)),
            ("00ffaa", rgb(0, 255, 170)),
            ("Cc33aA", rgb(204, 51, 170)),
        ];

        for (input, expected) in cases {
            assert_eq!(parse_hex_color(input), Ok(expected));
        }
    }

    #[test]
    fn rejects_invalid_hex_lengths() {
        for input in [
            "",
            "#",
            "0",
            "00",
            "0000",
            "#0000",
            "00000",
            "0000000",
            "#00000000",
            "0xffaa00",
            " red ",
        ] {
            assert!(
                matches!(
                    parse_hex_color(input),
                    Err(GlasbeyError::InvalidHexLength { .. })
                ),
                "{input:?} should fail with invalid length"
            );
        }
    }

    #[test]
    fn rejects_invalid_hex_digits() {
        for input in ["ggg", "#12x45f", "f f", "zzzzzz"] {
            assert!(
                matches!(
                    parse_hex_color(input),
                    Err(GlasbeyError::InvalidHexDigit { .. })
                ),
                "{input:?} should fail with invalid digit"
            );
        }
    }

    #[test]
    fn rejects_whitespace_without_trimming() {
        for input in [" abc", "abc ", "#abc\n", "\tabc"] {
            assert!(parse_hex_color(input).is_err());
        }
    }
}
