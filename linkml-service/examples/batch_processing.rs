//! Batch Processing Example
//! 
//! This example demonstrates high-performance batch processing patterns:
//! - Parallel validation of multiple documents
//! - Streaming large datasets
//! - Progress tracking
//! - Error aggregation
//! - Performance optimization techniques

use linkml_core::prelude::*;
use linkml_service::prelude::*;
use serde_json::json;
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use tokio::sync::Semaphore;
use indicatif::{ProgressBar, ProgressStyle};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("LinkML Batch Processing Example");
    println!("==============================\n");

    // Schema for customer records
    let schema_yaml = r#"
id: https://example.com/customer
name: CustomerSchema
description: Customer data schema for batch processing

classes:
  Customer:
    description: Customer record
    slots:
      - customer_id
      - name
      - email
      - phone
      - address
      - registration_date
      - status
      - credit_score
      - total_purchases
    slot_usage:
      customer_id:
        identifier: true
      email:
        pattern: "^[\\w.-]+@[\\w.-]+\\.\\w+$"
      credit_score:
        minimum_value: 300
        maximum_value: 850

  Address:
    description: Customer address
    slots:
      - street
      - city
      - state
      - postal_code
      - country

slots:
  customer_id:
    range: string
    pattern: "^CUST-\\d{8}$"
  name:
    range: string
    required: true
  email:
    range: string
    required: true
  phone:
    range: string
  street:
    range: string
  city:
    range: string
  state:
    range: string
  postal_code:
    range: string
  country:
    range: string
  address:
    range: Address
  registration_date:
    range: date
  status:
    range: CustomerStatus
  credit_score:
    range: integer
  total_purchases:
    range: decimal
    minimum_value: 0

enums:
  CustomerStatus:
    permissible_values:
      active:
      inactive:
      suspended:
      vip:
