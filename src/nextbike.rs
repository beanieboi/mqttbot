use anyhow::Result;
use chrono::Utc;

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

pub async fn run(mqtt_client: &paho_mqtt::Client, client: &reqwest::Client) {
    let data = match get_data(client).await {
        Ok(data) => data,
        Err(err) => {
            panic!("Error getting nextbike data: {}", err);
        }
    };

    publish(
        mqtt_client,
        crate::mqtt::Payload {
            topic_suffix: "e_cargo_available".to_string(),
            payload: match bike_finder(data) {
                Some(_) => "true".to_string(),
                None => "false".to_string(),
            },
        },
    );

    publish(
        mqtt_client,
        crate::mqtt::Payload {
            topic_suffix: "update_date".to_string(),
            payload: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        },
    );
}

fn bike_finder(data: Data) -> Option<()> {
    let home = HomeStation {
        id: 4101,
        country: "Germany".to_string(),
        city: "Leipzig".to_string(),
    };
    let e_cargo_bikes = ["20091", "20095", "20096", "20111", "20118", "20119"];

    let c = data.countries.iter().find(|c| c.name == home.country)?;
    let city = c.cities.iter().find(|c| c.name == home.city)?;
    let place = city.places.iter().find(|p| p.number == home.id)?;
    let _ = place
        .bike_numbers
        .iter()
        .find(|bn| e_cargo_bikes.contains(&bn.as_str()))?;

    Some(())
}

async fn get_data(client: &reqwest::Client) -> Result<Data> {
    let url =
        "https://maps.nextbike.net/maps/nextbike-live.json?city=1&domains=le&list_cities=0&bikes=0";
    let resp = client.get(url).send().await?.json::<Data>().await?;

    Ok(resp)
}

fn publish(mqtt_client: &paho_mqtt::Client, payload: crate::mqtt::Payload) {
    mqtt_client
        .publish(payload.to_msg("mobility/nextbike"))
        .unwrap();
}
