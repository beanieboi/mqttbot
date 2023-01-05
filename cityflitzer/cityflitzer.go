package cityflitzer

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"

	MQTT "github.com/eclipse/paho.mqtt.golang"
	log "github.com/sirupsen/logrus"
)

type Vehicle struct {
	Distance float64 `json:"distance"`
}

func Runner(mqttClient MQTT.Client) {
	ctx := context.Background()
	vehicles, err := getData(ctx)
	if err != nil {
		log.WithFields(log.Fields{
			"error": err,
		}).Error("Cityflitzer request failed")
	}

	foundNearby := Finder(vehicles)
	publish(mqttClient, "cityflitzer_nearby", fmt.Sprintf("%t", foundNearby))
	publish(mqttClient, "update_date", time.Now().Format(time.RFC3339))

	log.WithFields(log.Fields{
		"foundNearby": foundNearby,
	}).Info("finished checking Cityflitzer and sent result to MQTT")
}

func publish(mqttClient MQTT.Client, topicPostfix string, payload interface{}) {
	topicPrefix := "mobility/cityflitzer"
	topic := fmt.Sprintf("%s/%s", topicPrefix, topicPostfix)
	token := mqttClient.Publish(topic, byte(0), true, payload)
	token.Wait()
}

func getData(ctx context.Context) ([]Vehicle, error) {
	client := &http.Client{
		Transport: http.DefaultTransport,
		Timeout:   time.Second * 5,
	}

	lat := 51.32032033409821
	long := 12.36535400104385
	start := time.Now().Truncate(time.Hour).UTC()
	end := start.Add(time.Hour).UTC()

	uri := fmt.Sprintf("https://de1.cantamen.de/casirest/v3/pointsofinterest?&placeIsFixed=false&lat=%f&lng=%f&range=30000&start=%s&end=%s&sort=distance", lat, long, start.Format(time.RFC3339), end.Format(time.RFC3339))
	req, _ := http.NewRequestWithContext(ctx, "GET", uri, nil)
	req.Header.Add("X-API-KEY", "45d38969-0086-978d-dc06-7959b0d2fe79")

	res, err := client.Do(req)
	if err != nil {
		return []Vehicle{}, err
	}

	body, err := io.ReadAll(res.Body)
	if err != nil {
		return []Vehicle{}, err
	}

	err = res.Body.Close()
	if err != nil {
		return []Vehicle{}, err
	}

	var vehicles []Vehicle

	err = json.Unmarshal(body, &vehicles)
	if err != nil {
		return []Vehicle{}, err
	}

	return vehicles, nil
}

func Finder(vehicles []Vehicle) bool {
	maxDistance := 500.0
	foundNearby := false
	for _, vehicle := range vehicles {
		if vehicle.Distance < maxDistance {
			foundNearby = true
			break
		}
	}
	return foundNearby
}
