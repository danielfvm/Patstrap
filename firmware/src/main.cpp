
#include <ESP8266mDNS.h>
#include <ESP8266WiFi.h>
#include <cstdint>
#include "protocol.hpp"
#include <string>

#define PIN_BATTERY_LEVEL A0
#define PIN_INTERNAL_LED 2 // indicates if connected with server (low active)


#if defined(PORT)
static WiFiServer server(PORT);
#else
#error "Missing -DPORT option in platformio.ini"
#endif

#if defined(USE_PNP)
#define HAPTIC_ON LOW
#define HAPTIC_OFF HIGH
#else
#define HAPTIC_ON HIGH
#define HAPTIC_OFF LOW
#endif

#define ADCResolution 1023.0  // ESP8266 has 10bit ADC
#define ADCVoltageMax 1.0     // ESP8266 input is 1.0 V = 1023.0

// See https://github.com/SlimeVR/SlimeVR-Tracker-ESP/blob/main/src/batterymonitor.h for more information
#define BATTERY_SHIELD_R1 100.0
#define BATTERY_SHIELD_R2 220.0
#define BATTERY_SHIELD_RESISTANCE 180.0
#define ADCMultiplier (BATTERY_SHIELD_R1 + BATTERY_SHIELD_R2 + BATTERY_SHIELD_RESISTANCE) / BATTERY_SHIELD_R1

/*
 * Battery code taken from SlimeVR Github:
 *  https://github.com/SlimeVR/SlimeVR-Tracker-ESP/blob/main/src/batterymonitor.cpp
 */
float getBatteryLevel() {
  float voltage = ((float)analogRead(PIN_BATTERY_LEVEL)) * ADCVoltageMax / ADCResolution * ADCMultiplier;
  float level = 0.0f;

  // Estimate battery level, 3.2V is 0%, 4.17V is 100% (1.0)
  if (voltage > 3.975f)
      level = (voltage - 2.920f) * 0.8f;
  else if (voltage > 3.678f)
      level = (voltage - 3.300f) * 1.25f;
  else if (voltage > 3.489f)
      level = (voltage - 3.400f) * 1.7f;
  else if (voltage > 3.360f)
      level = (voltage - 3.300f) * 0.8f;
  else
      level = (voltage - 3.200f) * 0.3f;

  level = (level - 0.05f) / 0.95f;      // Cut off the last 5% (3.36V)
  level = max(min(level, 1.0f), 0.0f);  // Clamp between 0 and 1

  return level;
}

#if defined(HAPTIC_PINS)
static const std::vector<std::pair<char*, int>> GPIO_HAPTIC_MAPPING = HAPTIC_PINS;
#else
  #error "Missing -DHAPTIC_PINS option in platformio.ini"
#endif

// This here was made because regular analogWrite does not work well to adjust
// vibration strength. Instead we make a PWM signal where the High signal is
// long enough to get the motor starting to move (fixed ms length)
struct HapticPWM {
  long _duration, _interval;
  float _strength;
  int _pin;

  const long HIGH_TIME = 5; // Time in ms of high signal (this is fixed)
  const long OFF_TIME = 10; // Max time in ms of pause (OFF_TIME * strength)

  void init(int pin) {
    _pin = pin;
    _duration = 0;
    _interval = 0;
  }

  void set(long durtaion, float strength) {
    _duration = durtaion;
    _strength = 1.0 - strength;
    _interval = 0;
  }

  void update(unsigned long dt) {
    if (_duration > 0) {
      _duration -= dt;
      _interval += dt;
    }

    long period = HIGH_TIME + OFF_TIME * _strength;
    if (_interval > period)
      _interval = 0;

    bool on = _duration > 0 && _interval <= HIGH_TIME;

    digitalWrite(_pin, on ? HAPTIC_ON : HAPTIC_OFF);
  }
};

void setup() {
  pinMode(PIN_INTERNAL_LED, OUTPUT);
  pinMode(PIN_BATTERY_LEVEL, INPUT);

  for (auto& [_, pin] : GPIO_HAPTIC_MAPPING) {
    pinMode(pin, OUTPUT);
    digitalWrite(pin, HAPTIC_OFF);
  }

  Serial.begin(9600);

  // Connect to wifi
  WiFi.mode(WIFI_STA);
  #if defined(WIFI_CREDS_SSID) && defined(WIFI_CREDS_PASSWD)
    WiFi.begin(WIFI_CREDS_SSID, WIFI_CREDS_PASSWD);
  #else
    #error "Missing -DWIFI_CREDS_SSID and -DWIFI_CREDS_PASSWD options in platformio.ini"
  #endif
 
  // Wait for connection  
  Serial.println("Connecting to Wifi");
  while (WiFi.status() != WL_CONNECTED) {   
    delay(100);
    digitalWrite(PIN_INTERNAL_LED, HIGH);
    Serial.print(".");
    delay(100);
    digitalWrite(PIN_INTERNAL_LED, LOW);
  }

  // Start the mDNS responder for patstrap.local
  if (!MDNS.begin("patstrap")) {
    Serial.println("Error setting up MDNS responder!");
  }
  MDNS.addService("http", "tcp", PORT);
  Serial.println("mDNS responder started");

  // Play connection indication by vibrating all motors
  for (auto& [_, pin] : GPIO_HAPTIC_MAPPING)
    digitalWrite(pin, HAPTIC_ON);
  delay(500);
  for (auto& [_, pin] : GPIO_HAPTIC_MAPPING)
    digitalWrite(pin, HAPTIC_OFF);


  Serial.print("IP address: ");
  Serial.println(WiFi.localIP());  
  server.begin();
}

