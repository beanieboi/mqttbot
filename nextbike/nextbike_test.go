package nextbike_test

import (
	"encoding/json"
	"log"
	"os"
	"testing"

	"github.com/beanieboi/mqttbot/nextbike"
)

func TestBikeFinder(t *testing.T) {
	content, err := os.ReadFile("nextbike.json")
	if err != nil {
		log.Fatal("Error when opening file: ", err)
	}

	var nbd nextbike.Data
	err = json.Unmarshal(content, &nbd)
	if err != nil {
		log.Fatal("Error during Unmarshal(): ", err)
	}

	if len(nbd.Countries) != 1 {
		t.Errorf("number of Countries) = %d; want 1", len(nbd.Countries))
	}

}
