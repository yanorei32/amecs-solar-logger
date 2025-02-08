use std::{
    io::Cursor,
    time::{SystemTime, UNIX_EPOCH, Duration},
};

mod amecs_solar;

use clap::{Parser, Subcommand};
use futures::prelude::*;
use influxdb2::{models::DataPoint, Client};
use tokio::time;

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    DryRun {
        #[command(flatten)]
        coord: amecs_solar::Coord,
    },

    Run {
        #[command(flatten)]
        coord: amecs_solar::Coord,

        #[command(flatten)]
        influx: InfluxConnectionInfo,
    },
}

#[derive(Debug, Parser)]
struct InfluxConnectionInfo {
    #[clap(long, env)]
    infl_host: String,

    #[clap(long, env)]
    infl_token: String,

    #[clap(long, env)]
    infl_org: String,

    #[clap(long, env)]
    infl_bucket: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = Cli::parse();
    tracing_subscriber::fmt().init();

    let (coord, influx): (_, Option<(Client, String)>) = match cli.command {
        Commands::DryRun { coord } => (coord, None),
        Commands::Run { coord, influx } => (
            coord,
            Some((
                Client::new(influx.infl_host, influx.infl_org, influx.infl_token),
                influx.infl_bucket,
            )),
        ),
    };

    let http_client = reqwest::ClientBuilder::new()
        .user_agent("otaku-handmade-hems-optimizer/0.1.0")
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap();

    let mut interval = time::interval(Duration::from_secs(60 * 60));

    loop {
        interval.tick().await;

        let http_client = http_client.clone();
        let influx = influx.clone();

        tokio::spawn(async move {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();

            let csv_str = http_client
                .get(format!(
                    "https://www.amecs.co.jp/solar/data/ndata.csv?{current_time}"
                ))
                .send()
                .await
                .expect("Failed to connect to amecs.co.jp")
                .error_for_status()
                .expect("Invalid resposne from server")
                .text()
                .await
                .expect("Failed to read response from amecs.co.jp");

            let data = amecs_solar::SolarData::try_new(Cursor::new(csv_str))
                .expect("Failed to parse CSV as SolarData");

            let (actual_coord, series) = data.nearest_series_data(coord);

            if series.is_empty() {
                tracing::info!("No datapoints obtained");
                return;
            }

            let from = series.first().unwrap().timestamp.to_rfc2822();
            let to = series.last().unwrap().timestamp.to_rfc2822();
            let total = series.len();

            tracing::info!("total {total} datapoints obtained ({actual_coord}). ({from} -> {to})");

            let series: Result<Vec<_>, _> = series
                .iter()
                .map(|dp| {
                    DataPoint::builder("amecs-solar")
                        .tag("coord", actual_coord.to_string())
                        .timestamp(dp.timestamp.timestamp_nanos_opt().unwrap())
                        .field("power", dp.power as f64)
                        .build()
                })
                .collect();

            let series = series.expect("Failed to rebuild as DataPoint");

            if let Some((influx_client, bucket)) = influx {
                influx_client
                    .write(&bucket, stream::iter(series.into_iter()))
                    .await
                    .expect("Failed to write datapoint");

                tracing::info!("Written!");
            };
        });
    }
}
