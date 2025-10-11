//! Summary generator for `LinkML` schemas
//!
//! This module generates summary statistics and reports from `LinkML` schemas,
//! providing insights into schema structure, complexity, and usage patterns.

use crate::generator::traits::{Generator, GeneratorConfig};
use indexmap::IndexMap;
use linkml_core::error::LinkMLError;
use linkml_core::types::{
    ClassDefinition, EnumDefinition, SchemaDefinition, SlotDefinition, TypeDefinition,
};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// Summary generator configuration
#[derive(Debug, Clone)]
pub struct SummaryGeneratorConfig {
    /// Base generator configuration
    pub base: GeneratorConfig,
    /// Output format
    pub format: SummaryFormat,
    /// Include detailed statistics
    pub detailed: bool,
    /// Include visualization data
    pub include_viz_data: bool,
    /// Include usage patterns
    pub analyze_usage: bool,
    /// Include complexity metrics
    pub complexity_metrics: bool,
}

/// Summary output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryFormat {
    /// Tab-separated values
    Tsv,
    /// Markdown report
    Markdown,
    /// `JSON` statistics
    Json,
    /// HTML report
    Html,
}

impl Default for SummaryGeneratorConfig {
    fn default() -> Self {
        Self {
            base: GeneratorConfig::default(),
            format: SummaryFormat::Tsv,
            detailed: false,
            include_viz_data: false,
            analyze_usage: true,
            complexity_metrics: true,
        }
    }
}

/// Summary generator
pub struct SummaryGenerator {
    config: SummaryGeneratorConfig,
    /// Additional generator options for customization
    options: super::traits::GeneratorOptions,
}

/// Schema statistics
#[derive(Debug, Default)]
struct SchemaStats {
    // Basic counts
    class_count: usize,
    slot_count: usize,
    type_count: usize,
    enum_count: usize,
    subset_count: usize,

    // Class statistics
    abstract_class_count: usize,
    mixin_class_count: usize,
    classes_with_slots: usize,
    classes_with_attributes: usize,
    max_inheritance_depth: usize,
    avg_slots_per_class: f64,

    // Slot statistics
    required_slot_count: usize,
    multivalued_slot_count: usize,
    identifier_slot_count: usize,
    slots_with_patterns: usize,
    slots_with_constraints: usize,

    // Type statistics
    custom_type_count: usize,
    types_with_patterns: usize,
    types_with_constraints: usize,

    // Enum statistics
    total_permissible_values: usize,
    avg_values_per_enum: f64,

    // Relationship statistics
    inheritance_relationships: usize,
    mixin_relationships: usize,
    slot_usage_count: HashMap<String, usize>,

    // Complexity metrics
    schema_complexity_score: f64,
    cyclomatic_complexity: usize,
    coupling_score: f64,
    cohesion_score: f64,

    // Documentation coverage
    documented_classes: usize,
    documented_slots: usize,
    documented_types: usize,
    documented_enums: usize,
    documentation_coverage: f64,
}

impl SummaryGenerator {
    /// Create a new summary generator
    #[must_use]
    pub fn new(config: SummaryGeneratorConfig) -> Self {
        Self {
            config,
            options: super::traits::GeneratorOptions::default(),
        }
    }

    /// Create generator with custom options
    #[must_use]
    pub fn with_options(
        config: SummaryGeneratorConfig,
        options: super::traits::GeneratorOptions,
    ) -> Self {
        Self { config, options }
    }

    /// Get custom option value
    fn get_custom_option(&self, key: &str) -> Option<&String> {
        self.options.custom.get(key)
    }

    /// Generate summary from schema
    fn generate_summary(&self, schema: &SchemaDefinition) -> Result<String, LinkMLError> {
        let stats = self.calculate_statistics(schema);

        match self.config.format {
            SummaryFormat::Tsv => Ok(self.generate_tsv(&stats, schema)),
            SummaryFormat::Markdown => self.generate_markdown(&stats, schema),
            SummaryFormat::Json => self.generate_json(&stats, schema),
            SummaryFormat::Html => self.generate_html(&stats, schema),
        }
    }

