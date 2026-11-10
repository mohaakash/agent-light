#include <Arduino.h>

#include "../../common/agentlight_serial.h"

// ESP32 DevKit V1: GPIO25 red, GPIO26 yellow, GPIO27 green.
AgentLightSerial traffic_light(25, 26, 27);

void setup() { traffic_light.begin(); }

void loop() { traffic_light.poll(); }
