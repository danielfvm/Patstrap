#include <ESP8266mDNS.h>        // Include the mDNS library
#include <ESP8266WiFi.h>

#define PIN_BATTERY_LEVEL 0
#define PIN_INTERNAL_LED 2 // indicates if connected with server (low active)
#define PIN_HAPTIC_LEFT  5 // D1
#define PIN_HAPTIC_RIGHT 4 // D2
#define SERVER_PORT 8888

static WiFiServer server(SERVER_PORT);

// number between 0x00(off) and 0xFF(full power)
static unsigned int haptic_left_level = 0;
static unsigned int haptic_right_level = 0;
static unsigned int pwm_number = 0;
static unsigned int keep_alive = 0;

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
byte getBatteryLevel() {
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

  return level * 100;
}

void setup() {
  pinMode(PIN_INTERNAL_LED, OUTPUT);
  pinMode(PIN_HAPTIC_LEFT, OUTPUT);
  pinMode(PIN_HAPTIC_RIGHT, OUTPUT);
  pinMode(PIN_BATTERY_LEVEL, INPUT);

  digitalWrite(PIN_HAPTIC_LEFT, HAPTIC_OFF);
  digitalWrite(PIN_HAPTIC_RIGHT, HAPTIC_OFF);

  Serial.begin(115200);


  WiFi.mode(WIFI_STA);
  #if defined(WIFI_CREDS_SSID) && defined(WIFI_CREDS_PASSWD)
    WiFi.begin(WIFI_CREDS_SSID, WIFI_CREDS_PASSWD); //Connect to wifi
  #else
    #error "Missing defines WIFI_CREDS_SSID and WIFI_CREDS_PASSWD"
  #endif
 
  // Wait for connection  
  Serial.println("Connecting to Wifi");
  while (WiFi.status() != WL_CONNECTED) {   
    delay(100);
    digitalWrite(PIN_INTERNAL_LED, LOW);
    Serial.print(".");
    delay(100);
    digitalWrite(PIN_INTERNAL_LED, HIGH);
  }

  Serial.print("IP address: ");
  Serial.println(WiFi.localIP());  
  server.begin();

  // Start the mDNS responder for patstrap.local
  if (!MDNS.begin("patstrap")) {
    Serial.println("Error setting up MDNS responder!");
  }
  MDNS.addService("http", "tcp", 80);
  Serial.println("mDNS responder started");


  digitalWrite(PIN_HAPTIC_LEFT, HAPTIC_ON);
  digitalWrite(PIN_HAPTIC_RIGHT, HAPTIC_ON);
  delay(500);
  digitalWrite(PIN_HAPTIC_LEFT, HAPTIC_OFF);
  digitalWrite(PIN_HAPTIC_RIGHT, HAPTIC_OFF);
}

void loop() {
  MDNS.update();

  delay(500);
  digitalWrite(PIN_INTERNAL_LED, HIGH);
  delay(500);
  digitalWrite(PIN_INTERNAL_LED, LOW);

  WiFiClient client = server.available();
  
  if (client) {
    Serial.println("Client Connected");
    
    while (client.connected()) {

      // Process recv byte
      while (client.available() > 0) {
        char data = client.read();
        haptic_right_level = data & 0x0F;
        haptic_left_level = data >> 4;
      }

      // create PWM signal for both haptic sensors 
      digitalWrite(PIN_HAPTIC_LEFT, haptic_left_level > pwm_number ? HAPTIC_OFF : HAPTIC_ON);
      digitalWrite(PIN_HAPTIC_RIGHT, haptic_right_level > pwm_number ? HAPTIC_OFF : HAPTIC_ON);
      delayMicroseconds(1);

      if (++pwm_number >= 0xF) {
        pwm_number = 0;
      }

      // send keep_alive package every second
      if (++keep_alive >= 100000) {
        keep_alive = 0;
        #if defined(USE_BATTERY)
        client.print((char)getBatteryLevel());
        #else
        client.print((char)255);
        #endif
        client.flush();
      }
    }

    client.stop();
    Serial.println("Client disconnected");    
  }
}
