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

type NextbikeHomeStation struct {
	ID      int
	Country string
	City    string
}

type NextbikePlace struct {
	Number       int
	BikesNumbers []string `json:"bike_numbers"`
}

type NextbikeCity struct {
	Name   string          `json:"name"`
	Places []NextbikePlace `json:"places"`
}

type NextbikeCountry struct {
	Name   string         `json:"country_name"`
	Cities []NextbikeCity `json:"cities"`
}

type NextbikeData struct {
	Countries []NextbikeCountry `json:"countries"`
}

func NextbikeRunner() {
	home := NextbikeHomeStation{
		ID:      4103,
		Country: "Germany",
		City:    "Leipzig",
	}

	eCargoBikes := []string{"20091", "20095", "20096", "20111", "20118", "20119"}

	ctx := context.Background()
	client := &http.Client{
		Transport: http.DefaultTransport,
		Timeout:   time.Second * 5,
	}

	req, _ := http.NewRequestWithContext(ctx, "GET", "https://maps.nextbike.net/maps/nextbike-live.json?city=1&domains=le&list_cities=0&bikes=0", nil)

	res, err := client.Do(req)
	if err != nil {
		log.WithFields(log.Fields{
			"error": err,
		}).Error("Nextbike request failed")
		return
	}

	body, err := io.ReadAll(res.Body)
	if err != nil {
		log.WithFields(log.Fields{
			"error": err,
		}).Error("Nextbike request failed")
		return
	}

	err = res.Body.Close()
	if err != nil {
		log.WithFields(log.Fields{
			"error": err,
		}).Error("Nextbike request failed")
		return
	}

	var nbd NextbikeData

	err = json.Unmarshal(body, &nbd)
	if err != nil {
		log.WithFields(log.Fields{
			"error": err,
		}).Error("Nextbike request failed")
		return
	}

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

	token := mqttClient.Publish("mobility/nextbike/e_cargo_available", byte(0), true, fmt.Sprintf("%t", found))
	token.Wait()
	token = mqttClient.Publish("mobility/nextbike/update_date", byte(0), true, time.Now().Format(time.RFC3339))
	token.Wait()

	log.WithFields(log.Fields{
		"foundNearby": found,
	}).Info("finished checking Nextbike and sent result to MQTT")
}
