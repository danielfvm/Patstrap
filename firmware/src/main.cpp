#include <ESP8266mDNS.h>        // Include the mDNS library
#include <ESP8266WiFi.h>

#define PIN_BATTERY_LEVEL A0
#define PIN_INTERNAL_LED 2 // indicates if connected with server (low active)
#define PIN_HAPTIC_LEFT  5 // D1
#define PIN_HAPTIC_RIGHT 4 // D2

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

void setup() {
  pinMode(PIN_INTERNAL_LED, OUTPUT);
  pinMode(PIN_HAPTIC_LEFT, OUTPUT);
  pinMode(PIN_HAPTIC_RIGHT, OUTPUT);
  pinMode(PIN_BATTERY_LEVEL, INPUT);

  digitalWrite(PIN_HAPTIC_LEFT, HAPTIC_OFF);
  digitalWrite(PIN_HAPTIC_RIGHT, HAPTIC_OFF);

  Serial.begin(9600);

  WiFi.mode(WIFI_STA);
  #if defined(WIFI_CREDS_SSID) && defined(WIFI_CREDS_PASSWD)
    WiFi.begin(WIFI_CREDS_SSID, WIFI_CREDS_PASSWD); //Connect to wifi
  #else
    #error "Missing -DWIFI_CREDS_SSID and -DWIFI_CREDS_PASSWD options in platformio.ini"
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
  MDNS.addService("http", "tcp", PORT);
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

  WiFiClient client = server.accept();
  
  if (client) {
    Serial.println("Client Connected");

    unsigned long previousMillis = millis();

    while (client.connected()) {
      unsigned long currentMillis = millis();

      // Process recv byte
      while (client.available() > 0) {
        char data = client.read();
        unsigned int haptic_right_level = (data & 0x0F) << 4;
        unsigned int haptic_left_level = data & 0xF0;

        // one channel goes from 0x00 to 0xF0, we add the missing 0x0F to be full range from 0x00 to 0xFF
        haptic_right_level |= haptic_right_level >> 4;
        haptic_left_level |= haptic_left_level >> 4;

        // Generates a pwm signal
        analogWrite(PIN_HAPTIC_LEFT, HAPTIC_OFF ? haptic_left_level : 0xFF - haptic_left_level);
        analogWrite(PIN_HAPTIC_RIGHT, HAPTIC_OFF ? haptic_right_level : 0xFF - haptic_right_level);
      }

      // Send keep alive packet with averaged battery value
      if (currentMillis - previousMillis >= 1000) {
        // send keep_alive package every second
        #if defined(USE_BATTERY)
        client.print((char)round(max(min(getBatteryLevel() * 100.0f, 100.0f), 0.0f)));
        #else
        client.print((char)255);
        #endif

        previousMillis = currentMillis;
        client.flush();
      }
    }

    client.stop();
    Serial.println("Client disconnected");    
  }
}
