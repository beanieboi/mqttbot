package nextbike

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"time"

	MQTT "github.com/eclipse/paho.mqtt.golang"
	log "github.com/sirupsen/logrus"
)

type HomeStation struct {
	ID      int
	Country string
	City    string
}

type Place struct {
	Number       int
	BikesNumbers []string `json:"bike_numbers"`
}

type City struct {
	Name   string  `json:"name"`
	Places []Place `json:"places"`
}

type Country struct {
	Name   string `json:"country_name"`
	Cities []City `json:"cities"`
}

type Data struct {
	Countries []Country `json:"countries"`
}

func Runner(mqttClient MQTT.Client) {
	ctx := context.Background()
	nbd, err := GetNextbikeData(ctx)
	log.WithFields(log.Fields{
		"error": err,
	}).Error("Nextbike request failed")

	found := BikeFinder(nbd)

	publish(mqttClient, "e_cargo_available", fmt.Sprintf("%t", found))
	publish(mqttClient, "update_date", time.Now().Format(time.RFC3339))

	log.WithFields(log.Fields{
		"foundNearby": found,
	}).Info("finished checking Nextbike and sent result to MQTT")
}

func BikeFinder(nbd Data) bool {
	home := HomeStation{
		ID:      4103,
		Country: "Germany",
		City:    "Leipzig",
	}

	eCargoBikes := []string{"20091", "20095", "20096", "20111", "20118", "20119"}

	found := false

	for _, c := range nbd.Countries {
		if c.Name == home.Country {
			for _, city := range c.Cities {
				if city.Name == home.City {
					for _, place := range city.Places {
						if place.Number == home.ID {
							for _, bn := range place.BikesNumbers {
								for _, e := range eCargoBikes {
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
	return found
}

func GetNextbikeData(ctx context.Context) (Data, error) {
	client := &http.Client{
		Transport: http.DefaultTransport,
		Timeout:   time.Second * 5,
	}

	uri := "https://maps.nextbike.net/maps/nextbike-live.json?city=1&domains=le&list_cities=0&bikes=0"
	req, _ := http.NewRequestWithContext(ctx, "GET", uri, nil)

	res, err := client.Do(req)
	if err != nil {
		return Data{}, err
	}
	body, err := io.ReadAll(res.Body)
	if err != nil {
		return Data{}, errors.New("nextbike request failed")
	}
	err = res.Body.Close()
	if err != nil {
		return Data{}, errors.New("nextbike request failed")
	}

	var nbd Data
	err = json.Unmarshal(body, &nbd)
	if err != nil {
		return Data{}, errors.New("nextbike request failed")
	}
	return nbd, nil
}

func publish(mqttClient MQTT.Client, topicPostfix string, payload interface{}) {
	topicPrefix := "mobility/nextbike"
	topic := fmt.Sprintf("%s/%s", topicPrefix, topicPostfix)
	token := mqttClient.Publish(topic, byte(0), true, payload)
	token.Wait()
}
