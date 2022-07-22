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
	myStation := 4103
	myCountry := "Germany"
	myCity := "Leipzig"

	eCargoBikes := []string{"20093", "20094", "20095", "20096"}

	ctx := context.Background()
	client := &http.Client{
		Transport: http.DefaultTransport,
		Timeout:   time.Second * 5,
	}

	req, _ := http.NewRequestWithContext(ctx, "GET", "https://maps.nextbike.net/maps/nextbike-live.json?city=1&domains=le&list_cities=0&bikes=0", nil)

	res, err := client.Do(req)
	if err != nil {
		logger.Error("error fetching JSON", zap.Error(err))
		return
	}

	body, err := ioutil.ReadAll(res.Body)
	if err != nil {
		logger.Error("error reading body", zap.Error(err))
		return
	}

	err = res.Body.Close()
	if err != nil {
		logger.Error("error closing body", zap.Error(err))
		return
	}

	var nbd NextbikeData

	err = json.Unmarshal(body, &nbd)
	if err != nil {
		logger.Error("error unmarshalling JSON", zap.Error(err))
		return
	}

	found := false

	for _, c := range nbd.Countries {
		if c.Name == myCountry {
			for _, city := range c.Cities {
				if city.Name == myCity {
					for _, place := range city.Places {
						if place.Number == myStation {
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
