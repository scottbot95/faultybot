use crate::settings::config::FaultybotConfig;
use metrics::{counter, describe_counter, describe_histogram, histogram, NoopRecorder};
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_exporter_statsd::StatsdBuilder;
use metrics_util::MetricKindMask;
use poise::serenity_prelude as serenity;
use std::error::Error;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tokio::time::Instant;
use tracing::{info, warn};
use crate::util::AuditInfo;

#[derive(Debug, Clone, Eq, PartialEq)]
struct CommandInvocationData<'a> {
    start: Instant,
    labels: Vec<(&'a str, String)>
}

pub(crate) fn init_metrics(settings: &FaultybotConfig) {
    if install_prometheus_recorder(settings).expect("Failed to install Prometheus recorder") {
        info!("Installed Prometheus metrics recorder");
        describe_metrics();
    } else if install_statsd_recorder(settings).expect("Failed to install StatsD recorder") {
        info!("Installed Statsd metrics recorder");
        // Nothing (yet?)
    } else {
        warn!("No metrics recorder specified, defaulting to NoopRecorder");
        metrics::set_global_recorder(NoopRecorder).expect("Failed to install NoopRecorder");
    }
}

/// Periodically emit metrics about bot state
pub(crate) fn periodic_metrics(cache: Arc<serenity::Cache>, period: Duration) {
    tokio::spawn(async move {
        let mut interval = time::interval(period);

        loop {
            interval.tick().await;

            let guild_count = cache.guild_count();
            histogram!("guilds_in_cache").record(guild_count as f64);
        }
    });
}

pub async fn record_command_start(ctx: crate::Context<'_>) {
    let start = Instant::now();

    let mut labels = AuditInfo::from(&ctx)
        .as_metric_labels();

    labels.extend([
        ("command_name", ctx.command().qualified_name.clone())
    ]);

    counter!("command_executions_total", &labels).increment(1);

    ctx.set_invocation_data(CommandInvocationData {
        start,
        labels,
    }).await;
}

pub async fn record_command_completion(ctx: crate::Context<'_>) {
    let invocation_data = ctx.invocation_data::<CommandInvocationData>().await;
    if invocation_data.is_none() { return; }
    let invocation_data = invocation_data.unwrap();

    let duration = invocation_data.start.elapsed();

    histogram!("command_execution_seconds", &invocation_data.labels).record(duration.as_secs_f64());
}

/// We register these metrics, which gives us a chance to specify a description for them.  The
/// Prometheus exporter records this description and adds it as HELP text when the endpoint is
/// scraped.
///
/// Registering metrics ahead of using them is not required, but is the only way to specify the
/// description of a metric.
fn describe_metrics() {
    describe_counter!("errors_total", "The total number of errors");
    describe_counter!(
        "access_denied_total",
        "The total number of Access Denied responses"
    );
    describe_counter!(
        "command_executions_total",
        "The total number of commands attempted to be run"
    );
    describe_histogram!(
        "command_execution_seconds",
        "The time in seconds taken for a command to execute"
    );
    describe_histogram!(
        "guilds_in_cache",
        "The number of guilds currently in the serenity cache. This value is shared across shards."
    );

    describe_counter!(
        "gpt_errors_total",
        "The total number of errors from the ChatGPT API"
    );
    describe_counter!(
        "gpt_requests_total",
        "The total number of response requests to FaultyGPT"
    );
    describe_counter!(
        "gpt_responses_total",
        "The total number of responses provided by FaultyGPT"
    );
    describe_histogram!(
        "gpt_response_seconds",
        "The time taken for a GPT a response to be generated by FaultyGPT"
    );
    describe_histogram!(
        "gpt_response_delay_seconds",
        "The total delay in seconds between a user sending a message and FaultyBot send a reply"
    );
}

fn install_prometheus_recorder(settings: &FaultybotConfig) -> Result<bool, Box<dyn Error>> {
    let address = match &settings.prometheus {
        Some(prometheus) => SocketAddr::from_str(&prometheus.listen)?,
        None => return Ok(false),
    };

    PrometheusBuilder::new()
        .idle_timeout(
            MetricKindMask::COUNTER | MetricKindMask::HISTOGRAM,
            Some(Duration::from_secs(10)),
        )
        .with_http_listener(address)
        .install()?;

    Ok(true)
}

fn install_statsd_recorder(settings: &FaultybotConfig) -> Result<bool, Box<dyn Error>> {
    let (host, port) = match settings.statsd.as_ref() {
        Some(statsd) => (statsd.host.clone(), statsd.port),
        None => return Ok(false),
    };

    let recorder = StatsdBuilder::from(host, port)
        .histogram_is_timer()
        .build(None)?;

    metrics::set_global_recorder(recorder)?;

    Ok(true)
}
