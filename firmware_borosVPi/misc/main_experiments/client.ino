#include<Wire.h>
//#include "inc/vpi_regs.h"

void help(){

    Serial.println("Command:");
    Serial.println("t - address");
}

void setup(){
    Serial.begin(9600);
    Serial.println("VPI Firmaware tester (h) for help");
    Serial.print("\n>");

    
    Wire.begin();
    //Wire.setClock(400000);

}

void loop() {
  uint8_t e;

    if(Serial.available()) {
        char c=Serial.read();
        Serial.print(c); Serial.println();
        switch(c) {
            case 't':
                Wire.beginTransmission(0x33);
                Wire.write(0);
                e=Wire.endTransmission();
                Serial.print("Result:"); Serial.println(e);
                break;
            case 'r':
                Serial.println("Reading...");
                e=Wire.requestFrom(0x33,1);
                Serial.print("Readed:"); Serial.println(e);
                while(Wire.available()) { Serial.print(" "); Serial.print(Wire.read(),HEX);}
                break;
                
            case 'h':
                help();
                break;
            default:
                Serial.println("Invalid command");
        }
        Serial.print("\n>");

    }

}