    /// Calculate schema statistics
    fn calculate_statistics(&self, schema: &SchemaDefinition) -> SchemaStats {
        let mut stats = SchemaStats {
            class_count: schema.classes.len(),
            slot_count: schema.slots.len(),
            type_count: schema.types.len(),
            enum_count: schema.enums.len(),
            subset_count: schema.subsets.len(),
            ..SchemaStats::default()
        };

        // Analyze classes
        self.analyze_classes(&schema.classes, &mut stats);

        // Analyze slots
        self.analyze_slots(&schema.slots, &mut stats);

        // Analyze types
        self.analyze_types(&schema.types, &mut stats);

        // Analyze enums
        self.analyze_enums(&schema.enums, &mut stats);

        // Calculate derived metrics
        self.calculate_derived_metrics(&mut stats, schema);

        // Calculate complexity metrics if requested
        if self.config.complexity_metrics {
            self.calculate_complexity_metrics(&mut stats, schema);
        }

        stats
    }

    /// Analyze classes
    fn analyze_classes(
        &self,
        classes: &IndexMap<String, ClassDefinition>,
        stats: &mut SchemaStats,
    ) {
        for (_name, class_def) in classes {
            if class_def.abstract_.unwrap_or(false) {
                stats.abstract_class_count += 1;
            }

            if class_def.mixin.unwrap_or(false) {
                stats.mixin_class_count += 1;
            }

            if !class_def.slots.is_empty() || !class_def.slot_usage.is_empty() {
                stats.classes_with_slots += 1;
            }

            if !class_def.attributes.is_empty() {
                stats.classes_with_attributes += 1;
            }

            if class_def.description.is_some() {
                stats.documented_classes += 1;
            }

            // Count slot usage
            for slot in &class_def.slots {
                *stats.slot_usage_count.entry(slot.clone()).or_insert(0) += 1;
            }

            // Count inheritance relationships
            if class_def.is_a.is_some() {
                stats.inheritance_relationships += 1;
            }

            stats.mixin_relationships += class_def.mixins.len();
        }

        // Calculate average slots per class
        let total_slots: usize = classes.values().map(|c| c.slots.len()).sum();

        if stats.class_count > 0 {
            stats.avg_slots_per_class = crate::utils::usize_to_f64(total_slots)
                / crate::utils::usize_to_f64(stats.class_count);
        }
    }

    /// Analyze slots
    fn analyze_slots(&self, slots: &IndexMap<String, SlotDefinition>, stats: &mut SchemaStats) {
        for (_, slot_def) in slots {
            if slot_def.required.unwrap_or(false) {
                stats.required_slot_count += 1;
            }

            if slot_def.multivalued.unwrap_or(false) {
                stats.multivalued_slot_count += 1;
            }

            if slot_def.identifier.unwrap_or(false) {
                stats.identifier_slot_count += 1;
            }

            if slot_def.pattern.is_some() {
                stats.slots_with_patterns += 1;
            }

            if slot_def.minimum_value.is_some() || slot_def.maximum_value.is_some() {
                stats.slots_with_constraints += 1;
            }

            if slot_def.description.is_some() {
                stats.documented_slots += 1;
            }
        }
    }

    /// Analyze types
    fn analyze_types(&self, types: &IndexMap<String, TypeDefinition>, stats: &mut SchemaStats) {
        for (_, type_def) in types {
            if type_def.base_type.is_none() {
                stats.custom_type_count += 1;
            }

            if type_def.pattern.is_some() {
                stats.types_with_patterns += 1;
            }

            if type_def.minimum_value.is_some() || type_def.maximum_value.is_some() {
                stats.types_with_constraints += 1;
            }

            if type_def.description.is_some() {
                stats.documented_types += 1;
            }
        }
    }

