#![allow(clippy::redundant_field_names)]
use std::time::Duration;
use tokio::{task, time};

mod cityflitzer;
mod mqtt;
mod nextbike;
mod raid;

#[tokio::main]
async fn main() {
    env_logger::init();
    let forever = task::spawn(async {
        let mut interval = time::interval(Duration::from_millis(5000));

        loop {
            nextbike::run().await;
            cityflitzer::run().await;
            raid::run().await;
            interval.tick().await;
        }
    });

    forever.await.unwrap()
}
