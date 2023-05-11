use anyhow::Result;
use chrono::Utc;
use std::time::Duration;

struct HomeStation {
    id: i32,
    country: String,
    city: String,
}

#[derive(Debug, serde::Deserialize)]
struct Place {
    number: i32,
    bike_numbers: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct City {
    name: String,
    places: Vec<Place>,
}

#[derive(Debug, serde::Deserialize)]
struct Country {
    name: String,
    cities: Vec<City>,
}

#[derive(Debug, serde::Deserialize)]
struct Data {
    countries: Vec<Country>,
}

pub async fn run() {
    let mqtt_client = crate::mqtt::new_mqtt_client();
    let conn_opts = crate::mqtt::conn_opts();
    mqtt_client.client.connect(conn_opts).unwrap_or_else(|err| {
        panic!("Unable to connect: {:?}", err);
    });

    let data = get_nextbike_data()
        .await
        .unwrap_or_else(|_| Data { countries: vec![] });
    let bike_found = bike_finder(data).await.unwrap_or(false);

    publish(
        &mqtt_client,
        "e_cargo_available",
        match bike_found {
            true => "true",
            false => "false",
        },
    );
    publish(
        &mqtt_client,
        "update_date",
        &Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    );
}

async fn bike_finder(data: Data) -> Result<bool> {
    let home = HomeStation {
        id: 4101,
        country: "Germany".to_string(),
        city: "Leipzig".to_string(),
    };
    let mut found = false;
    let e_cargo_bikes = ["20091", "20095", "20096", "20111", "20118", "20119"];

    for c in data.countries.iter() {
        if c.name == home.country {
            for city in c.cities.iter() {
                if city.name == home.city {
                    for place in city.places.iter() {
                        if place.number == home.id {
                            for bn in place.bike_numbers.iter() {
                                for e in e_cargo_bikes.iter() {
                                    println!("comparing {} with {} ", e, bn);
                                    if e == bn {
                                        found = true
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(found)
}

async fn get_nextbike_data() -> Result<Data> {
    let client = reqwest::Client::builder()
        .timeout(Duration::new(1, 0))
        .build();

    let url =
        "https://maps.nextbike.net/maps/nextbike-live.json?city=1&domains=le&list_cities=0&bikes=0";
    let resp = client?.get(url).send().await?.json::<Data>().await?;

    Ok(resp)
}

fn publish(mqtt_client: &crate::mqtt::MqttClient, topic_suffix: &str, payload: &str) {
    let topic_prefix = "mobility/nextbike1";
    let topic = format!("{}/{}", topic_prefix, topic_suffix);
    mqtt_client.publish(topic, payload).unwrap();
}
