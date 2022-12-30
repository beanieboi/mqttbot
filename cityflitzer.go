package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"

	log "github.com/sirupsen/logrus"
)

type CityflitzerVehicle struct {
	Distance float64 `json:"distance"`
}

func CityflitzerRunner() {
	maxDistance := 500.0
	ctx := context.Background()
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
		log.WithFields(log.Fields{
			"error": err,
		}).Error("Cityflitzer request failed")
		return
	}

	body, err := io.ReadAll(res.Body)
	if err != nil {
		log.WithFields(log.Fields{
			"error": err,
		}).Error("Cityflitzer request failed")
		return
	}

	err = res.Body.Close()
	if err != nil {
		log.WithFields(log.Fields{
			"error": err,
		}).Error("Cityflitzer request failed")
		return
	}

	var vehicles []CityflitzerVehicle

	err = json.Unmarshal(body, &vehicles)
	if err != nil {
		log.WithFields(log.Fields{
			"error": err,
		}).Error("Cityflitzer request failed")
		return
	}

	foundNearby := false
	for _, vehicle := range vehicles {
		if vehicle.Distance < maxDistance {
			foundNearby = true
			break
		}
	}

	token := mqttClient.Publish("mobility/cityflitzer/cityflitzer_nearby", byte(0), true, fmt.Sprintf("%t", foundNearby))
	token.Wait()
	token = mqttClient.Publish("mobility/cityflitzer/update_date", byte(0), true, time.Now().Format(time.RFC3339))
	token.Wait()

	log.WithFields(log.Fields{
		"foundNearby": foundNearby,
	}).Info("finished checking Cityflitzer and sent result to MQTT")
}