    /// Analyze enums
    fn analyze_enums(&self, enums: &IndexMap<String, EnumDefinition>, stats: &mut SchemaStats) {
        for (_, enum_def) in enums {
            stats.total_permissible_values += enum_def.permissible_values.len();

            if enum_def.description.is_some() {
                stats.documented_enums += 1;
            }
        }

        if stats.enum_count > 0 {
            stats.avg_values_per_enum = crate::utils::usize_to_f64(stats.total_permissible_values)
                / crate::utils::usize_to_f64(stats.enum_count);
        }
    }

    /// Calculate derived metrics
    fn calculate_derived_metrics(&self, stats: &mut SchemaStats, schema: &SchemaDefinition) {
        // Calculate documentation coverage
        let total_elements =
            stats.class_count + stats.slot_count + stats.type_count + stats.enum_count;
        let documented_elements = stats.documented_classes
            + stats.documented_slots
            + stats.documented_types
            + stats.documented_enums;

        if total_elements > 0 {
            stats.documentation_coverage = crate::utils::usize_to_f64(documented_elements)
                / crate::utils::usize_to_f64(total_elements);
        }

        // Calculate max inheritance depth
        stats.max_inheritance_depth = self.calculate_max_inheritance_depth(&schema.classes);
    }

    /// Calculate maximum inheritance depth
    fn calculate_max_inheritance_depth(
        &self,
        classes: &IndexMap<String, ClassDefinition>,
    ) -> usize {
        let mut max_depth = 0;
        let mut visited = HashSet::new();

        for class_name in classes.keys() {
            let depth = self.calculate_inheritance_depth(class_name, classes, &mut visited);
            max_depth = max_depth.max(depth);
        }

        max_depth
    }

    /// Calculate inheritance depth for a class
    fn calculate_inheritance_depth(
        &self,
        class_name: &str,
        classes: &IndexMap<String, ClassDefinition>,
        visited: &mut HashSet<String>,
    ) -> usize {
        if visited.contains(class_name) {
            return 0;
        }

        visited.insert(class_name.to_string());

        if let Some(class_def) = classes.get(class_name)
            && let Some(parent) = &class_def.is_a
        {
            return 1 + self.calculate_inheritance_depth(parent, classes, visited);
        }

        0
    }

    /// Calculate complexity metrics
    fn calculate_complexity_metrics(&self, stats: &mut SchemaStats, schema: &SchemaDefinition) {
        // Schema complexity score (simple heuristic)
        stats.schema_complexity_score = (crate::utils::usize_to_f64(stats.class_count) * 1.0)
            + (crate::utils::usize_to_f64(stats.slot_count) * 0.5)
            + (crate::utils::usize_to_f64(stats.inheritance_relationships) * 2.0)
            + (crate::utils::usize_to_f64(stats.mixin_relationships) * 1.5)
            + (crate::utils::usize_to_f64(stats.slots_with_constraints) * 0.8);

        // Cyclomatic complexity (simplified)
        stats.cyclomatic_complexity = stats.inheritance_relationships
            + stats.mixin_relationships
            + stats.slots_with_constraints;

        // Coupling score (based on cross-references)
        let mut references = 0;
        for class_def in schema.classes.values() {
            references += class_def.slots.len();
        }

        if stats.class_count > 0 {
            stats.coupling_score = crate::utils::usize_to_f64(references)
                / crate::utils::usize_to_f64(stats.class_count);
        }

        // Cohesion score (based on shared slots)
        let mut shared_slots = 0;
        for count in stats.slot_usage_count.values() {
            if *count > 1 {
                shared_slots += 1;
            }
        }

        if stats.slot_count > 0 {
            stats.cohesion_score =
                f64::from(shared_slots) / crate::utils::usize_to_f64(stats.slot_count);
        }
    }

