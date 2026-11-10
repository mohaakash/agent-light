#include "../common/agentlight_serial.h"

// Arduino Uno/Nano: D5 red, D6 yellow, D7 green.
AgentLightSerial traffic_light(5, 6, 7);

void setup() { traffic_light.begin(); }

void loop() { traffic_light.poll(); }
