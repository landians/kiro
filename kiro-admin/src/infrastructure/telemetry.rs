use std::{collections::HashMap, sync::Mutex, time::Duration};

use anyhow::{Context, Result};
use opentelemetry::{KeyValue, global, metrics::MeterProvider, trace::TracerProvider};
use opentelemetry_otlp::{
    MetricExporter, Protocol, SpanExporter, WithExportConfig, WithHttpConfig, WithTonicConfig,
};
use opentelemetry_sdk::{
    Resource,
    metrics::{PeriodicReader, SdkMeterProvider},
    trace::SdkTracerProvider,
};
use tonic::metadata::MetadataMap;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::infrastructure::{
    config::{TelemetryConfig, TelemetryProtocol},
    observability::HttpObservability,
};

pub struct TelemetryBuilder {
    enabled: bool,
    service_name: String,
    service_namespace: Option<String>,
    service_version: String,
    tracer_name: String,
    endpoint: Option<String>,
    protocol: TelemetryProtocol,
    env_filter: String,
    export_interval: Duration,
    authorization: Option<String>,
    environment: Option<String>,
}

pub struct InstalledTelemetry {
    pub guard: TelemetryGuard,
    pub http_observability: HttpObservability,
}

pub struct TelemetryGuard {
    tracer_provider: Mutex<Option<SdkTracerProvider>>,
    meter_provider: Mutex<Option<SdkMeterProvider>>,
}