"#;

    // Parse schema
    let parser = YamlParser::new();
    let schema = parser.parse_str(schema_yaml)?;
    let service = Arc::new(create_example_linkml_service().await?);

    // Example 1: Simple batch validation
    println!("Example 1: Simple Batch Validation");
    println!("---------------------------------");
    
    let customers = generate_customer_batch(100);
    let start = std::time::Instant::now();
    
    // Validate all customers in parallel
    let validation_futures: Vec<_> = customers
        .iter()
        .map(|customer| {
            let service = service.clone();
            let schema = schema.clone();
            let customer = customer.clone();
            async move {
                service.validate(&customer, &schema, "Customer").await
            }
        })
        .collect();
    
    let results = futures::future::join_all(validation_futures).await;
    let duration = start.elapsed();
    
    let valid_count = results.iter().filter(|r| r.as_ref().unwrap().valid).count();
    let invalid_count = results.len() - valid_count;
    
    println!("Validated {} customers in {:?}", results.len(), duration);
    println!("Valid: {}, Invalid: {}", valid_count, invalid_count);
    println!("Throughput: {:.0} records/sec", results.len() as f64 / duration.as_secs_f64());
    
    // Example 2: Controlled concurrency with semaphore
    println!("\n\nExample 2: Controlled Concurrency");
    println!("---------------------------------");
    
    let customers = generate_customer_batch(1000);
    let semaphore = Arc::new(Semaphore::new(50)); // Limit to 50 concurrent validations
    let progress = ProgressBar::new(customers.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-")
    );
    
    let start = std::time::Instant::now();
    let validation_stream = stream::iter(customers)
        .map(|customer| {
            let service = service.clone();
            let schema = schema.clone();
            let semaphore = semaphore.clone();
            let progress = progress.clone();
            
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                let result = service.validate(&customer, &schema, "Customer").await;
                progress.inc(1);
                result
            }
        })
        .buffer_unordered(100);
    
    let results: Vec<_> = validation_stream.collect().await;
    let duration = start.elapsed();
    progress.finish();
    
    println!("Processed {} records in {:?}", results.len(), duration);
    println!("Throughput: {:.0} records/sec", results.len() as f64 / duration.as_secs_f64());
    
    // Example 3: Streaming validation with error aggregation
    println!("\n\nExample 3: Streaming with Error Aggregation");
    println!("------------------------------------------");
    
    let customers = generate_customer_batch(500);
    let mut error_summary: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    
    let validation_stream = stream::iter(customers)
        .enumerate()
        .map(|(idx, customer)| {
            let service = service.clone();
            let schema = schema.clone();
            
            async move {
                let result = service.validate(&customer, &schema, "Customer").await?;
                Ok::<_, Box<dyn std::error::Error>>((idx, result))
            }
        })
        .buffer_unordered(50);
    
    tokio::pin!(validation_stream);
    
    while let Some(result) = validation_stream.next().await {
        match result {
            Ok((idx, report)) => {
                if !report.valid {
                    for error in &report.errors {
                        let key = error.field.clone().unwrap_or_else(|| "general".to_string());
                        *error_summary.entry(key).or_insert(0) += 1;
                    }
                    
                    if error_summary.values().sum::<usize>() <= 5 {
                        println!("Record {}: {} errors", idx, report.errors.len());
                    }
                }
            }
            Err(e) => eprintln!("Validation error: {}", e),
        }
    }
    
    println!("\nError Summary:");
    for (field, count) in error_summary.iter() {
        println!("  {}: {} errors", field, count);
    }
    
    // Example 4: Batch processing with chunking
    println!("\n\nExample 4: Chunked Batch Processing");
    println!("-----------------------------------");
    
    let large_dataset = generate_customer_batch(5000);
    let chunk_size = 250;
    let chunks: Vec<_> = large_dataset.chunks(chunk_size).collect();
    
    println!("Processing {} records in {} chunks of {}", 
            large_dataset.len(), chunks.len(), chunk_size);
    
    let start = std::time::Instant::now();
    for (chunk_idx, chunk) in chunks.iter().enumerate() {
        let chunk_start = std::time::Instant::now();
        
        let chunk_results: Vec<_> = futures::future::join_all(
            chunk.iter().map(|customer| {
                let service = service.clone();
                let schema = schema.clone();
                let customer = customer.clone();
                async move {
                    service.validate(&customer, &schema, "Customer").await
                }
            })
        ).await;
        
        let valid = chunk_results.iter().filter(|r| r.as_ref().unwrap().valid).count();
        let chunk_duration = chunk_start.elapsed();
        
        println!("Chunk {}: {} records in {:?} ({} valid)", 
                chunk_idx + 1, chunk.len(), chunk_duration, valid);
    }
    
    let total_duration = start.elapsed();
    println!("Total: {} records in {:?}", large_dataset.len(), total_duration);
    println!("Average throughput: {:.0} records/sec", 
            large_dataset.len() as f64 / total_duration.as_secs_f64());
    
    // Performance optimization tips
    println!("\n\nPerformance Optimization Tips");
    println!("============================");
    println!("1. Use parallel validation for independent documents");
    println!("2. Control concurrency with semaphores to avoid overwhelming the system");
    println!("3. Stream large datasets instead of loading all into memory");
    println!("4. Chunk very large datasets for better progress tracking");
    println!("5. Use buffer_unordered for maximum throughput");
    println!("6. Consider compiled schemas for repeated validations");
    println!("7. Profile and identify bottlenecks with tools like flamegraph");
    
    Ok(())
}

fn generate_customer_batch(count: usize) -> Vec<serde_json::Value> {
    (0..count)
        .map(|i| {
            let valid = i % 10 != 0; // Make 10% invalid
            json!({
                "customer_id": format!("CUST-{:08}", i),
                "name": format!("Customer {}", i),
                "email": if valid { 
                    format!("customer{}@example.com", i) 
                } else { 
                    "invalid-email" 
                },
                "phone": "+1-555-0100",
                "address": {
                    "street": "123 Main St",
                    "city": "Springfield",
                    "state": "IL",
                    "postal_code": "62701",
                    "country": "US"
                },
                "registration_date": "2024-01-15",
                "status": if i % 100 == 0 { "vip" } else { "active" },
                "credit_score": if valid { 700 } else { 200 }, // Invalid score
                "total_purchases": 1234.56
            })
        })
        .collect()
}

async fn create_example_linkml_service() -> Result<LinkMLService, Box<dyn std::error::Error>> {
    // In a real application, this would initialize with all dependencies
    todo!("Initialize LinkML service with dependencies")
}