    /// Generate TSV format
    fn generate_tsv(&self, stats: &SchemaStats, schema: &SchemaDefinition) -> String {
        let mut output = String::new();

        // Header
        output.push_str(
            "Metric\tValue
",
        );

        // Basic information
        if !schema.name.is_empty() {
            writeln!(output, "Schema Name\t{}", schema.name)
                .expect("writeln! to String should never fail");
        }
        if let Some(version) = &schema.version {
            writeln!(output, "Schema Version\t{version}")
                .expect("writeln! to String should never fail");
        }

        // Basic counts
        writeln!(output, "Total Classes\t{}", stats.class_count)
            .expect("writeln! to String should never fail");
        writeln!(output, "Total Slots\t{}", stats.slot_count)
            .expect("writeln! to String should never fail");
        writeln!(output, "Total Types\t{}", stats.type_count)
            .expect("writeln! to String should never fail");
        writeln!(output, "Total Enums\t{}", stats.enum_count)
            .expect("writeln! to String should never fail");
        writeln!(output, "Total Subsets\t{}", stats.subset_count)
            .expect("writeln! to String should never fail");

        // Class statistics
        writeln!(output, "Abstract Classes\t{}", stats.abstract_class_count)
            .expect("LinkML operation should succeed");
        writeln!(output, "Mixin Classes\t{}", stats.mixin_class_count)
            .expect("writeln! to String should never fail");
        writeln!(output, "Classes with Slots\t{}", stats.classes_with_slots)
            .expect("LinkML operation should succeed");
        writeln!(
            output,
            "Classes with Attributes\t{}",
            stats.classes_with_attributes
        )
        .expect("LinkML operation should succeed");
        writeln!(
            output,
            "Max Inheritance Depth\t{}",
            stats.max_inheritance_depth
        )
        .expect("LinkML operation should succeed");
        writeln!(
            output,
            "Avg Slots per Class\t{:.2}",
            stats.avg_slots_per_class
        )
        .expect("LinkML operation should succeed");

        // Slot statistics
        writeln!(output, "Required Slots\t{}", stats.required_slot_count)
            .expect("writeln! to String should never fail");
        writeln!(
            output,
            "Multivalued Slots\t{}",
            stats.multivalued_slot_count
        )
        .expect("LinkML operation should succeed");
        writeln!(output, "Identifier Slots\t{}", stats.identifier_slot_count)
            .expect("LinkML operation should succeed");
        writeln!(output, "Slots with Patterns\t{}", stats.slots_with_patterns)
            .expect("LinkML operation should succeed");
        writeln!(
            output,
            "Slots with Constraints\t{}",
            stats.slots_with_constraints
        )
        .expect("LinkML operation should succeed");

        // Documentation
        writeln!(
            output,
            "Documentation Coverage\t{:.1}%",
            stats.documentation_coverage * 100.0
        )
        .expect("LinkML operation should succeed");

        if self.config.complexity_metrics {
            writeln!(
                output,
                "Schema Complexity Score\t{:.2}",
                stats.schema_complexity_score
            )
            .expect("LinkML operation should succeed");
            writeln!(
                output,
                "Cyclomatic Complexity\t{}",
                stats.cyclomatic_complexity
            )
            .expect("LinkML operation should succeed");
            writeln!(output, "Coupling Score\t{:.2}", stats.coupling_score)
                .expect("writeln! to String should never fail");
            writeln!(output, "Cohesion Score\t{:.2}", stats.cohesion_score)
                .expect("writeln! to String should never fail");
        }

        // Detailed slot usage if requested
        if self.config.detailed && !stats.slot_usage_count.is_empty() {
            output.push_str(
                "
Slot Usage Analysis
",
            );
            output.push_str(
                "Slot\tUsage Count
",
            );

            let mut slot_usage: Vec<_> = stats.slot_usage_count.iter().collect();
            slot_usage.sort_by(|a, b| b.1.cmp(a.1));

            for (slot, count) in slot_usage {
                writeln!(output, "{slot}\t{count}").expect("writeln! to String should never fail");
            }
        }

        output
    }