impl TelemetryBuilder {
    pub fn new(config: TelemetryConfig) -> Self {
        Self {
            enabled: config.enabled,
            service_name: config.service_name,
            service_namespace: config.service_namespace,
            service_version: config
                .service_version
                .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_owned()),
            tracer_name: config.tracer_name,
            endpoint: config.endpoint,
            protocol: config.protocol,
            env_filter: config.level,
            export_interval: Duration::from_secs(config.export_interval_seconds),
            authorization: config.authorization,
            environment: None,
        }
    }

    pub fn with_environment(mut self, environment: impl Into<String>) -> Self {
        self.environment = Some(environment.into());
        self
    }

    pub fn build(self) -> Result<InstalledTelemetry> {
        let meter_name: &'static str = Box::leak(self.service_name.clone().into_boxed_str());
        let env_filter = EnvFilter::try_new(&self.env_filter).context("invalid telemetry.level")?;

        if !self.enabled {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer())
                .try_init()
                .context("failed to install tracing subscriber")?;

            return Ok(InstalledTelemetry {
                guard: TelemetryGuard::default(),
                http_observability: HttpObservability::new(global::meter(meter_name)),
            });
        }

        let resource = self.build_resource();
        let tracer_provider = self
            .build_tracer_provider(resource.clone())
            .context("failed to build OTLP tracer provider")?;
        let meter_provider = self
            .build_meter_provider(resource)
            .context("failed to build OTLP meter provider")?;
        let tracer = tracer_provider.tracer(self.tracer_name.clone());
        let meter = meter_provider.meter(meter_name);

        global::set_tracer_provider(tracer_provider.clone());
        global::set_meter_provider(meter_provider.clone());

        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .try_init()
            .context("failed to install tracing subscriber")?;

        Ok(InstalledTelemetry {
            guard: TelemetryGuard::new(Some(tracer_provider), Some(meter_provider)),
            http_observability: HttpObservability::new(meter),
        })
    }

    fn build_resource(&self) -> Resource {
        let mut attributes = vec![
            KeyValue::new("service.name", self.service_name.clone()),
            KeyValue::new("service.version", self.service_version.clone()),
        ];

        if let Some(service_namespace) = &self.service_namespace {
            attributes.push(KeyValue::new(
                "service.namespace",
                service_namespace.clone(),
            ));
        }

        if let Some(environment) = &self.environment {
            attributes.push(KeyValue::new(
                "deployment.environment.name",
                environment.clone(),
            ));
        }

        Resource::builder().with_attributes(attributes).build()
    }

    fn build_tracer_provider(&self, resource: Resource) -> Result<SdkTracerProvider> {
        let endpoint = self
            .endpoint
            .clone()
            .context("telemetry.endpoint is required when telemetry.enabled = true")?;
        let exporter = match self.protocol {
            TelemetryProtocol::Grpc => {
                let mut exporter = SpanExporter::builder().with_tonic().with_endpoint(endpoint);

                if let Some(authorization) = self.authorization.as_deref() {
                    exporter = exporter.with_metadata(build_grpc_metadata(authorization)?);
                }

                exporter
                    .build()
                    .context("failed to build gRPC OTLP span exporter")?
            }
            TelemetryProtocol::Http => {
                let mut exporter = SpanExporter::builder()
                    .with_http()
                    .with_protocol(Protocol::HttpBinary)
                    .with_endpoint(endpoint);

                if let Some(authorization) = self.authorization.as_deref() {
                    exporter = exporter.with_headers(build_http_headers(authorization));
                }

                exporter
                    .build()
                    .context("failed to build HTTP OTLP span exporter")?
            }
        };

        Ok(SdkTracerProvider::builder()
            .with_resource(resource)
            .with_batch_exporter(exporter)
            .build())
    }

    fn build_meter_provider(&self, resource: Resource) -> Result<SdkMeterProvider> {
        let endpoint = self
            .endpoint
            .clone()
            .context("telemetry.endpoint is required when telemetry.enabled = true")?;
        let exporter = match self.protocol {
            TelemetryProtocol::Grpc => {
                let mut exporter = MetricExporter::builder()
                    .with_tonic()
                    .with_endpoint(endpoint);

                if let Some(authorization) = self.authorization.as_deref() {
                    exporter = exporter.with_metadata(build_grpc_metadata(authorization)?);
                }

                exporter
                    .build()
                    .context("failed to build gRPC OTLP metric exporter")?
            }
            TelemetryProtocol::Http => {
                let mut exporter = MetricExporter::builder()
                    .with_http()
                    .with_protocol(Protocol::HttpBinary)
                    .with_endpoint(endpoint);

                if let Some(authorization) = self.authorization.as_deref() {
                    exporter = exporter.with_headers(build_http_headers(authorization));
                }

                exporter
                    .build()
                    .context("failed to build HTTP OTLP metric exporter")?
            }
        };

        let reader = PeriodicReader::builder(exporter)
            .with_interval(self.export_interval)
            .build();

        Ok(SdkMeterProvider::builder()
            .with_resource(resource)
            .with_reader(reader)
            .build())
    }
}

impl TelemetryGuard {
    fn new(
        tracer_provider: Option<SdkTracerProvider>,
        meter_provider: Option<SdkMeterProvider>,
    ) -> Self {
        Self {
            tracer_provider: Mutex::new(tracer_provider),
            meter_provider: Mutex::new(meter_provider),
        }
    }

    pub fn shutdown(&self) {
        if let Some(tracer_provider) = self
            .tracer_provider
            .lock()
            .expect("telemetry tracer mutex poisoned")
            .take()
        {
            let _ = tracer_provider.shutdown();
        }

        if let Some(meter_provider) = self
            .meter_provider
            .lock()
            .expect("telemetry meter mutex poisoned")
            .take()
        {
            let _ = meter_provider.shutdown();
        }
    }
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl Default for TelemetryGuard {
    fn default() -> Self {
        Self::new(None, None)
    }
}

fn build_http_headers(authorization: &str) -> HashMap<String, String> {
    HashMap::from([("authorization".to_owned(), authorization.to_owned())])
}

fn build_grpc_metadata(authorization: &str) -> Result<MetadataMap> {
    let mut metadata = MetadataMap::new();
    let value = authorization
        .parse()
        .context("invalid telemetry.authorization header value")?;
    metadata.insert("authorization", value);

    Ok(metadata)
}
