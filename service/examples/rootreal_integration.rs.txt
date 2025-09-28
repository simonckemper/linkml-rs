//! RootReal integration example for LinkML service
//!
//! This example demonstrates:
//! - Integration with RootReal services
//! - Using LinkML with TypeDB
//! - Schema-driven development
//! - Service coordination
//! - Performance monitoring

use linkml_service::{create_linkml_service_with_config, LinkMLService};
use linkml_core::{prelude::*, error::Result, config::LinkMLConfig, traits::GenerationOperations};
use serde_json::json;
use std::sync::Arc;

// Simulated RootReal service imports (would be actual in real usage)
mod mock_rootreal {
    use std::sync::Arc;

    pub struct LoggerService;
    pub struct ConfigurationService;
    pub struct CacheService;
    pub struct TelemetryService;
    pub struct TypeDBService;

    impl LoggerService {
        pub fn new() -> Arc<Self> { Arc::new(Self) }
        pub fn info(&self, msg: &str) { println!("[INFO] {}", msg); }
        pub fn error(&self, msg: &str) { eprintln!("[ERROR] {}", msg); }
    }

    impl ConfigurationService {
        pub fn new() -> Arc<Self> { Arc::new(Self) }
        pub fn get(&self, key: &str) -> Option<String> {
            match key {
                "linkml.cache_enabled" => Some("true".to_string()),
                "linkml.validation_timeout" => Some("30".to_string()),
                _ => None,
            }
        }
    }

    impl CacheService {
        pub fn new() -> Arc<Self> { Arc::new(Self) }
        pub async fn get(&self, _key: &str) -> Option<Vec<u8>> { None }
        pub async fn set(&self, _key: &str, _value: Vec<u8>) -> Result<(), String> { Ok(()) }
    }

    impl TelemetryService {
        pub fn new() -> Arc<Self> { Arc::new(Self) }
        pub fn record_metric(&self, name: &str, value: f64) {
            println!("[METRIC] {} = {}", name, value);
        }
    }

    impl TypeDBService {
        pub fn new() -> Arc<Self> { Arc::new(Self) }
        pub async fn execute(&self, query: &str) -> Result<String, String> {
            Ok(format!("Executed: {}", query))
        }
    }
}

use mock_rootreal::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("LinkML RootReal Integration Example");
    println!("==================================\n");

    // Initialize RootReal services
    let logger = LoggerService::new();
    let config_service = ConfigurationService::new();
    let cache_service = CacheService::new();
    let telemetry_service = TelemetryService::new();
    let typedb_service = TypeDBService::new();

    logger.info("Initializing LinkML service with RootReal integration");

    // Configure LinkML service with RootReal services
    let mut config = LinkMLConfig::default();

    // Apply configuration from ConfigurationService
    if let Some(cache_enabled) = config_service.get("linkml.cache_enabled") {
        config.schema.enable_cache = cache_enabled == "true";
    }
    if let Some(timeout) = config_service.get("linkml.validation_timeout") {
        if let Ok(seconds) = timeout.parse::<u64>() {
            config.validation.timeout = std::time::Duration::from_secs(seconds);
        }
    }

    // Initialize RootReal services for LinkML (in production, these would be properly initialized)
    // Following dyn-compatibility guidelines
    use logger_core::LoggerService as LoggerTrait;
    use timestamp_core::TimestampService as TimestampTrait;
    use task_management_service::StandardTaskManagementService;
    use error_handling_service::StandardErrorHandlingService;
    use configuration_service::StandardConfigurationService;
    use cache_service::ValkeyCache;
    use monitoring_service::StandardMonitoringService;

    let logger_svc: Arc<dyn LoggerTrait<Error = logger_core::LoggerError>> =
        Arc::new(logger_service::StandardLoggerService::new()?);
    let timestamp: Arc<dyn TimestampTrait<Error = timestamp_core::TimestampError>> =
        Arc::new(timestamp_service::StandardTimestampService::new()?);

    // Non-dyn-compatible services use concrete types
    let task_manager = Arc::new(StandardTaskManagementService::new()?);
    let error_handler = Arc::new(StandardErrorHandlingService::new(
        logger_svc.clone(),
        timestamp.clone(),
    )?);
    let config_svc = Arc::new(StandardConfigurationService::new()?);

    // Dyn-compatible services
    let cache: Arc<dyn cache_core::CacheService<Error = cache_core::CacheError>> =
        Arc::new(ValkeyCache::new(
            cache_core::CacheConfig::default(),
            logger_svc.clone(),
            Arc::new(container_management_service::StandardContainerManagementService::new()?),
            task_manager.clone(),
            Arc::new(memory_service::StandardMemoryService::new()?),
        ).await?);

    let monitor: Arc<dyn monitoring_core::MonitoringService<Error = monitoring_core::MonitoringError>> =
        Arc::new(StandardMonitoringService::new(
            logger_svc.clone(),
            timestamp.clone(),
            task_manager.clone(),
        )?);

    let linkml_service = create_linkml_service_with_config(
        config,
        logger_svc,
        timestamp,
        task_manager,
        error_handler,
        config_svc,
        cache,
        monitor,
    ).await?;

    // Example 1: TypeDB-aligned schema
    println!("1. Creating TypeDB-aligned LinkML schema:");
    let typedb_schema_yaml = r#"
