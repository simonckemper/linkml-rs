use super::super::traits::GeneratorError;

/// Safely cast usize to u16 for Excel column indices.
/// Excel has a maximum of 16,384 columns (2^14).
pub(super) fn usize_to_u16_column(value: usize) -> Result<u16, GeneratorError> {
    const MAX_EXCEL_COLUMNS: usize = 16_384;

    if value >= MAX_EXCEL_COLUMNS {
        return Err(GeneratorError::Generation(format!(
            "Too many columns for Excel: {value} (max: {MAX_EXCEL_COLUMNS})"
        )));
    }

    u16::try_from(value)
        .map_err(|_| GeneratorError::Generation(format!("Column index {value} cannot fit in u16")))
}

/// Safely cast i64 to i32 for Excel data validation ranges.
pub(super) fn i64_to_i32_validation(value: i64) -> Result<i32, GeneratorError> {
    i32::try_from(value).map_err(|_| {
        GeneratorError::Generation(format!("Validation value {value} is outside i32 range"))
    })
}
