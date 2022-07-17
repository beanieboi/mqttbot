package main

import MQTT "github.com/eclipse/paho.mqtt.golang"

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
