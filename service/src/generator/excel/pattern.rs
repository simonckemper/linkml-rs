use super::super::traits::GeneratorError;
use regex::Regex;

/// Build an Excel data validation formula that enforces the provided regex pattern.
///
/// The implementation relies on Excel's `REGEXMATCH` function which is available in
/// Microsoft 365 (2022+) and Excel for the web. Consumers on older Excel versions
/// will receive the standard Excel validation error when opening the workbook.
pub(super) fn build_pattern_formula(
    pattern: &str,
    column: u16,
    start_row: u32,
) -> Result<String, GeneratorError> {
    Regex::new(pattern).map_err(|err| {
        GeneratorError::Generation(format!(
            "Invalid pattern '{pattern}' for Excel validation: {err}"
        ))
    })?;

    let column_ref = column_index_to_letter(column);
    let row_ref = start_row + 1; // Excel rows are 1-based.
    let escaped_pattern = pattern.replace('"', "\"\"");

    Ok(format!(
        "=LET(_value,{column_ref}{row_ref},OR(_value=\"\",REGEXMATCH(_value,\"{escaped_pattern}\")))"
    ))
}

fn column_index_to_letter(column: u16) -> String {
    let mut col = i32::from(column);
    let mut letters = Vec::new();

    loop {
        let remainder = col % 26;
        letters.push(char::from(
            b'A' + u8::try_from(remainder).expect("A-Z range"),
        ));
        col = (col / 26) - 1;
        if col < 0 {
            break;
        }
    }

    letters.iter().rev().collect()
}
