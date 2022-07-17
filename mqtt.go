package main

import (
	MQTT "github.com/eclipse/paho.mqtt.golang"
	"go.uber.org/zap"
)

func NewMQTTClient(host string, username string, password string) MQTT.Client {
	opts := MQTT.NewClientOptions()
	opts.AddBroker(host)
	opts.SetClientID("mqttbot")
	opts.SetUsername(username)
	opts.SetPassword(password)

	client := MQTT.NewClient(opts)
	if token := client.Connect(); token.Wait() && token.Error() != nil {
		logger.Error("error connecting to MQTT", zap.Error(token.Error()))
	}
	return client
}
