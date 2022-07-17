package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"time"
)

type LuftdatenData struct {
	EspId           string                 `json:"esp8266id"`
	SoftwareVersion string                 `json:"software_version"`
	SensorData      []LuftdatenMeasurement `json:"sensordatavalues"`
}

type LuftdatenMeasurement struct {
	ValueType string  `json:"value_type"`
	Value     float64 `json:"value,string"`
}

func luftdatenHandler(w http.ResponseWriter, r *http.Request) {
	var d LuftdatenData
	err := json.NewDecoder(r.Body).Decode(&d)
	if err != nil {
		fmt.Println(err)
		w.WriteHeader(http.StatusInternalServerError)
	}
	for _, measurement := range d.SensorData {
		topic := fmt.Sprintf("home/luftdaten/%s", measurement.ValueType)
		token := mqttClient.Publish(topic, byte(0), true, fmt.Sprintf("%.2f", measurement.Value))
		token.Wait()
	}
	pm25 := findPM25(d.SensorData)
	token := mqttClient.Publish("home/luftdaten/aqi", byte(0), true, fmt.Sprintf("%.2f", calcAQI(pm25)))
	token.Wait()
	token = mqttClient.Publish("home/luftdaten/aqi_human", byte(0), true, humanaqi(calcAQI(pm25)))
	token.Wait()
	token = mqttClient.Publish("home/luftdaten/update_date", byte(0), true, time.Now().Format(time.RFC3339))
	token.Wait()

	w.WriteHeader(http.StatusOK)
}

func findPM25(sensordata []LuftdatenMeasurement) float64 {
	var pm25 float64
	for _, measurement := range sensordata {
		if measurement.ValueType == "SDS_P2" {
			pm25 = measurement.Value
		}
	}
	return pm25
}

// {
// 	"esp8266id": "16659171",
// 	"software_version": "NRZ-2020-133",
// 	"sensordatavalues":[
// 		{"value_type":"SDS_P1","value":"1.10"},
// 		{"value_type":"SDS_P2","value":"1.00"},
// 		{"value_type":"samples","value":"5032003"},
// 		{"value_type":"min_micro","value":"28"},
// 		{"value_type":"max_micro","value":"20253"},
// 		{"value_type":"interval","value":"145000"},
// 		{"value_type":"signal","value":"-79"}
// 	]
// }

func humanaqi(aqi float64) string {
	if aqi >= 401.0 {
		return "Hazardous"
	} else if aqi >= 301.0 {
		return "Hazardous"
	} else if aqi >= 201.0 {
		return "Very Unhealthy"
	} else if aqi >= 151.0 {
		return "Unhealthy"
	} else if aqi >= 101.0 {
		return "Unhealthy for Sensitive Groups"
	} else if aqi >= 51.0 {
		return "Moderate"
	} else if aqi >= 0.0 {
		return "Good"
	} else {
		return "undefined"
	}
}

func aqi(Cp float64, Ih float64, Il float64, BPh float64, BPl float64) float64 {
	return (((Ih-Il)/(BPh-BPl))*(Cp-BPl) + Il)
}

func calcAQI(pm25 float64) float64 {
	if pm25 > 1000 {
		return -1
	} else if pm25 > 350.5 {
		return aqi(pm25, 500.0, 401.0, 500.0, 350.5)
	} else if pm25 > 250.5 {
		return aqi(pm25, 400.0, 301.0, 350.4, 250.5)
	} else if pm25 > 150.5 {
		return aqi(pm25, 300.0, 201.0, 250.4, 150.5)
	} else if pm25 > 55.5 {
		return aqi(pm25, 200.0, 151.0, 150.4, 55.5)
	} else if pm25 > 35.5 {
		return aqi(pm25, 150.0, 101.0, 55.4, 35.5)
	} else if pm25 > 12.1 {
		return aqi(pm25, 100.0, 51.0, 35.4, 12.1)
	} else if pm25 >= 0.0 {
		return aqi(pm25, 50.0, 0.0, 12.0, 0.0)
	} else {
		return -1
	}
}