CommandServer getCommand(WiFiClient client)
{
  const char* data = client.peekBuffer();
  int datalen = client.peekAvailable();

  int end = -1;
  //int start = -1;
  for (int i = 0; i < datalen; i++) {
    /*if (data[i] == '\x0B') {
      start = i;
    }*/

    if (data[i] == '\x0A') {
      end = i;
      break;
    }
  }

  if (end == -1/* || start == -1*/)
    return CommandServer { .tag = Tag::Invalid };

  uint8_t channel = stoi(std::string(data, 2), nullptr, 16);
  uint8_t strength = stoi(std::string(data + 2, 2), nullptr, 16);
  uint16_t duration = stoi(std::string(data + 4, 4), nullptr, 16);

  client.peekConsume(end + 1);

  return CommandServer {
    .tag = Tag::Haptic,
    .haptic = {
      .channel = channel,
      .strength = strength,
      .duration = duration,
    }
  };
}

void loop() {
  MDNS.update();

  WiFiClient client = server.available();
  
  if (client) {
    Serial.println("Client Connected");
    digitalWrite(PIN_INTERNAL_LED, HIGH);

    unsigned long previousMillis = millis();
    unsigned long keepAliveTimer;

    HapticPWM pwm[GPIO_HAPTIC_MAPPING.size()];
    for (auto i = 0; i < GPIO_HAPTIC_MAPPING.size(); i++)
      pwm[i].init(GPIO_HAPTIC_MAPPING[i].second);

    auto reply = [&](CommandClient command) {
      uint8_t bytes[64];
      size_t len = command.to_bytes(bytes);
      client.write(bytes, len);
    };

    // Send haptic configuration to server
    for (int channel = 0; channel < GPIO_HAPTIC_MAPPING.size(); channel++) {
      CommandClient cmd = CommandClient {
        .tag = Tag::Info,
        .info = { .channel = (uint8_t)channel },
      };
      strcpy(cmd.info.name, GPIO_HAPTIC_MAPPING[channel].first);
      reply(cmd);
    }
    
    while (client.connected()) {
      unsigned long currentMillis = millis();
      unsigned long dt = currentMillis - previousMillis;
      previousMillis = currentMillis;

      for (int i = 0; i < GPIO_HAPTIC_MAPPING.size(); i++)
        pwm[i].update(dt);

      // Process recv bytes
      // uint8_t data[32];

      CommandServer cmd = getCommand(client);
      while (cmd.tag == Tag::Haptic) {
        if (cmd.haptic.channel < GPIO_HAPTIC_MAPPING.size()) {
          auto& [name, pin] = GPIO_HAPTIC_MAPPING[cmd.haptic.channel];
          uint8_t strength = cmd.haptic.strength;
          uint16_t duration = cmd.haptic.duration;

          pwm[cmd.haptic.channel].set(duration, (float)strength / 255.0);
        }

        cmd = getCommand(client);
      }


          //  if (command.haptic.channel < GPIO_HAPTIC_MAPPING.size()) {
     /* auto& [name, pin] = GPIO_HAPTIC_MAPPING[command.haptic.channel];
      uint8_t strength = command.haptic.strength;
      uint16_t duration = command.haptic.duration;

      pwm[command.haptic.channel].set(duration, (float)strength / 255.0);*/

      /*if (len > 0) {
        CommandServer command = CommandServer::from_bytes(data, len);
        switch (command.tag) {
          case Tag::Info: {
            for (int channel = 0; channel < GPIO_HAPTIC_MAPPING.size(); channel++) {
              CommandClient cmd = CommandClient {
                .tag = Tag::Info,
                .info = { .channel = (uint8_t)channel },
              };
              strcpy(cmd.info.name, GPIO_HAPTIC_MAPPING[channel].first);
              reply(cmd);
            }
          }
          case Tag::Battery: { 
            reply(CommandClient {
              .tag = Tag::Battery,
              .battery = { .level = (uint8_t)(getBatteryLevel() * 100) },
            });
          }
          case Tag::Haptic: {
            if (command.haptic.channel < GPIO_HAPTIC_MAPPING.size()) {
              auto& [name, pin] = GPIO_HAPTIC_MAPPING[command.haptic.channel];
              uint8_t strength = command.haptic.strength;
              uint16_t duration = command.haptic.duration;
  
              pwm[command.haptic.channel].set(duration, (float)strength / 255.0);

            }
          }
          default: {
          }
        }
      }*/
      client.flush();

      // Send keep alive packet with averaged battery value
      keepAliveTimer += dt;
      if (keepAliveTimer >= 3000) {
        reply(CommandClient {
          .tag = Tag::Battery,
          .battery = { .level = (uint8_t)(getBatteryLevel() * 100) },
        });

        keepAliveTimer = 0;
      }
    }

    for (auto i = 0; i < GPIO_HAPTIC_MAPPING.size(); i++)
      digitalWrite(GPIO_HAPTIC_MAPPING[i].second, HAPTIC_OFF);

    // Disconnection indication blink
    delay(500);
    digitalWrite(PIN_INTERNAL_LED, HIGH);
    delay(500);
    digitalWrite(PIN_INTERNAL_LED, LOW);
    delay(500);
    digitalWrite(PIN_INTERNAL_LED, HIGH);
    delay(500);
    digitalWrite(PIN_INTERNAL_LED, LOW);

    // close the connection:
    client.stop();
    Serial.println("Client disconnected");
  }
}
