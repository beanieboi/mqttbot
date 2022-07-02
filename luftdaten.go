package main

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
