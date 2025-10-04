//! Example demonstrating Graphviz generation from LinkML schemas
//!
//! This example shows how to generate DOT format files that can be visualized
//! with Graphviz tools to create schema diagrams.

use linkml_core::prelude::*;
use linkml_service::generator::graphviz::{GraphvizLayout, GraphvizOptions, GraphvizStyle};
use linkml_service::generator::{Generator, GeneratorOptions, GraphvizGenerator};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Create a sample schema representing a simple e-commerce model
    let mut schema = SchemaDefinition::default();
    schema.name = "ECommerceSchema".to_string();
    schema.id = "https://example.org/ecommerce".to_string();
    schema.description = Some("A simple e-commerce schema".to_string());

    // Base class for all entities
    let mut entity = ClassDefinition::default();
    entity.abstract_ = Some(true);
    entity.description = Some("Base class for all entities".to_string());
    entity.slots = vec!["id".to_string(), "created_at".to_string()];
    schema.classes.insert("Entity".to_string(), entity);

    // Customer class
    let mut customer = ClassDefinition::default();
    customer.description = Some("A customer who can place orders".to_string());
    customer.is_a = Some("Entity".to_string());
    customer.slots = vec![
        "name".to_string(),
        "email".to_string(),
        "orders".to_string(),
    ];
    schema.classes.insert("Customer".to_string(), customer);

    // Product class
    let mut product = ClassDefinition::default();
    product.description = Some("A product that can be ordered".to_string());
    product.is_a = Some("Entity".to_string());
    product.slots = vec![
        "name".to_string(),
        "price".to_string(),
        "category".to_string(),
    ];
    schema.classes.insert("Product".to_string(), product);

    // Order class
    let mut order = ClassDefinition::default();
    order.description = Some("An order placed by a customer".to_string());
    order.is_a = Some("Entity".to_string());
    order.slots = vec![
        "customer".to_string(),
        "order_items".to_string(),
        "total".to_string(),
        "status".to_string(),
    ];
    schema.classes.insert("Order".to_string(), order);

    // OrderItem class
    let mut order_item = ClassDefinition::default();
    order_item.description = Some("A line item in an order".to_string());
    order_item.slots = vec![
        "product".to_string(),
        "quantity".to_string(),
        "price".to_string(),
    ];
    schema.classes.insert("OrderItem".to_string(), order_item);

    // Define slots
    let mut id_slot = SlotDefinition::default();
    id_slot.identifier = Some(true);
    id_slot.range = Some("string".to_string());
    id_slot.required = Some(true);
    schema.slots.insert("id".to_string(), id_slot);

    let mut created_at_slot = SlotDefinition::default();
    created_at_slot.range = Some("datetime".to_string());
    schema
        .slots
        .insert("created_at".to_string(), created_at_slot);

    let mut name_slot = SlotDefinition::default();
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    schema.slots.insert("name".to_string(), name_slot);

    let mut email_slot = SlotDefinition::default();
    email_slot.range = Some("string".to_string());
    email_slot.pattern = Some(r"^\S+@\S+\.\S+$".to_string());
    schema.slots.insert("email".to_string(), email_slot);

    let mut price_slot = SlotDefinition::default();
    price_slot.range = Some("decimal".to_string());
    price_slot.minimum_value = Some(serde_json::json!(0));
    schema.slots.insert("price".to_string(), price_slot);

    let mut customer_slot = SlotDefinition::default();
    customer_slot.range = Some("Customer".to_string());
    customer_slot.required = Some(true);
    schema.slots.insert("customer".to_string(), customer_slot);

    let mut orders_slot = SlotDefinition::default();
    orders_slot.range = Some("Order".to_string());
    orders_slot.multivalued = Some(true);
    schema.slots.insert("orders".to_string(), orders_slot);

    let mut order_items_slot = SlotDefinition::default();
    order_items_slot.range = Some("OrderItem".to_string());
    order_items_slot.multivalued = Some(true);
    order_items_slot.required = Some(true);
    schema
        .slots
        .insert("order_items".to_string(), order_items_slot);

    let mut product_slot = SlotDefinition::default();
    product_slot.range = Some("Product".to_string());
    product_slot.required = Some(true);
    schema.slots.insert("product".to_string(), product_slot);

    let mut quantity_slot = SlotDefinition::default();
    quantity_slot.range = Some("integer".to_string());
    quantity_slot.minimum_value = Some(serde_json::json!(1));
    schema.slots.insert("quantity".to_string(), quantity_slot);

    let mut total_slot = SlotDefinition::default();
    total_slot.range = Some("decimal".to_string());
    schema.slots.insert("total".to_string(), total_slot);

    let mut status_slot = SlotDefinition::default();
    status_slot.range = Some("OrderStatus".to_string());
    schema.slots.insert("status".to_string(), status_slot);

    let mut category_slot = SlotDefinition::default();
    category_slot.range = Some("ProductCategory".to_string());
    schema.slots.insert("category".to_string(), category_slot);

    // Define enumerations
    let mut order_status = EnumDefinition::default();
    order_status.description = Some("Status of an order".to_string());
    order_status.permissible_values = vec![
        PermissibleValue::Simple("PENDING".to_string()),
        PermissibleValue::Simple("PROCESSING".to_string()),
        PermissibleValue::Simple("SHIPPED".to_string()),
        PermissibleValue::Simple("DELIVERED".to_string()),
        PermissibleValue::Simple("CANCELLED".to_string()),
    ];
    schema.enums.insert("OrderStatus".to_string(), order_status);

    let mut product_category = EnumDefinition::default();
    product_category.description = Some("Product categories".to_string());
    product_category.permissible_values = vec![
        PermissibleValue::Simple("ELECTRONICS".to_string()),
        PermissibleValue::Simple("CLOTHING".to_string()),
        PermissibleValue::Simple("BOOKS".to_string()),
        PermissibleValue::Simple("HOME".to_string()),
    ];
    schema
        .enums
        .insert("ProductCategory".to_string(), product_category);

    println!(
        "=== Graphviz Generation Examples ===
"
    );

    // Example 1: Simple style
    println!("1. Simple Diagram Style:");
    println!("------------------------");
    let simple_generator = GraphvizGenerator::new().with_style(GraphvizStyle::Simple);
    let result = simple_generator
        .generate(&schema, &GeneratorOptions::default())
        .await?;
    println!("Generated: {}", result[0].filename);
    println!(
        "Preview (first 300 chars):
{}
...
",
        result[0].content.chars().take(300).collect::<String>()
    );

    // Example 2: UML style
    println!("2. UML Class Diagram Style:");
    println!("---------------------------");
    let uml_generator = GraphvizGenerator::new().with_style(GraphvizStyle::Uml);
    let result = uml_generator
        .generate(&schema, &GeneratorOptions::default())
        .await?;
    println!("Generated: {}", result[0].filename);
    // Save this one to show full content
    std::fs::write("ecommerce_uml.dot", &result[0].content)?;
    println!("Full UML diagram saved to: ecommerce_uml.dot");
    println!(
        "To render: dot -Tpng ecommerce_uml.dot -o ecommerce_uml.png
"
    );

    // Example 3: Entity-Relationship style
    println!("3. Entity-Relationship Style:");
    println!("-----------------------------");
    let er_generator = GraphvizGenerator::new().with_style(GraphvizStyle::EntityRelationship);
    let result = er_generator
        .generate(&schema, &GeneratorOptions::default())
        .await?;
    std::fs::write("ecommerce_er.dot", &result[0].content)?;
    println!(
        "ER diagram saved to: ecommerce_er.dot
"
    );

    // Example 4: Hierarchical with different layout
    println!("4. Hierarchical Style with Circular Layout:");
    println!("-------------------------------------------");
    let custom_options = GraphvizOptions {
        style: GraphvizStyle::Hierarchical,
        layout: GraphvizLayout::Circo,
        include_slots: true,
        include_enums: true,
        include_types: false,
        show_cardinality: true,
        show_inheritance: true,
        show_mixins: false,
        use_colors: true,
        rankdir: "TB".to_string(),
    };
    let hierarchical_generator = GraphvizGenerator::with_options(custom_options);
    let result = hierarchical_generator
        .generate(&schema, &GeneratorOptions::default())
        .await?;
    std::fs::write("ecommerce_hierarchical.dot", &result[0].content)?;
    println!(
        "Hierarchical diagram saved to: ecommerce_hierarchical.dot
"
    );

    // Example 5: Left-to-right layout
    println!("5. Left-to-Right UML Layout:");
    println!("----------------------------");
    let lr_options = GraphvizOptions {
        style: GraphvizStyle::Uml,
        layout: GraphvizLayout::Dot,
        include_slots: true,
        include_enums: false, // Don't show enums for cleaner diagram
        include_types: false,
        show_cardinality: true,
        show_inheritance: true,
        show_mixins: false,
        use_colors: true,
        rankdir: "LR".to_string(), // Left to right
    };
    let lr_generator = GraphvizGenerator::with_options(lr_options);
    let result = lr_generator
        .generate(&schema, &GeneratorOptions::default())
        .await?;
    std::fs::write("ecommerce_lr.dot", &result[0].content)?;
    println!(
        "Left-to-right diagram saved to: ecommerce_lr.dot
"
    );

    println!("✅ Graphviz generation complete!");
    println!(
        "
To render the diagrams, use Graphviz tools:"
    );
    println!("  dot -Tpng file.dot -o file.png    # For hierarchical layouts");
    println!("  neato -Tpng file.dot -o file.png  # For spring model layouts");
    println!("  fdp -Tpng file.dot -o file.png    # For force-directed layouts");
    println!("  circo -Tpng file.dot -o file.png  # For circular layouts");
    println!("  twopi -Tpng file.dot -o file.png  # For radial layouts");
    println!(
        "
Or view online at: https://dreampuf.github.io/GraphvizOnline/"
    );

    Ok(())
}
