#pragma once

#include <Arduino.h>
#include <cstring>

class AgentLightSerial {
 public:
  AgentLightSerial(uint8_t red_pin, uint8_t yellow_pin, uint8_t green_pin)
      : red_pin_(red_pin), yellow_pin_(yellow_pin), green_pin_(green_pin) {}

  void begin() {
    pinMode(red_pin_, OUTPUT);
    pinMode(yellow_pin_, OUTPUT);
    pinMode(green_pin_, OUTPUT);
    allOff();
    Serial.begin(115200);
    last_valid_command_at_ = millis();
    Serial.println("AGENTLIGHT:SERIAL:1");
  }

  void poll() {
    while (Serial.available() > 0) {
      const char value = static_cast<char>(Serial.read());
      if (value == '\r') continue;
      if (value == '\n') {
        if (!discarding_line_ && line_length_ > 0) {
          line_buffer_[line_length_] = '\0';
          processLine(line_buffer_);
        }
        line_length_ = 0;
        discarding_line_ = false;
        continue;
      }
      if (discarding_line_) continue;
      if (line_length_ >= kMaxLineLength) {
        line_length_ = 0;
        discarding_line_ = true;
        continue;
      }
      line_buffer_[line_length_++] = value;
    }
    if (millis() - last_valid_command_at_ >= kCommunicationTimeoutMs) allOff();
  }

 private:
  static constexpr size_t kMaxLineLength = 48;
  static constexpr uint32_t kCommunicationTimeoutMs = 15000;

  uint8_t red_pin_;
  uint8_t yellow_pin_;
  uint8_t green_pin_;
  char line_buffer_[kMaxLineLength + 1]{};
  size_t line_length_ = 0;
  bool discarding_line_ = false;
  uint32_t last_valid_command_at_ = 0;

  void allOff() {
    digitalWrite(red_pin_, LOW);
    digitalWrite(yellow_pin_, LOW);
    digitalWrite(green_pin_, LOW);
  }

  void setStatus(const char* status) {
    allOff();
    if (std::strcmp(status, "WAITING") == 0 || std::strcmp(status, "ERROR") == 0) {
      digitalWrite(red_pin_, HIGH);
    } else if (std::strcmp(status, "WORKING") == 0) {
      digitalWrite(yellow_pin_, HIGH);
    } else if (std::strcmp(status, "COMPLETED") == 0) {
      digitalWrite(green_pin_, HIGH);
    }
  }

  bool isStatus(const char* status) {
    return std::strcmp(status, "WAITING") == 0 || std::strcmp(status, "WORKING") == 0 ||
        std::strcmp(status, "COMPLETED") == 0 || std::strcmp(status, "IDLE") == 0 ||
        std::strcmp(status, "ERROR") == 0;
  }

  void processLine(const char* line) {
    if (std::strcmp(line, "HELLO") == 0) {
      Serial.println("AGENTLIGHT:SERIAL:1");
    } else if (std::strcmp(line, "PING") == 0) {
      Serial.println("PONG");
    } else if (std::strncmp(line, "STATUS:", 7) == 0 && isStatus(line + 7)) {
      const char* status = line + 7;
      setStatus(status);
      Serial.print("ACK:STATUS:");
      Serial.println(status);
    } else {
      Serial.println("ERR:UNKNOWN_COMMAND");
      return;
    }
    last_valid_command_at_ = millis();
  }
};
