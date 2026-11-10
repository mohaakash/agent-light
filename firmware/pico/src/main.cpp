#include <Arduino.h>

#include "../../common/agentlight_serial.h"

// Raspberry Pi Pico using the Arduino-Pico core: GP6 red, GP7 yellow, GP8 green.
AgentLightSerial traffic_light(6, 7, 8);

void setup() { traffic_light.begin(); }

void loop() { traffic_light.poll(); }
