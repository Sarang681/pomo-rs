//! Converts a time string such as `25:00` into large block digits.
//!
//! Digits are 3 columns wide and 5 rows tall, the colon is 1 column wide, and one blank
//! column separates neighbouring characters. This module knows nothing about ratatui.

pub const DIGIT_HEIGHT: usize = 5;

type Glyph = [&'static str; DIGIT_HEIGHT];

fn glyph(c: char) -> Glyph {
    match c {
        '0' => ["███", "█ █", "█ █", "█ █", "███"],
        '1' => [" █ ", "██ ", " █ ", " █ ", "███"],
        '2' => ["███", "  █", "███", "█  ", "███"],
        '3' => ["███", "  █", "███", "  █", "███"],
        '4' => ["█ █", "█ █", "███", "  █", "  █"],
        '5' => ["███", "█  ", "███", "  █", "███"],
        '6' => ["███", "█  ", "███", "█ █", "███"],
        '7' => ["███", "  █", "  █", "  █", "  █"],
        '8' => ["███", "█ █", "███", "█ █", "███"],
        '9' => ["███", "█ █", "███", "  █", "███"],
        ':' => [" ", "█", " ", "█", " "],
        _ => ["   ", "   ", "   ", "   ", "   "],
    }
}

/// Returns the five rows of text that draw `text` in large digits.
///
/// Characters other than `0-9` and `:` are drawn as a blank 3-column cell, so the layout
/// never shifts.
pub fn big_digits(text: &str) -> [String; DIGIT_HEIGHT] {
    let mut rows: [String; DIGIT_HEIGHT] = Default::default();

    for (index, c) in text.chars().enumerate() {
        for (row, part) in rows.iter_mut().zip(glyph(c)) {
            if index > 0 {
                row.push(' ');
            }
            row.push_str(part);
        }
    }

    rows
}

/// Number of columns `big_digits(text)` occupies, without building the rows.
pub fn big_digits_width(text: &str) -> usize {
    let glyphs: usize = text.chars().map(|c| glyph(c)[0].chars().count()).sum();
    let separators = text.chars().count().saturating_sub(1);
    glyphs + separators
}

#[cfg(test)]
mod tests {
    use super::*;

    fn width_of(row: &str) -> usize {
        row.chars().count()
    }

    #[test]
    fn test_every_digit_is_three_columns_wide_on_every_row() {
        for digit in '0'..='9' {
            let rows = big_digits(&digit.to_string());

            for row in rows {
                assert_eq!(width_of(&row), 3, "digit {digit}");
            }
        }
    }

    #[test]
    fn test_colon_is_one_column_wide_with_blocks_on_the_second_and_fourth_rows() {
        let rows = big_digits(":");

        assert_eq!(rows, [" ", "█", " ", "█", " "].map(String::from));
    }

    #[test]
    fn test_a_time_is_five_rows_of_equal_width() {
        let rows = big_digits("25:00");

        assert_eq!(rows.len(), DIGIT_HEIGHT);
        for row in &rows {
            assert_eq!(width_of(row), 17);
        }
    }

    #[test]
    fn test_a_time_matches_the_expected_drawing() {
        let rows = big_digits("25:00");

        assert_eq!(
            rows,
            [
                "███ ███   ███ ███",
                "  █ █   █ █ █ █ █",
                "███ ███   █ █ █ █",
                "█     █ █ █ █ █ █",
                "███ ███   ███ ███",
            ]
            .map(String::from)
        );
    }

    #[test]
    fn test_width_matches_the_rendered_rows() {
        for text in ["", "0", ":", "25:00", "04:59", "00:00", "99:59"] {
            let rows = big_digits(text);

            assert_eq!(big_digits_width(text), width_of(&rows[0]), "text {text:?}");
        }
    }

    #[test]
    fn test_empty_text_gives_five_empty_rows() {
        let rows = big_digits("");

        assert!(rows.iter().all(|row| row.is_empty()));
        assert_eq!(big_digits_width(""), 0);
    }

    #[test]
    fn test_unknown_characters_are_drawn_as_blank_cells() {
        let rows = big_digits("x");

        assert_eq!(rows, ["   ", "   ", "   ", "   ", "   "].map(String::from));
    }

    #[test]
    fn test_every_row_of_every_digit_uses_only_blocks_and_spaces() {
        for digit in '0'..='9' {
            for row in big_digits(&digit.to_string()) {
                assert!(row.chars().all(|c| c == '█' || c == ' '));
            }
        }
    }
}
