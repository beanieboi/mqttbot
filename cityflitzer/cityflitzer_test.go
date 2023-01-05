package cityflitzer_test

import (
	"encoding/json"
	"log"
	"os"
	"testing"

	"github.com/beanieboi/mqttbot/cityflitzer"
)

func TestBikeFinder(t *testing.T) {
	content, err := os.ReadFile("cityflitzer.json")
	if err != nil {
		log.Fatal("Error when opening file: ", err)
	}

	var vehicles []cityflitzer.Vehicle
	err = json.Unmarshal(content, &vehicles)
	if err != nil {
		log.Fatal("Error during Unmarshal(): ", err)
	}

	if len(vehicles) != 177 {
		t.Errorf("number of Vehicles) = %d; want 177", len(vehicles))
	}

}
