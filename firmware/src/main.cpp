#include <ESP8266mDNS.h>        // Include the mDNS library
#include <ESP8266WiFi.h>

#define INTERNAL_LED 2 // indicates if connected with server (low active)
#define HAPTIC_LEFT 5  // D1
#define HAPTIC_RIGHT 4 // D2

#define SERVER_PORT 8888

static WiFiServer server(SERVER_PORT);

// number between 0(off) and 0xFF(full power)
static unsigned int haptic_left_level = 0;
static unsigned int haptic_right_level = 0;
static unsigned int pwm_number = 0;
static unsigned int keep_alive = 0;

void setup() {
  pinMode(INTERNAL_LED, OUTPUT);
  pinMode(HAPTIC_LEFT, OUTPUT);
  pinMode(HAPTIC_RIGHT, OUTPUT);

  digitalWrite(HAPTIC_LEFT, HIGH);
  digitalWrite(HAPTIC_RIGHT, HIGH);

  Serial.begin(9600);


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
    digitalWrite(INTERNAL_LED, LOW);
    Serial.print(".");
    delay(100);
    digitalWrite(INTERNAL_LED, HIGH);
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


  digitalWrite(HAPTIC_LEFT, LOW);
  digitalWrite(HAPTIC_RIGHT, LOW);
  delay(500);
  digitalWrite(HAPTIC_LEFT, HIGH);
  digitalWrite(HAPTIC_RIGHT, HIGH);
}

void loop() {
  MDNS.update();

  delay(500);
  digitalWrite(INTERNAL_LED, HIGH);
  delay(500);
  digitalWrite(INTERNAL_LED, LOW);

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
      digitalWrite(HAPTIC_LEFT, haptic_left_level > pwm_number);
      digitalWrite(HAPTIC_RIGHT, haptic_right_level > pwm_number);
      delayMicroseconds(1);

      if (++pwm_number >= 0xF) {
        pwm_number = 0;
      }

      // send keep_alive package every second
      if (++keep_alive >= 100000) {
        keep_alive = 0;
        client.print('k');
        client.flush();
      }
    }

    client.stop();
    Serial.println("Client disconnected");    
  }
}