    /// Generate Markdown format
    fn generate_markdown(
        &self,
        stats: &SchemaStats,
        schema: &SchemaDefinition,
    ) -> Result<String, LinkMLError> {
        let mut output = String::new();

        // Title
        output.push_str(
            "# LinkML Schema Summary Report

",
        );

        if !schema.name.is_empty() {
            writeln!(
                output,
                "## Schema: {}
",
                schema.name
            )
            .expect("writeln! to String should never fail");
        }

        if let Some(description) = &schema.description {
            writeln!(
                output,
                "{description}
"
            )
            .expect("writeln! to String should never fail");
        }

        // Basic information
        output.push_str(
            "## Overview

",
        );
        output.push_str(
            "| Metric | Value |
",
        );
        output.push_str(
            "|--------|-------|
",
        );

        if let Some(version) = &schema.version {
            writeln!(output, "| Version | {version} |")
                .expect("writeln! to String should never fail");
        }

        writeln!(output, "| Total Classes | {} |", stats.class_count)
            .expect("writeln! to String should never fail");
        writeln!(output, "| Total Slots | {} |", stats.slot_count)
            .expect("writeln! to String should never fail");
        writeln!(output, "| Total Types | {} |", stats.type_count)
            .expect("writeln! to String should never fail");
        writeln!(output, "| Total Enums | {} |", stats.enum_count)
            .expect("writeln! to String should never fail");
        writeln!(
            output,
            "| Documentation Coverage | {:.1}% |",
            stats.documentation_coverage * 100.0
        )
        .expect("LinkML operation should succeed");

        // Class analysis
        output.push_str(
            "
## Class Analysis

",
        );
        output.push_str(
            "| Metric | Value |
",
        );
        output.push_str(
            "|--------|-------|
",
        );
        writeln!(
            output,
            "| Abstract Classes | {} |",
            stats.abstract_class_count
        )
        .expect("LinkML operation should succeed");
        writeln!(output, "| Mixin Classes | {} |", stats.mixin_class_count)
            .expect("LinkML operation should succeed");
        writeln!(
            output,
            "| Max Inheritance Depth | {} |",
            stats.max_inheritance_depth
        )
        .expect("LinkML operation should succeed");
        writeln!(
            output,
            "| Average Slots per Class | {:.2} |",
            stats.avg_slots_per_class
        )
        .expect("LinkML operation should succeed");

        // Slot analysis
        output.push_str(
            "
## Slot Analysis

",
        );
        output.push_str(
            "| Metric | Value |
",
        );
        output.push_str(
            "|--------|-------|
",
        );
        writeln!(output, "| Required Slots | {} |", stats.required_slot_count)
            .expect("LinkML operation should succeed");
        writeln!(
            output,
            "| Multivalued Slots | {} |",
            stats.multivalued_slot_count
        )
        .expect("LinkML operation should succeed");
        writeln!(
            output,
            "| Identifier Slots | {} |",
            stats.identifier_slot_count
        )
        .expect("LinkML operation should succeed");
        writeln!(
            output,
            "| Slots with Constraints | {} |",
            stats.slots_with_constraints
        )
        .expect("LinkML operation should succeed");

        // Complexity metrics
        if self.config.complexity_metrics {
            output.push_str(
                "
## Complexity Metrics

",
            );

            // Add custom header from options if provided
            if let Some(custom_header) = self.get_custom_option("complexity_header") {
                write!(
                    output,
                    "{custom_header}

"
                )
                .expect("Writing to string cannot fail");
            }

            output.push_str(
                "| Metric | Value |
",
            );
            output.push_str(
                "|--------|-------|
",
            );
            writeln!(
                output,
                "| Schema Complexity Score | {:.2} |",
                stats.schema_complexity_score
            )
            .expect("LinkML operation should succeed");
            writeln!(
                output,
                "| Cyclomatic Complexity | {} |",
                stats.cyclomatic_complexity
            )
            .expect("LinkML operation should succeed");
            writeln!(output, "| Coupling Score | {:.2} |", stats.coupling_score)
                .expect("LinkML operation should succeed");
            writeln!(output, "| Cohesion Score | {:.2} |", stats.cohesion_score)
                .expect("LinkML operation should succeed");
        }

        // Most used slots
        if self.config.detailed && !stats.slot_usage_count.is_empty() {
            output.push_str(
                "
## Most Used Slots

",
            );

            let mut slot_usage: Vec<_> = stats.slot_usage_count.iter().collect();
            slot_usage.sort_by(|a, b| b.1.cmp(a.1));

            output.push_str(
                "| Slot | Usage Count |
",
            );
            output.push_str(
                "|------|-------------|
",
            );

            for (slot, count) in slot_usage.iter().take(10) {
                writeln!(output, "| {slot} | {count} |")
                    .expect("writeln! to String should never fail");
            }
        }

        Ok(output)
    }

