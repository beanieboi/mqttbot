package main

import (
	"os"
	"time"

	MQTT "github.com/eclipse/paho.mqtt.golang"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

var mqttClient MQTT.Client
var logger *zap.Logger

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

	atom := zap.NewAtomicLevel()

	encoderCfg := zap.NewProductionEncoderConfig()
	encoderCfg.TimeKey = "timestamp"
	encoderCfg.EncodeTime = zapcore.ISO8601TimeEncoder

	logger = zap.New(zapcore.NewCore(
		zapcore.NewConsoleEncoder(encoderCfg),
		zapcore.Lock(os.Stdout),
		atom,
	))
	defer logger.Sync() //nolint:errcheck

	mqttClient = NewMQTTClient(host, username, password)

	go func() {
		for range time.Tick(2 * time.Minute) {
			go NextbikeRunner()
			go CityflitzerRunner()
		}
	}()
}
