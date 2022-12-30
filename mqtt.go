package main

import (
	MQTT "github.com/eclipse/paho.mqtt.golang"
	log "github.com/sirupsen/logrus"
)

func NewMQTTClient(host string, username string, password string) MQTT.Client {
	opts := MQTT.NewClientOptions()
	opts.AddBroker(host)
	opts.SetClientID("mqttbot")
	opts.SetUsername(username)
	opts.SetPassword(password)

	client := MQTT.NewClient(opts)
	if token := client.Connect(); token.Wait() && token.Error() != nil {
		log.WithFields(log.Fields{
			"error": token.Error(),
		}).Error("MQTT request failed")
	}
	return client
}