    /// Generate `JSON` format
    fn generate_json(
        &self,
        stats: &SchemaStats,
        schema: &SchemaDefinition,
    ) -> Result<String, LinkMLError> {
        use serde_json::{Map, Value, json};

        let mut root = Map::new();

        // Schema information
        let mut schema_info = Map::new();
        if !schema.name.is_empty() {
            schema_info.insert("name".to_string(), json!(&schema.name));
        }
        if let Some(version) = &schema.version {
            schema_info.insert("version".to_string(), json!(version));
        }
        if let Some(description) = &schema.description {
            schema_info.insert("description".to_string(), json!(description));
        }
        root.insert("schema".to_string(), Value::Object(schema_info));

        // Basic statistics
        let mut basic_stats = Map::new();
        basic_stats.insert("class_count".to_string(), json!(stats.class_count));
        basic_stats.insert("slot_count".to_string(), json!(stats.slot_count));
        basic_stats.insert("type_count".to_string(), json!(stats.type_count));
        basic_stats.insert("enum_count".to_string(), json!(stats.enum_count));
        basic_stats.insert("subset_count".to_string(), json!(stats.subset_count));
        root.insert("basic_stats".to_string(), Value::Object(basic_stats));

        // Class statistics
        let mut class_stats = Map::new();
        class_stats.insert(
            "abstract_count".to_string(),
            json!(stats.abstract_class_count),
        );
        class_stats.insert("mixin_count".to_string(), json!(stats.mixin_class_count));
        class_stats.insert("with_slots".to_string(), json!(stats.classes_with_slots));
        class_stats.insert(
            "with_attributes".to_string(),
            json!(stats.classes_with_attributes),
        );
        class_stats.insert(
            "max_inheritance_depth".to_string(),
            json!(stats.max_inheritance_depth),
        );
        class_stats.insert(
            "avg_slots_per_class".to_string(),
            json!(stats.avg_slots_per_class),
        );
        root.insert("class_stats".to_string(), Value::Object(class_stats));

        // Slot statistics
        let mut slot_stats = Map::new();
        slot_stats.insert(
            "required_count".to_string(),
            json!(stats.required_slot_count),
        );
        slot_stats.insert(
            "multivalued_count".to_string(),
            json!(stats.multivalued_slot_count),
        );
        slot_stats.insert(
            "identifier_count".to_string(),
            json!(stats.identifier_slot_count),
        );
        slot_stats.insert(
            "with_patterns".to_string(),
            json!(stats.slots_with_patterns),
        );
        slot_stats.insert(
            "with_constraints".to_string(),
            json!(stats.slots_with_constraints),
        );
        root.insert("slot_stats".to_string(), Value::Object(slot_stats));

        // Documentation
        let mut doc_stats = Map::new();
        doc_stats.insert(
            "documented_classes".to_string(),
            json!(stats.documented_classes),
        );
        doc_stats.insert(
            "documented_slots".to_string(),
            json!(stats.documented_slots),
        );
        doc_stats.insert(
            "documented_types".to_string(),
            json!(stats.documented_types),
        );
        doc_stats.insert(
            "documented_enums".to_string(),
            json!(stats.documented_enums),
        );
        doc_stats.insert(
            "coverage_percentage".to_string(),
            json!(stats.documentation_coverage * 100.0),
        );
        root.insert("documentation".to_string(), Value::Object(doc_stats));

        // Complexity metrics
        if self.config.complexity_metrics {
            let mut complexity = Map::new();
            complexity.insert(
                "schema_complexity_score".to_string(),
                json!(stats.schema_complexity_score),
            );
            complexity.insert(
                "cyclomatic_complexity".to_string(),
                json!(stats.cyclomatic_complexity),
            );
            complexity.insert("coupling_score".to_string(), json!(stats.coupling_score));
            complexity.insert("cohesion_score".to_string(), json!(stats.cohesion_score));
            root.insert("complexity_metrics".to_string(), Value::Object(complexity));
        }

        // Slot usage
        if self.config.detailed && !stats.slot_usage_count.is_empty() {
            root.insert("slot_usage".to_string(), json!(stats.slot_usage_count));
        }

        serde_json::to_string_pretty(&root).map_err(|e| {
            LinkMLError::ServiceError(format!("Failed to serialize summary JSON: {e}"))
        })
    }

