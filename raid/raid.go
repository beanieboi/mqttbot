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
	UUID          string
	Name          string
	Status        string
	FaultyDevices []RaidMember
}

type RaidMember struct {
	UUID    string `plist:"AppleRAIDMemberUUID"`
	BSDName string `plist:"BSD Name"`
	Status  string `plist:"MemberStatus"`
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

type DiskutilOutput struct {
	RaidSets []RaidSet `plist:"AppleRAIDSets"`
}

func Runner(mqttClient MQTT.Client) {
	mqttPrefix := "home/storage/raidstatus/"
	raid, err := DiskStatus()

	if err != nil {
		setError(mqttClient, mqttPrefix, err)
	}

	for _, s := range raid {
		if s.Status != "Online" {
			faultyNames := []string{}
			for _, f := range s.FaultyDevices {
				faultyNames = append(faultyNames, f.BSDName)
			}
			setUnhealthy(mqttClient, mqttPrefix, faultyNames)
		} else {
			setHealthy(mqttClient, mqttPrefix)
		}
		log.WithFields(log.Fields{
			"status": s.Status,
			"raid":   s.Name,
		}).Info("checked Raid and sent status to mqtt")
	}

	token := mqttClient.Publish(fmt.Sprintf("%s/update_date", mqttPrefix), byte(0), true, time.Now().Format(time.RFC3339))
	token.Wait()
}

func setHealthy(mqttClient MQTT.Client, mqttPrefix string) {
	token := mqttClient.Publish(fmt.Sprintf("%s/healthy", mqttPrefix), byte(0), true, "true")
	token.Wait()
	token = mqttClient.Publish(fmt.Sprintf("%s/faultydevices", mqttPrefix), byte(0), true, "")
	token.Wait()
	token = mqttClient.Publish(fmt.Sprintf("%s/error", mqttPrefix), byte(0), true, "")
	token.Wait()
}

func setUnhealthy(mqttClient MQTT.Client, mqttPrefix string, faultyNames []string) {
	token := mqttClient.Publish(fmt.Sprintf("%s/healthy", mqttPrefix), byte(0), true, "false")
	token.Wait()
	token = mqttClient.Publish(fmt.Sprintf("%s/faultydevices", mqttPrefix), byte(0), true, strings.Join(faultyNames, ","))
	token.Wait()
}

func setError(mqttClient MQTT.Client, mqttPrefix string, err error) {
	token := mqttClient.Publish(fmt.Sprintf("%s/healthy", mqttPrefix), byte(0), true, "false")
	token.Wait()
	token = mqttClient.Publish(fmt.Sprintf("%s/error", mqttPrefix), byte(0), true, err.Error())
	token.Wait()
}

func DiskStatus() ([]RaidStatus, error) {
	output, err := Parser(Execute())

	var health []RaidStatus

	if err != nil {
		return nil, err
	}

	for _, r := range output.RaidSets {
		rHealth := RaidStatus{UUID: r.UUID, Name: r.Name, Status: r.Status}

		for _, m := range r.Members {
			if m.Status != "Online" {
				rHealth.FaultyDevices = append(rHealth.FaultyDevices, m)
			}
		}

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
