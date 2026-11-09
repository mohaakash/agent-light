#include <Arduino.h>
#include <cstring>

namespace {
constexpr uint8_t RED_PIN = 5;     // D1 / GPIO5
constexpr uint8_t YELLOW_PIN = 4;  // D2 / GPIO4
constexpr uint8_t GREEN_PIN = 14;  // D5 / GPIO14
constexpr size_t MAX_LINE_LENGTH = 48;
constexpr uint32_t COMMUNICATION_TIMEOUT_MS = 15000;

char lineBuffer[MAX_LINE_LENGTH + 1];
size_t lineLength = 0;
bool discardingLine = false;
uint32_t lastValidCommandAt = 0;

void allOff() {
  digitalWrite(RED_PIN, LOW);
  digitalWrite(YELLOW_PIN, LOW);
  digitalWrite(GREEN_PIN, LOW);
}

void setStatus(const char* status) {
  allOff();
  if (std::strcmp(status, "WAITING") == 0 || std::strcmp(status, "ERROR") == 0) {
    digitalWrite(RED_PIN, HIGH);
  } else if (std::strcmp(status, "WORKING") == 0) {
    digitalWrite(YELLOW_PIN, HIGH);
  } else if (std::strcmp(status, "COMPLETED") == 0) {
    digitalWrite(GREEN_PIN, HIGH);
  }
}

void processLine(const char* line) {
  if (std::strcmp(line, "HELLO") == 0) {
    Serial.println("AGENTLIGHT:ESP8266:1");
    lastValidCommandAt = millis();
    return;
  }
  if (std::strcmp(line, "PING") == 0) {
    Serial.println("PONG");
    lastValidCommandAt = millis();
    return;
  }
  if (std::strncmp(line, "STATUS:", 7) == 0) {
    const char* status = line + 7;
    if (std::strcmp(status, "WAITING") == 0 || std::strcmp(status, "WORKING") == 0 ||
        std::strcmp(status, "COMPLETED") == 0 || std::strcmp(status, "IDLE") == 0 ||
        std::strcmp(status, "ERROR") == 0) {
      setStatus(status);
      Serial.print("ACK:STATUS:");
      Serial.println(status);
      lastValidCommandAt = millis();
      return;
    }
  }

  Serial.println("ERR:UNKNOWN_COMMAND");
}

void readSerialCommands() {
  while (Serial.available() > 0) {
    const char value = static_cast<char>(Serial.read());
    if (value == '\r') continue;
    if (value == '\n') {
      if (!discardingLine && lineLength > 0) {
        lineBuffer[lineLength] = '\0';
        processLine(lineBuffer);
      }
      lineLength = 0;
      discardingLine = false;
      continue;
    }

    if (discardingLine) continue;
    if (lineLength >= MAX_LINE_LENGTH) {
      lineLength = 0;
      discardingLine = true;
      continue;
    }
    lineBuffer[lineLength++] = value;
  }
}
}  // namespace

void setup() {
  pinMode(RED_PIN, OUTPUT);
  pinMode(YELLOW_PIN, OUTPUT);
  pinMode(GREEN_PIN, OUTPUT);
  allOff();
  Serial.begin(115200);
  lastValidCommandAt = millis();
  Serial.println("AGENTLIGHT:ESP8266:1");
}

void loop() {
  readSerialCommands();
  if (millis() - lastValidCommandAt >= COMMUNICATION_TIMEOUT_MS) {
    allOff();
  }
  yield();
}
