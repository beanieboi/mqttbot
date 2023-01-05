package raid

import (
	"bytes"
	"fmt"
	"io"
	"os/exec"
	"strings"
	"time"

	MQTT "github.com/eclipse/paho.mqtt.golang"
	log "github.com/sirupsen/logrus"
	"howett.net/plist"
)

type RaidStatus struct {
	UUID    string
	Name    string
	Status  string
	Devices []RaidMember
}

type RaidMember struct {
	UUID                string `plist:"AppleRAIDMemberUUID"`
	BSDName             string `plist:"BSD Name"`
	MemberRebuildStatus int    `plist:"MemberRebuildStatus"`
	Status              string `plist:"MemberStatus"`
}

type RaidSet struct {
	UUID       string `plist:"AppleRAIDSetUUID"`
	Name       string
	Rebuild    string
	Size       uint64
	BSDName    string `plist:"BSD Name"`
	ChunkCount uint64
	ChunkSize  uint64
	Content    string
	Level      string
	Members    []RaidMember
	Spares     []RaidMember
	Status     string
}

func (r RaidStatus) UnhealthyDevices() []RaidMember {
	var devices []RaidMember
	for _, d := range r.Devices {
		if d.Status != "Online" {
			devices = append(devices, d)
		}
	}
	return devices
}

type DiskutilOutput struct {
	RaidSets []RaidSet `plist:"AppleRAIDSets"`
}

func Runner(mqttClient MQTT.Client) {
	raid, err := DiskStatus()

	if err != nil {
		setError(mqttClient, err)
	}

	for _, s := range raid {
		if s.Status != "Online" {
			faultyNames := []string{}
			for _, f := range s.UnhealthyDevices() {
				faultyNames = append(faultyNames, fmt.Sprintf("%s/%s", s.Name, f.BSDName))
			}
			setUnhealthy(mqttClient, faultyNames)
		} else {
			setHealthy(mqttClient)
		}
		log.WithFields(log.Fields{
			"status": s.Status,
			"raid":   s.Name,
		}).Info("checked Raid and sent status to mqtt")
	}
	publish(mqttClient, "update_date", time.Now().Format(time.RFC3339))
}

func setHealthy(mqttClient MQTT.Client) {
	publish(mqttClient, "healthy", "true")
	publish(mqttClient, "faultydevices", "")
	publish(mqttClient, "error", "")
}

func setUnhealthy(mqttClient MQTT.Client, faultyNames []string) {
	publish(mqttClient, "healthy", "false")
	publish(mqttClient, "faultydevices", strings.Join(faultyNames, ","))
}

func setError(mqttClient MQTT.Client, err error) {
	publish(mqttClient, "healthy", "false")
	publish(mqttClient, "error", err.Error())
}

func DiskStatus() ([]RaidStatus, error) {
	output, err := Parser(Execute())

	var health []RaidStatus

	if err != nil {
		return nil, err
	}

	for _, r := range output.RaidSets {
		rHealth := RaidStatus{UUID: r.UUID, Name: r.Name, Status: r.Status}
		rHealth.Devices = append(rHealth.Devices, r.Members...)
		health = append(health, rHealth)
	}

	return health, nil
}

func Execute() io.ReadSeeker {
	cmd := exec.Command("diskutil", "appleRAID", "list", "-plist")
	var out bytes.Buffer
	cmd.Stdout = &out
	err := cmd.Run()

	if err != nil {
		panic(err)
	}

	outBytes, err := io.ReadAll(&out)
	if err != nil {
		panic(err)
	}
	r := bytes.NewReader(outBytes)

	if err != nil {
		log.Fatal(err)
	}
	return r
}

func Parser(input io.ReadSeeker) (DiskutilOutput, error) {
	var status DiskutilOutput
	if err := plist.NewDecoder(input).Decode(&status); err != nil {
		return DiskutilOutput{}, err
	}

	return status, nil
}

func publish(mqttClient MQTT.Client, topicPostfix string, payload interface{}) {
	topicPrefix := "home/storage/raidstatus"
	topic := fmt.Sprintf("%s/%s", topicPrefix, topicPostfix)
	token := mqttClient.Publish(topic, byte(0), true, payload)
	token.Wait()
}
