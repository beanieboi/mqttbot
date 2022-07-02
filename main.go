package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/beanieboi/luftdaten2mqtt/middleware"
	MQTT "github.com/eclipse/paho.mqtt.golang"
	"github.com/gorilla/handlers"
	"github.com/gorilla/mux"
	"go.uber.org/zap"
)

type Measurement struct {
	ValueType string  `json:"value_type"`
	Value     float64 `json:"value,string"`
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

var mqttClient MQTT.Client

type Data struct {
	EspId           string        `json:"esp8266id"`
	SoftwareVersion string        `json:"software_version"`
	SensorData      []Measurement `json:"sensordatavalues"`
}

func main() {
	port := os.Getenv("PORT")
	if len(port) == 0 {
		port = "8080"
	}

	host := os.Getenv("MQTT_HOST")
	if len(host) == 0 {
		host = "tcp://192.168.1.5:1883"
	}

	username := os.Getenv("MQTT_USERNAME")
	if len(username) == 0 {
		panic("username needed")
	}

	password := os.Getenv("MQTT_PASSWORD")
	if len(password) == 0 {
		panic("password needed")
	}

	mqttClient = NewMQTTClient(host, username, password)

	logger, _ := zap.NewProduction()
	defer logger.Sync()

	r := mux.NewRouter()
	r.Use(middleware.RequestID)
	r.Use(middleware.Logger(logger))

	r.Use(handlers.RecoveryHandler())
	r.HandleFunc("/", mqttHandler).Methods("POST")

	address := fmt.Sprintf(":%s", port)
	err := http.ListenAndServe(address, handlers.LoggingHandler(os.Stdout, r))
	if err != nil {
		log.Panic("Unable to start server ", err)
	}
}

func mqttHandler(w http.ResponseWriter, r *http.Request) {
	var d Data
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

func findPM25(sensordata []Measurement) float64 {
	var pm25 float64
	for _, measurement := range sensordata {
		if measurement.ValueType == "SDS_P2" {
			pm25 = measurement.Value
		}
	}
	return pm25
}

func NewMQTTClient(host string, username string, password string) MQTT.Client {
	opts := MQTT.NewClientOptions()
	opts.AddBroker(host)
	opts.SetClientID("luftdaten2mqtt")
	opts.SetUsername(username)
	opts.SetPassword(password)

	client := MQTT.NewClient(opts)
	if token := client.Connect(); token.Wait() && token.Error() != nil {
		panic(token.Error())
	}
	return client
}