    /// Generate HTML format
    fn generate_html(
        &self,
        stats: &SchemaStats,
        schema: &SchemaDefinition,
    ) -> Result<String, LinkMLError> {
        let mut html = String::new();

        html.push_str(
            "<!DOCTYPE html>
",
        );
        html.push_str(
            "<html lang=\"en\">
",
        );
        html.push_str(
            "<head>
",
        );
        html.push_str(
            "    <meta charset=\"UTF-8\">
",
        );
        html.push_str(
            "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">
",
        );
        html.push_str(
            "    <title>LinkML Schema Summary Report</title>
",
        );
        html.push_str(
            "    <style>
",
        );
        html.push_str(
            "        body { font-family: Arial, sans-serif; margin: 20px; }
",
        );
        html.push_str(
            "        h1, h2 { color: #333; }
",
        );
        html.push_str(
            "        table { border-collapse: collapse; width: 100%; margin: 20px 0; }
",
        );
        html.push_str(
            "        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
",
        );
        html.push_str(
            "        th { background-color: #f2f2f2; }
",
        );
        html.push_str("        .metric-card { background-color: #f8f9fa; padding: 15px; margin: 10px 0; border-radius: 5px; }
");
        html.push_str(
            "        .metric-value { font-size: 24px; font-weight: bold; color: #007bff; }
",
        );
        html.push_str(
            "    </style>
",
        );
        html.push_str(
            "</head>
",
        );
        html.push_str(
            "<body>
",
        );

        html.push_str(
            "    <h1>LinkML Schema Summary Report</h1>
",
        );

        if !schema.name.is_empty() {
            writeln!(html, "    <h2>Schema: {}</h2>", schema.name)
                .expect("writeln! to String should never fail");
        }

        if let Some(description) = &schema.description {
            writeln!(html, "    <p>{description}</p>")
                .expect("writeln! to String should never fail");
        }

        // Overview cards
        html.push_str(
            "    <div class=\"overview\">
",
        );
        writeln!(html, "        <div class=\"metric-card\">Classes: <span class=\"metric-value\">{}</span></div>", stats.class_count).expect("writeln! to String should never fail");
        writeln!(html, "        <div class=\"metric-card\">Slots: <span class=\"metric-value\">{}</span></div>", stats.slot_count).expect("writeln! to String should never fail");
        writeln!(html, "        <div class=\"metric-card\">Types: <span class=\"metric-value\">{}</span></div>", stats.type_count).expect("writeln! to String should never fail");
        writeln!(html, "        <div class=\"metric-card\">Enums: <span class=\"metric-value\">{}</span></div>", stats.enum_count).expect("writeln! to String should never fail");
        writeln!(html, "        <div class=\"metric-card\">Documentation: <span class=\"metric-value\">{:.1}%</span></div>", stats.documentation_coverage * 100.0).expect("writeln! to String should never fail");
        html.push_str(
            "    </div>
",
        );

        // Detailed statistics table
        html.push_str(
            "    <h2>Detailed Statistics</h2>
",
        );
        html.push_str(
            "    <table>
",
        );
        html.push_str(
            "        <tr><th>Metric</th><th>Value</th></tr>
",
        );
        writeln!(
            html,
            "        <tr><td>Abstract Classes</td><td>{}</td></tr>",
            stats.abstract_class_count
        )
        .expect("LinkML operation should succeed");
        writeln!(
            html,
            "        <tr><td>Mixin Classes</td><td>{}</td></tr>",
            stats.mixin_class_count
        )
        .expect("LinkML operation should succeed");
        writeln!(
            html,
            "        <tr><td>Max Inheritance Depth</td><td>{}</td></tr>",
            stats.max_inheritance_depth
        )
        .expect("LinkML operation should succeed");
        writeln!(
            html,
            "        <tr><td>Required Slots</td><td>{}</td></tr>",
            stats.required_slot_count
        )
        .expect("LinkML operation should succeed");
        writeln!(
            html,
            "        <tr><td>Multivalued Slots</td><td>{}</td></tr>",
            stats.multivalued_slot_count
        )
        .expect("LinkML operation should succeed");

        if self.config.complexity_metrics {
            writeln!(
                html,
                "        <tr><td>Schema Complexity Score</td><td>{:.2}</td></tr>",
                stats.schema_complexity_score
            )
            .expect("LinkML operation should succeed");
            writeln!(
                html,
                "        <tr><td>Cyclomatic Complexity</td><td>{}</td></tr>",
                stats.cyclomatic_complexity
            )
            .expect("LinkML operation should succeed");
        }

        html.push_str(
            "    </table>
",
        );

        html.push_str(
            "</body>
",
        );
        html.push_str(
            "</html>
",
        );

        Ok(html)
    }
}

impl Generator for SummaryGenerator {
    fn name(&self) -> &'static str {
        "summary"
    }

