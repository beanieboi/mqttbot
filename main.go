package main

import (
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/beanieboi/mqttbot/middleware"
	MQTT "github.com/eclipse/paho.mqtt.golang"
	"github.com/gorilla/handlers"
	"github.com/gorilla/mux"
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
		zapcore.NewJSONEncoder(encoderCfg),
		zapcore.Lock(os.Stdout),
		atom,
	))
	defer logger.Sync()

	mqttClient = NewMQTTClient(host, username, password)

	for range time.Tick(5 * time.Minute) {
		NextbikeRunner()
	}

	r := mux.NewRouter()
	r.Use(middleware.RequestID)
	r.Use(middleware.Logger(logger))

	r.Use(handlers.RecoveryHandler())
	r.HandleFunc("/", luftdatenHandler).Methods("POST")

	address := fmt.Sprintf(":%s", port)
	logger.Info("ready to receive requests", zap.String("address", address))
	err := http.ListenAndServe(address, handlers.LoggingHandler(os.Stdout, r))

	if err != nil {
		log.Panic("Unable to start server ", err)
	}
}
