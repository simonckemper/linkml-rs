//! Tests for LinkML metrics integration with the monitoring service

use std::sync::Arc;
use std::time::Duration;

use linkml_core::error::Result;
use linkml_service::monitoring_integration::LinkMLMetrics;

mod mock_services;
use mock_services::MockMonitoringService;

fn metric_value(map: &std::collections::HashMap<String, f64>, key: &str) -> f64 {
    map.get(key).copied().unwrap_or_default()
}

#[tokio::test]
async fn track_validation_records_success_metrics() -> Result<()> {
    let monitoring = Arc::new(MockMonitoringService::new());
    let metrics = LinkMLMetrics::new(monitoring.clone());

    metrics
        .track_validation(
            "test-schema",
            2048,
            Duration::from_millis(150),
            true,
        )
        .await?;

    let stored = monitoring.get_all_metrics().await;
    assert_eq!(metric_value(&stored, "linkml.validation.success_total"), 1.0);
    assert_eq!(
        metric_value(&stored, "linkml.validation.data_size_bytes"),
        2048.0
    );
    assert_eq!(metric_value(&stored, "linkml.validation.failure_total"), 0.0);
    assert_eq!(metric_value(&stored, "validation_duration_ms"), 150.0);

    Ok(())
}

#[tokio::test]
async fn track_validation_records_failure_counter() -> Result<()> {
    let monitoring = Arc::new(MockMonitoringService::new());
    let metrics = LinkMLMetrics::new(monitoring.clone());

    metrics
        .track_validation(
            "test-schema",
            512,
            Duration::from_millis(75),
            false,
        )
        .await?;

    let stored = monitoring.get_all_metrics().await;
    assert_eq!(metric_value(&stored, "linkml.validation.failure_total"), 1.0);
    assert_eq!(metric_value(&stored, "linkml.validation.success_total"), 0.0);
    assert_eq!(metric_value(&stored, "validation_duration_ms"), 75.0);

    Ok(())
}

#[tokio::test]
async fn track_generation_records_metrics() -> Result<()> {
    let monitoring = Arc::new(MockMonitoringService::new());
    let metrics = LinkMLMetrics::new(monitoring.clone());

    metrics
        .track_generation(
            "rust",
            "test-schema",
            Duration::from_millis(90),
            5,
            true,
        )
        .await?;

    let stored = monitoring.get_all_metrics().await;
    assert_eq!(metric_value(&stored, "linkml.generation.files_generated"), 5.0);
    assert_eq!(metric_value(&stored, "linkml.generation.success_total"), 1.0);
    assert_eq!(metric_value(&stored, "linkml.generation.failure_total"), 0.0);
    assert_eq!(metric_value(&stored, "generation_duration_ms"), 90.0);

    Ok(())
}