id: txp:digital/schemas/sensor-data
name: SensorDataSchema
description: Schema for IoT sensor data aligned with TypeDB
version: "1.0.0"

prefixes:
  txp: https://rootreal.org/txp/
  iot: https://rootreal.org/iot/

default_prefix: iot

classes:
  Device:
    description: IoT device that produces sensor readings
    class_uri: iot:Device
    attributes:
      - device_id
      - device_type
      - location
      - status
      - last_seen
    slots:
      - device_id
      - device_type
      - manufacturer
      - model
      - firmware_version
      - location
      - status
      - last_seen
      - readings

  SensorReading:
    description: Individual sensor measurement
    class_uri: iot:SensorReading
    attributes:
      - reading_id
      - timestamp
      - value
      - unit
    slots:
      - reading_id
      - device
      - sensor_type
      - timestamp
      - value
      - unit
      - quality
      - anomaly_detected

  Location:
    description: Geographic location
    class_uri: iot:Location
    slots:
      - location_id
      - name
      - latitude
      - longitude
      - altitude
      - timezone

  Anomaly:
    description: Detected anomaly in sensor data
    class_uri: iot:Anomaly
    slots:
      - anomaly_id
      - reading
      - anomaly_type
      - severity
      - detected_at
      - resolved_at
      - description

slots:
  device_id:
    identifier: true
    range: string
    pattern: "^DEV-[A-Z0-9]{8}$"
    required: true

  device_type:
    range: DeviceType
    required: true

  manufacturer:
    range: string

  model:
    range: string

  firmware_version:
    range: string
    pattern: "^\\d+\\.\\d+\\.\\d+$"

  location:
    range: Location
    required: true

  status:
    range: DeviceStatus
    required: true

  last_seen:
    range: datetime

  readings:
    range: SensorReading
    multivalued: true

  reading_id:
    identifier: true
    range: string
    required: true

  device:
    range: Device
    required: true

  sensor_type:
    range: SensorType
    required: true

  timestamp:
    range: datetime
    required: true

  value:
    range: float
    required: true

  unit:
    range: string
    required: true

  quality:
    range: float
    minimum_value: 0
    maximum_value: 1

  anomaly_detected:
    range: boolean

  location_id:
    identifier: true
    range: string

  name:
    range: string
    required: true

  latitude:
    range: float
    minimum_value: -90
    maximum_value: 90
    required: true

  longitude:
    range: float
    minimum_value: -180
    maximum_value: 180
    required: true

  altitude:
    range: float

  timezone:
    range: string

  anomaly_id:
    identifier: true
    range: string

  reading:
    range: SensorReading
    required: true

  anomaly_type:
    range: AnomalyType
    required: true

  severity:
    range: SeverityLevel
    required: true

  detected_at:
    range: datetime
    required: true

  resolved_at:
    range: datetime

  description:
    range: string

