//! Example demonstrating LinkML CLI usage
//!
//! This example shows various ways to use the LinkML command-line interface.

fn main() {
    println!(
        "=== LinkML CLI Usage Examples ===
"
    );

    // Note: These are example commands that would be run from the command line
    // In a real scenario, you would run these directly in your terminal

    println!("1. Validate Data");
    println!("================");
    println!("linkml validate --schema person.yaml --data people.json");
    println!("linkml validate -s schema.yaml -d data1.yaml data2.yaml --parallel");
    println!(
        "linkml validate -s schema.yaml -d *.json --class Person --strict
"
    );

    println!("2. Generate Code");
    println!("================");
    println!("linkml generate -s schema.yaml -o output/ -g python");
    println!("linkml generate -s schema.yaml -o types.ts -g typescript --option module=true");
    println!("linkml generate -s schema.yaml -o schema.sql -g sql --option dialect=postgresql");
    println!(
        "linkml generate -s schema.yaml -o docs/ -g docs --template-dir templates/
"
    );

    println!("3. Convert Schema Formats");
    println!("=========================");
    println!("linkml convert -i schema.yaml -o schema.json --to json --pretty");
    println!("linkml convert -i schema.json -o schema.yaml --to yaml");
    println!(
        "linkml convert -i schema.yaml -o schema.jsonld --to jsonld --validate
"
    );

    println!("4. Merge Schemas");
    println!("================");
    println!("linkml merge schema1.yaml schema2.yaml -o merged.yaml");
    println!("linkml merge *.yaml -o combined.yaml --strategy union --conflict first");
    println!(
        "linkml merge base.yaml feature1.yaml feature2.yaml -o final.yaml --base base.yaml
"
    );

    println!("5. Compare Schemas (Diff)");
    println!("=========================");
    println!("linkml diff v1/schema.yaml v2/schema.yaml");
    println!("linkml diff old.yaml new.yaml -f markdown -o changes.md");
    println!("linkml diff prod.yaml dev.yaml --breaking-only");
    println!(
        "linkml diff schema1.yaml schema2.yaml -f html -o diff.html
"
    );

    println!("6. Lint Schema");
    println!("==============");
    println!("linkml lint schema.yaml");
    println!("linkml lint schema.yaml --fix");
    println!("linkml lint schema.yaml -r naming-convention -r missing-documentation");
    println!(
        "linkml lint schema.yaml -c lint-config.yaml --strict
"
    );

    println!("7. Load and Dump Data");
    println!("=====================");
    println!("# Load from CSV");
    println!("linkml load -s schema.yaml -i data.csv -f csv -o data.json");
    println!("linkml load -s schema.yaml -i data.csv -f csv -o data.yaml --option delimiter=';'");
    println!();
    println!("# Load from database");
    println!("linkml load -s schema.yaml -f database -o dump.json \\");
    println!("  --option connection='postgresql://user:pass@localhost/db'");
    println!();
    println!("# Load from API");
    println!("linkml load -s schema.yaml -f api -o users.json \\");
    println!("  --option url=https://api.example.com \\");
    println!("  --option endpoint=/users");
    println!();
    println!("# Dump to different formats");
    println!("linkml dump -s schema.yaml -i data.json -o data.csv -f csv");
    println!("linkml dump -s schema.yaml -i data.json -o data.ttl -f rdf --option format=turtle");
    println!("linkml dump -s schema.yaml -i data.json -f database \\");
    println!("  --option connection='postgresql://user:pass@localhost/db' \\");
    println!(
        "  --option create_tables=true
"
    );

    println!("8. Start API Server");
    println!("===================");
    println!("linkml serve -s schema.yaml");
    println!("linkml serve -s schema.yaml -p 3000 --cors");
    println!("linkml serve -s schema.yaml --auth bearer --cert cert.pem --key key.pem");
    println!(
        "linkml serve -s schema.yaml -H 0.0.0.0 -p 8080 --docs-path /api-docs
"
    );

    println!("9. Interactive Shell");
    println!("====================");
    println!("linkml shell");
    println!("linkml shell -s schema.yaml");
    println!(
        "linkml shell --history ~/.linkml_history --init startup.linkml
"
    );

    println!("10. Advanced Examples");
    println!("=====================");

    println!("# Validate all JSON files in a directory");
    println!("linkml validate -s schema.yaml -d data/*.json --parallel --stats");
    println!();

    println!("# Generate multiple output formats");
    println!("for gen in python typescript sql graphql; do");
    println!("  linkml generate -s schema.yaml -o \"output.$gen\" -g \"$gen\"");
    println!("done");
    println!();

    println!("# Pipeline: Load CSV, validate, and dump to database");
    println!("linkml load -s schema.yaml -i users.csv -f csv -o - | \\");
    println!("  linkml validate -s schema.yaml -d - | \\");
    println!("  linkml dump -s schema.yaml -i - -f database \\");
    println!("    --option connection=$DATABASE_URL");
    println!();

    println!("# Merge multiple schemas with validation");
    println!("linkml merge base.yaml extension1.yaml extension2.yaml \\");
    println!("  -o merged.yaml --validate && \\");
    println!("  linkml lint merged.yaml --fix");
    println!();

    println!("# Generate documentation with custom templates");
    println!("linkml generate -s schema.yaml -o docs/ -g docs \\");
    println!("  --template-dir custom-templates/ \\");
    println!("  --option format=markdown \\");
    println!("  --option include_examples=true");
    println!();

    println!("# Complex data transformation");
    println!("linkml load -s source-schema.yaml -i legacy.xml -f xml -o - | \\");
    println!("  linkml convert --from json --to yaml | \\");
    println!("  linkml dump -s target-schema.yaml -i - -f api \\");
    println!("    --option url=https://api.example.com/import \\");
    println!("    --option method=POST");

    println!(
        "
✅ These examples demonstrate the full power of the LinkML CLI!"
    );
    println!(
        "
Key features:"
    );
    println!("- Multiple input/output formats");
    println!("- Schema validation and linting");
    println!("- Code generation for multiple languages");
    println!("- Data transformation pipelines");
    println!("- Schema versioning and migration");
    println!("- API server for schema operations");
    println!("- Interactive development shell");
}
