package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io/ioutil"
	"net/http"
	"net/url"
	"strings"
	"time"

	geo "github.com/kellydunn/golang-geo"
	"go.uber.org/zap"
)

type CityflitzerGeoPosition struct {
	Longitude float64 `json:"lon,string"`
	Latitude  float64 `json:"lat,string"`
	Show      bool    `json:"showMap"`
}

type CityflitzerVehicle struct {
	Name         string                 `json:"name"`
	LicensePlate string                 `json:"licensePlate"`
	Available    bool                   `json:"available"`
	DriveMode    string                 `json:"driveMode"`
	Position     CityflitzerGeoPosition `json:"geoPos,omitempty"`
}

type CityflitzerVehicles struct {
	Vehicles []CityflitzerVehicle `json:"data"`
}

type CityflitzerData struct {
	Container CityflitzerVehicles `json:"getVehicleCacheByGeoBounds"`
}

func CityflitzerRunner() {
	home := geo.NewPoint(51.3201768, 12.3660048)
	radius := 0.5

	ctx := context.Background()
	client := &http.Client{
		Transport: http.DefaultTransport,
		Timeout:   time.Second * 5,
	}

	form := url.Values{}
	form.Add("lat1", "49.0305875")
	form.Add("lat2", "53.9140125")
	form.Add("lon1", "0.76371300")
	form.Add("lon2", "21.527873")
	form.Add("requestTimestamp", "1658127848585")
	form.Add("platform", "tawebsite")
	form.Add("version", "10000000")
	form.Add("tracking", "off")

	req, _ := http.NewRequestWithContext(ctx, "POST", "https://sal2.teilauto.net/api/getVehicleCacheByGeoBounds", strings.NewReader(form.Encode()))
	req.Header.Add("Content-Type", "application/x-www-form-urlencoded")

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

	var cfd CityflitzerData

	err = json.Unmarshal(body, &cfd)
	if err != nil {
		logger.Panic("error unmarshalling JSON", zap.Error(err))
	}

	foundNearby := false

	// filter cityflitzer
	for _, car := range cfd.Container.Vehicles {
		if car.DriveMode == "cF" && car.Available {
			carPosition := geo.NewPoint(car.Position.Latitude, car.Position.Longitude)
			dist := home.GreatCircleDistance(carPosition)
			if dist < radius {
				foundNearby = true
			}
		}
	}

	token := mqttClient.Publish("home/cityflitzer/cityflitzer_nearby", byte(0), true, fmt.Sprintf("%t", foundNearby))
	token.Wait()
	token = mqttClient.Publish("home/cityflitzer/update_date", byte(0), true, time.Now().Format(time.RFC3339))
	token.Wait()

	logger.Info("finished checking Cityflitzer and sent result to MQTT", zap.Bool("foundNearby", foundNearby))
}