enums:
  DeviceType:
    permissible_values:
      temperature_sensor: {}
      humidity_sensor: {}
      pressure_sensor: {}
      motion_sensor: {}
      light_sensor: {}
      air_quality_sensor: {}

  DeviceStatus:
    permissible_values:
      online: {}
      offline: {}
      maintenance: {}
      error: {}

  SensorType:
    permissible_values:
      temperature: {}
      humidity: {}
      pressure: {}
      motion: {}
      light: {}
      co2: {}
      pm25: {}

  AnomalyType:
    permissible_values:
      out_of_range: {}
      sudden_change: {}
      sensor_failure: {}
      communication_error: {}
      pattern_deviation: {}

  SeverityLevel:
    permissible_values:
      low: {}
      medium: {}
      high: {}
      critical: {}
"#;

    let start = std::time::Instant::now();
    let schema = linkml_service.load_schema_str(typedb_schema_yaml, SchemaFormat::Yaml).await?;
    let load_time = start.elapsed();

    logger.info(&format!("Schema loaded in {:?}", load_time));
    telemetry_service.record_metric("linkml.schema.load_time_ms", load_time.as_millis() as f64);

    // Cache the schema
    let schema_key = format!("linkml:schema:{}", schema.id);
    let schema_bytes = serde_json::to_vec(&schema)?;
    cache_service.set(&schema_key, schema_bytes).await
        .map_err(|e| LinkMLError::ServiceError(e))?;
    logger.info(&format!("Schema cached with key: {}", schema_key));

    // Example 2: Generate TypeQL from LinkML
    println!("\n2. Generating TypeQL schema:");
    let typeql = linkml_service.generate_typeql(&schema).await?;

    // Execute in TypeDB (simulated)
    let result = typedb_service.execute(&typeql).await
        .map_err(|e| LinkMLError::ServiceError(e))?;
    logger.info(&format!("TypeDB schema created: {}", result));

    // Example 3: Validate sensor data
    println!("\n3. Validating sensor data:");
    let sensor_data = json!({
        "device_id": "DEV-TEMP0001",
        "device_type": "temperature_sensor",
        "manufacturer": "SensorCorp",
        "model": "TC-100",
        "firmware_version": "2.1.3",
        "location": {
            "location_id": "LOC-001",
            "name": "Server Room A",
            "latitude": 37.7749,
            "longitude": -122.4194,
            "altitude": 15.5,
            "timezone": "America/Los_Angeles"
        },
        "status": "online",
        "last_seen": "2024-01-20T10:30:00Z",
        "readings": [
            {
                "reading_id": "READ-001",
                "device": {"device_id": "DEV-TEMP0001"},
                "sensor_type": "temperature",
                "timestamp": "2024-01-20T10:30:00Z",
                "value": 22.5,
                "unit": "celsius",
                "quality": 0.98,
                "anomaly_detected": false
            },
            {
                "reading_id": "READ-002",
                "device": {"device_id": "DEV-TEMP0001"},
                "sensor_type": "temperature",
                "timestamp": "2024-01-20T10:31:00Z",
                "value": 45.8,  // Anomalous high temperature
                "unit": "celsius",
                "quality": 0.95,
                "anomaly_detected": true
            }
        ]
    });

    let validation_start = std::time::Instant::now();
    let report = linkml_service.validate(&sensor_data, &schema, "Device").await?;
    let validation_time = validation_start.elapsed();

    telemetry_service.record_metric("linkml.validation.time_ms", validation_time.as_millis() as f64);
    telemetry_service.record_metric("linkml.validation.errors", report.errors.len() as f64);

    if report.valid {
        logger.info("Sensor data validation passed");
    } else {
        logger.error(&format!("Validation failed with {} errors", report.errors.len()));
        for error in &report.errors {
            logger.error(&format!("  - {}", error.message));
        }
    }

    // Example 4: Anomaly detection workflow
    println!("\n4. Anomaly detection workflow:");

    // Check for anomalies in readings
    if let Some(readings) = sensor_data["readings"].as_array() {
        for reading in readings {
            if reading["anomaly_detected"].as_bool().unwrap_or(false) {
                let anomaly = json!({
                    "anomaly_id": "ANOM-001",
                    "reading": reading,
                    "anomaly_type": "out_of_range",
                    "severity": "high",
                    "detected_at": "2024-01-20T10:31:05Z",
                    "description": "Temperature exceeded normal operating range"
                });

                let anomaly_report = linkml_service.validate(&anomaly, &schema, "Anomaly").await?;
                if anomaly_report.valid {
                    logger.info("Anomaly record created and validated");

                    // Store in TypeDB (simulated)
                    let typeql_insert = format!(
                        "insert $anomaly isa Anomaly, has anomaly_id '{}';",
                        anomaly["anomaly_id"].as_str().unwrap()
                    );
                    typedb_service.execute(&typeql_insert).await
                        .map_err(|e| LinkMLError::ServiceError(e))?;
                }
            }
        }
    }

    // Example 5: Performance monitoring
    println!("\n5. Performance monitoring:");

    // Batch validation performance test
    let batch_size = 100;
    let mut batch_data = Vec::new();

    for i in 0..batch_size {
        batch_data.push(json!({
            "reading_id": format!("READ-{:04}", i),
            "device": {"device_id": "DEV-TEMP0001"},
            "sensor_type": "temperature",
            "timestamp": "2024-01-20T10:30:00Z",
            "value": 20.0 + (i as f64 * 0.1),
            "unit": "celsius",
            "quality": 0.95,
            "anomaly_detected": false
        }));
    }

    let batch_start = std::time::Instant::now();
    let mut valid_count = 0;

    for data in &batch_data {
        let report = linkml_service.validate(data, &schema, "SensorReading").await?;
        if report.valid {
            valid_count += 1;
        }
    }

    let batch_time = batch_start.elapsed();
    let avg_time = batch_time.as_millis() as f64 / batch_size as f64;

    telemetry_service.record_metric("linkml.batch.total_time_ms", batch_time.as_millis() as f64);
    telemetry_service.record_metric("linkml.batch.avg_time_ms", avg_time);
    telemetry_service.record_metric("linkml.batch.throughput_per_sec", 1000.0 / avg_time);

    logger.info(&format!(
        "Batch validation complete: {}/{} valid, avg {:.2}ms per record",
        valid_count, batch_size, avg_time
    ));

    // Example 6: Schema evolution
    println!("\n6. Schema evolution tracking:");

    // Check if schema has changed (simulated)
    if let Some(cached_schema_bytes) = cache_service.get(&schema_key).await {
        let cached_schema: SchemaDefinition = serde_json::from_slice(&cached_schema_bytes)?;

        if cached_schema.version != schema.version {
            logger.info(&format!(
                "Schema version changed from {} to {}",
                cached_schema.version.as_deref().unwrap_or("none"),
                schema.version.as_deref().unwrap_or("none")
            ));

            // Generate migration plan
            // Note: analyze_changes would require migration service integration
            // let migration = linkml_service.analyze_changes(&cached_schema, &schema).await?;
            // logger.info(&format!("Migration analysis: {} changes detected", migration.len()));
            logger.info("Schema version change detected - migration analysis would be performed here");
        }
    }

    // Example 7: Integration with RootReal error handling
    println!("\n7. Error handling integration:");

    let invalid_data = json!({
        "device_id": "INVALID-ID",  // Wrong pattern
        "device_type": "unknown_sensor",  // Not in enum
        "status": "running",  // Not in enum
        "location": {
            "latitude": 200,  // Out of range
            "longitude": -300  // Out of range
        }
    });

    let error_report = linkml_service.validate(&invalid_data, &schema, "Device").await?;

    if !error_report.valid {
        logger.error("Validation errors detected:");
        for error in &error_report.errors {
            logger.error(&format!(
                "  Field: {}, Error: {}",
                error.path.as_deref().unwrap_or("root"),
                error.message
            ));

            // Record error metrics
            telemetry_service.record_metric(
                &format!("linkml.validation.error.{}", error.error_type),
                1.0
            );
        }
    }

    println!("\n✓ RootReal integration example completed successfully!");

    // Final metrics summary
    println!("\nPerformance Summary:");
    println!("  Schema load time: {:.2}ms", load_time.as_millis());
    println!("  Single validation: {:.2}ms", validation_time.as_millis());
    println!("  Batch validation avg: {:.2}ms", avg_time);
    println!("  Throughput: {:.0} validations/sec", 1000.0 / avg_time);

    Ok(())
}
