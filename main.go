package main

import (
	"fmt"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/beanieboi/mqttbot/cityflitzer"
	"github.com/beanieboi/mqttbot/nextbike"
	"github.com/beanieboi/mqttbot/raid"
	MQTT "github.com/eclipse/paho.mqtt.golang"
	log "github.com/sirupsen/logrus"
)

var mqttClient MQTT.Client

func main() {
	log.SetFormatter(&log.JSONFormatter{})

	sigs := make(chan os.Signal, 1)
	signal.Notify(sigs, syscall.SIGINT, syscall.SIGTERM)
	done := make(chan bool, 1)

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

	go func() {
		<-sigs
		done <- true
	}()

	go func() {
		for range time.Tick(2 * time.Minute) {
			go nextbike.Runner(mqttClient)
			go cityflitzer.Runner(mqttClient)
			go raid.Runner(mqttClient)
		}
	}()

	fmt.Println("MQTT bot running...")
	<-done
	fmt.Println("MQTT bot shutting down...")
}
