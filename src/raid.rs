use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::process::Command;

extern crate plist;

#[derive(Serialize)]
struct RaidStatus {
    uuid: String,
    name: String,
    status: String,
    devices: Vec<RaidMember>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RaidMember {
    #[serde(rename(serialize = "BSD Name", deserialize = "BSD Name"))]
    bsdname: String,
    #[serde(rename(serialize = "MemberStatus", deserialize = "MemberStatus"))]
    status: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct RaidSet {
    #[serde(rename(serialize = "AppleRAIDSetUUID", deserialize = "AppleRAIDSetUUID"))]
    uuid: Option<String>,
    #[serde(rename(serialize = "BSD Name", deserialize = "BSD Name"))]
    bsdname: Option<String>,
    #[serde(rename(serialize = "Members", deserialize = "Members"))]
    members: Vec<RaidMember>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DiskutilOutput {
    #[serde(rename(serialize = "AppleRAIDSets", deserialize = "AppleRAIDSets"))]
    raidsets: Option<Vec<RaidSet>>,
}

pub async fn run() {
    let mqtt_client = crate::mqtt::new_mqtt_client();
    let conn_opts = crate::mqtt::conn_opts();
    mqtt_client.client.connect(conn_opts).unwrap_or_else(|err| {
        panic!("Unable to connect: {:?}", err);
    });

    let raid_sets = get_raid_status();
    for raid_set in raid_sets {
        if raid_set.status != "Online" {
            let mut faulty_names: Vec<String> = Vec::new();
            for device in raid_set.devices {
                if device.status != "Online" {
                    faulty_names.push(device.bsdname);
                }
            }
            publish(&mqtt_client, "healthy", "false");
            publish(&mqtt_client, "faultydevices", &faulty_names.join(","));
        } else {
            publish(&mqtt_client, "healthy", "true");
            publish(&mqtt_client, "faultydevices", "");
            publish(&mqtt_client, "error", "");
        }
    }
    publish(
        &mqtt_client,
        "update_date",
        &Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    );
}

fn get_raid_status() -> Vec<RaidStatus> {
    let output = Command::new("diskutil")
        .arg("appleRAID")
        .arg("list")
        .arg("-plist")
        .output()
        .expect("failed to execute process");

    let reader: Cursor<Vec<u8>> = Cursor::new(output.stdout);
    let data: DiskutilOutput = plist::from_reader(reader).expect("failed to read diskutil output");

    let mut raid_status: Vec<RaidStatus> = Vec::new();

    for raidset in data.raidsets.unwrap_or_default() {
        let mut devices: Vec<RaidMember> = Vec::new();
        for member in raidset.members {
            devices.push(member);
        }
        raid_status.push(RaidStatus {
            uuid: raidset.uuid.unwrap(),
            name: raidset.bsdname.unwrap(),
            status: "online".to_string(),
            devices: devices,
        });
    }

    raid_status
}

fn publish(mqtt_client: &crate::mqtt::MqttClient, topic_suffix: &str, payload: &str) {
    let topic_prefix = "home/storage/raidstatus";
    let topic = format!("{}/{}", topic_prefix, topic_suffix);
    mqtt_client.publish(topic, payload).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_file() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
        <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
        <plist version="1.0">
        <dict>
            <key>AppleRAIDSets</key>
            <array>
                <dict>
                    <key>AppleRAIDSetUUID</key>
                    <string>29A25F24-BEA2-47BA-B0F9-323CDF5545EC</string>
                    <key>BSD Name</key>
                    <string>disk4</string>
                    <key>ChunkCount</key>
                    <integer>122083833</integer>
                    <key>ChunkSize</key>
                    <integer>32768</integer>
                    <key>Content</key>
                    <string>7C3457EF-0000-11AA-AA11-00306543ECAC</string>
                    <key>Level</key>
                    <string>Mirror</string>
                    <key>Members</key>
                    <array>
                        <dict>
                            <key>AppleRAIDMemberUUID</key>
                            <string>6604E412-FC8E-4BD6-A2B1-972D47793A82</string>
                            <key>BSD Name</key>
                            <string>disk3s2</string>
                            <key>MemberStatus</key>
                            <string>Online</string>
                        </dict>
                        <dict>
                            <key>AppleRAIDMemberUUID</key>
                            <string>E4036DC8-CC28-47BD-B448-3A923098EEF1</string>
                            <key>BSD Name</key>
                            <string>disk2s2</string>
                            <key>MemberStatus</key>
                            <string>Online</string>
                        </dict>
                    </array>
                    <key>Name</key>
                    <string>DataRaid</string>
                    <key>Rebuild</key>
                    <string>Automatic</string>
                    <key>Size</key>
                    <integer>4000443039744</integer>
                    <key>Spares</key>
                    <array/>
                    <key>Status</key>
                    <string>Online</string>
                </dict>
            </array>
        </dict>
        </plist>"#.as_bytes();
        let data: DiskutilOutput = plist::from_bytes(content).unwrap();

        let raidsets = data.raidsets.unwrap();

        assert_eq!(raidsets.len(), 1);
        assert_eq!(raidsets[0].bsdname.as_ref().unwrap(), "disk4");
        assert_eq!(raidsets[0].members.len(), 2);
    }
}