    fn description(&self) -> &'static str {
        "Generate summary reports from LinkML schemas"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for summary generation",
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<String> {
        self.generate_summary(schema)
    }

    fn get_file_extension(&self) -> &str {
        match self.config.format {
            SummaryFormat::Tsv => "tsv",
            SummaryFormat::Markdown => "md",
            SummaryFormat::Json => "json",
            SummaryFormat::Html => "html",
        }
    }

    fn get_default_filename(&self) -> &'static str {
        "schema_summary"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    #[test]
    fn test_summary_generation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut schema = SchemaDefinition {
            name: "TestSchema".to_string(),
            description: Some("A test schema".to_string()),
            ..Default::default()
        };

        // Add some classes
        let person_class = ClassDefinition {
            description: Some("A person".to_string()),
            abstract_: Some(false),
            slots: vec!["name".to_string(), "age".to_string()],
            ..Default::default()
        };

        let abstract_class = ClassDefinition {
            abstract_: Some(true),
            description: Some("An abstract class".to_string()),
            ..Default::default()
        };

        let mut classes = IndexMap::new();
        classes.insert("Person".to_string(), person_class);
        classes.insert("NamedThing".to_string(), abstract_class);
        schema.classes = classes;

        // Add some slots
        let name_slot = SlotDefinition {
            required: Some(true),
            description: Some("Name of the entity".to_string()),
            ..Default::default()
        };

        let age_slot = SlotDefinition {
            range: Some("integer".to_string()),
            ..Default::default()
        };

        let mut slots = IndexMap::new();
        slots.insert("name".to_string(), name_slot);
        slots.insert("age".to_string(), age_slot);
        schema.slots = slots;

        // Test TSV generation
        let config = SummaryGeneratorConfig::default();
        let generator = SummaryGenerator::new(config);
        let result = generator
            .generate(&schema)
            .expect("should generate summary: {}");

        assert!(result.contains("Total Classes\t2"));
        assert!(result.contains("Total Slots\t2"));
        assert!(result.contains("Abstract Classes\t1"));
        Ok(())
    }
}
