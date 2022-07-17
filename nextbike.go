package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io/ioutil"
	"net/http"
	"time"

	"go.uber.org/zap"
)

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
	eCargoBikes := []string{"20094", "20096"}

	ctx := context.Background()
	client := &http.Client{
		Transport: http.DefaultTransport,
		Timeout:   time.Second * 5,
	}

	req, _ := http.NewRequestWithContext(ctx, "GET", "https://maps.nextbike.net/maps/nextbike-live.json?city=1&domains=le&list_cities=0&bikes=0", nil)

	res, err := client.Do(req)
	if err != nil {
		logger.Panic("error fetching JSON", zap.Error(err))
	}

	body, err := ioutil.ReadAll(res.Body)
	if err != nil {
		logger.Panic("error reading body", zap.Error(err))
	}

	err = res.Body.Close()
	if err != nil {
		logger.Panic("error closing body", zap.Error(err))
	}

	var nbd NextbikeData

	err = json.Unmarshal(body, &nbd)
	if err != nil {
		logger.Panic("error unmarshalling JSON", zap.Error(err))
	}

	found := false

	for _, c := range nbd.Countries {
		if c.Name == "Germany" {
			for _, city := range c.Cities {
				if city.Name == "Leipzig" {
					for _, place := range city.Places {
						if place.Number == 4103 {
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

	token := mqttClient.Publish("home/nextbike/e_cargo_available", byte(0), true, fmt.Sprintf("%t", found))
	token.Wait()
	token = mqttClient.Publish("home/nextbike/update_date", byte(0), true, time.Now().Format(time.RFC3339))
	token.Wait()

	logger.Info("finished checking Nextbike and sent result to MQTT", zap.Bool("found", found))
}
