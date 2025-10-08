use super::super::traits::GeneratorOptions;
use super::features::ExcelFeatures;

/// Excel generator entry point.
pub struct ExcelGenerator {
    /// Enabled Excel features.
    features: ExcelFeatures,
    /// Generator options.
    pub(super) options: GeneratorOptions,
}

impl ExcelGenerator {
    /// Create a new Excel generator with all features enabled.
    #[must_use]
    pub fn new() -> Self {
        Self {
            features: ExcelFeatures::ALL,
            options: GeneratorOptions::default(),
        }
    }

    /// Create generator with custom options.
    #[must_use]
    pub fn with_options(options: GeneratorOptions) -> Self {
        let mut generator = Self::new();
        generator.options = options;
        generator
    }

    /// Configure summary sheet generation.
    #[must_use]
    pub fn with_summary(mut self, enabled: bool) -> Self {
        if enabled {
            self.features.insert(ExcelFeatures::INCLUDE_SUMMARY);
        } else {
            self.features.remove(ExcelFeatures::INCLUDE_SUMMARY);
        }
        self
    }

    /// Check if summary sheet is enabled.
    #[must_use]
    pub fn include_summary(&self) -> bool {
        self.features.contains(ExcelFeatures::INCLUDE_SUMMARY)
    }

    /// Check if data validation is enabled.
    #[must_use]
    pub fn add_validation(&self) -> bool {
        self.features.contains(ExcelFeatures::ADD_VALIDATION)
    }

    /// Check if header freezing is enabled.
    #[must_use]
    pub fn freeze_headers(&self) -> bool {
        self.features.contains(ExcelFeatures::FREEZE_HEADERS)
    }

    /// Check if filters are enabled.
    #[must_use]
    pub fn add_filters(&self) -> bool {
        self.features.contains(ExcelFeatures::ADD_FILTERS)
    }

    /// Check if pattern validation formulas should be emitted.
    #[must_use]
    pub fn pattern_validation(&self) -> bool {
        self.features.contains(ExcelFeatures::PATTERN_VALIDATION)
    }

    /// Configure example data generation (reserved for future use).
    #[must_use]
    pub fn with_examples(self, _enabled: bool) -> Self {
        // Examples feature not yet implemented, but method exists for API compatibility.
        self
    }

    /// Configure data validation.
    #[must_use]
    pub fn with_validation(mut self, enabled: bool) -> Self {
        if enabled {
            self.features.insert(ExcelFeatures::ADD_VALIDATION);
        } else {
            self.features.remove(ExcelFeatures::ADD_VALIDATION);
        }
        self
    }

    /// Configure regex-based pattern validation support.
    #[must_use]
    pub fn with_pattern_validation(mut self, enabled: bool) -> Self {
        if enabled {
            self.features.insert(ExcelFeatures::PATTERN_VALIDATION);
        } else {
            self.features.remove(ExcelFeatures::PATTERN_VALIDATION);
        }
        self
    }

    /// Configure header freezing.
    #[must_use]
    pub fn with_frozen_headers(mut self, enabled: bool) -> Self {
        if enabled {
            self.features.insert(ExcelFeatures::FREEZE_HEADERS);
        } else {
            self.features.remove(ExcelFeatures::FREEZE_HEADERS);
        }
        self
    }

    /// Configure filter addition.
    #[must_use]
    pub fn with_filters(mut self, enabled: bool) -> Self {
        if enabled {
            self.features.insert(ExcelFeatures::ADD_FILTERS);
        } else {
            self.features.remove(ExcelFeatures::ADD_FILTERS);
        }
        self
    }
}
