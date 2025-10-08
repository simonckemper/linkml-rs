use bitflags::bitflags;

bitflags! {
    /// Excel generation features to enable.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ExcelFeatures: u8 {
        /// Include a summary sheet.
        const INCLUDE_SUMMARY = 0b0001;
        /// Add data validation.
        const ADD_VALIDATION = 0b0010;
        /// Freeze header rows.
        const FREEZE_HEADERS = 0b0100;
        /// Add filters.
        const ADD_FILTERS = 0b1000;
        /// Enforce regex patterns using Excel formulas when supported.
        const PATTERN_VALIDATION = 0b1_0000;

        /// All features enabled (default).
        const ALL = Self::INCLUDE_SUMMARY.bits()
                  | Self::ADD_VALIDATION.bits()
                  | Self::FREEZE_HEADERS.bits()
                  | Self::ADD_FILTERS.bits()
                  | Self::PATTERN_VALIDATION.bits();

        /// Basic features only (no validation or filters).
        const BASIC = Self::INCLUDE_SUMMARY.bits()
                    | Self::FREEZE_HEADERS.bits();

        /// No features (minimal Excel).
        const NONE = 0b0000;
    }
}
