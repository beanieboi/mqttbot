use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::process::Command;
use tracing::info;

extern crate plist;

#[derive(Serialize, Debug, Clone)]
struct RaidStatus {
    uuid: String,
    name: String,
    status: String,
    devices: Vec<RaidMember>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    #[serde(rename(serialize = "Name", deserialize = "Name"))]
    name: Option<String>,
    #[serde(rename(serialize = "Members", deserialize = "Members"))]
    members: Vec<RaidMember>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DiskutilOutput {
    #[serde(rename(serialize = "AppleRAIDSets", deserialize = "AppleRAIDSets"))]
    raidsets: Option<Vec<RaidSet>>,
}

pub async fn run(mqtt_client: &paho_mqtt::Client) {
    let raid_sets_status = get_raid_status(get_diskutil_output());
    for raid_set_status in raid_sets_status {
        let raid_name = raid_set_status.name.clone();
        match check_raid_health(raid_set_status) {
            Ok(_) => {
                publish(
                    mqtt_client,
                    crate::mqtt::Payload {
                        topic_suffix: "healthy".to_string(),
                        payload: "true".to_string(),
                    },
                );

                publish(
                    mqtt_client,
                    crate::mqtt::Payload {
                        topic_suffix: "faultydevices".to_string(),
                        payload: "".to_string(),
                    },
                );

                publish(
                    mqtt_client,
                    crate::mqtt::Payload {
                        topic_suffix: "error".to_string(),
                        payload: "".to_string(),
                    },
                );

                info!(raid_name, "healthy");
            }
            Err(e) => {
                publish(
                    mqtt_client,
                    crate::mqtt::Payload {
                        topic_suffix: "healthy".to_string(),
                        payload: "false".to_string(),
                    },
                );

                publish(
                    mqtt_client,
                    crate::mqtt::Payload {
                        topic_suffix: "healthy".to_string(),
                        payload: e,
                    },
                );

                info!(raid_name, "unhealthy");
            }
        }
    }

    publish(
        mqtt_client,
        crate::mqtt::Payload {
            topic_suffix: "update_date".to_string(),
            payload: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        },
    );
}

fn check_raid_health(raid_set_status: RaidStatus) -> Result<bool, String> {
    if raid_set_status.status != "Online" {
        let mut faulty_names: Vec<String> = Vec::new();
        for device in raid_set_status.devices {
            if device.status != "Online" {
                faulty_names.push(device.bsdname);
            }
        }
        Err(faulty_names.join(","))
    } else {
        Ok(true)
    }
}

fn get_diskutil_output() -> DiskutilOutput {
    let output = Command::new("diskutil")
        .arg("appleRAID")
        .arg("list")
        .arg("-plist")
        .output()
        .expect("failed to execute process");

    let reader: Cursor<Vec<u8>> = Cursor::new(output.stdout);
    plist::from_reader(reader).expect("failed to read diskutil output")
}

fn get_raid_status(disk_util_output: DiskutilOutput) -> Vec<RaidStatus> {
    let mut raid_status: Vec<RaidStatus> = Vec::new();

    for raidset in disk_util_output.raidsets.unwrap_or_default() {
        let mut devices: Vec<RaidMember> = Vec::new();
        for member in raidset.members {
            devices.push(member);
        }
        raid_status.push(RaidStatus {
            uuid: raidset.uuid.unwrap(),
            name: raidset.name.unwrap(),
            status: "Online".to_string(),
            devices: devices,
        });
    }

    raid_status
}

fn publish(mqtt_client: &paho_mqtt::Client, payload: crate::mqtt::Payload) {
    mqtt_client
        .publish(payload.to_msg("home/storage/raidstatus"))
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
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
    </plist>"#;

    #[test]
    fn parse_file() {
        let data: DiskutilOutput = plist::from_bytes(CONTENT.as_bytes()).unwrap();
        let raidsets = data.raidsets.unwrap();
        assert_eq!(raidsets.len(), 1);
        assert_eq!(raidsets[0].bsdname.as_ref().unwrap(), "disk4");
        assert_eq!(raidsets[0].name.as_ref().unwrap(), "DataRaid");
        assert_eq!(raidsets[0].members.len(), 2);
    }

    #[test]
    fn test_get_raid_status() {
        let output: DiskutilOutput = plist::from_bytes(CONTENT.as_bytes()).unwrap();
        let raid_status = get_raid_status(output);

        let first_status = raid_status.first().unwrap();
        assert_eq!(raid_status.len(), 1);
        assert_eq!(first_status.uuid, "29A25F24-BEA2-47BA-B0F9-323CDF5545EC");
    }
    #[test]
    fn test_check_raid_health() {
        let output: DiskutilOutput = plist::from_bytes(CONTENT.as_bytes()).unwrap();
        let raid_status = get_raid_status(output);
        let first_status = raid_status.first().unwrap();
        let result = check_raid_health(first_status.clone());

        assert!(result.is_ok());
    }
}
