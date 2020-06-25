#include "crc.h"
#pragma opt_code_balanced
inline uint8_t crc8_update(uint8_t data, uint8_t crc) {
    crc ^= data;
    for (uint8_t i = 0; i < 8; i++)
        crc = (crc & 0x80) ? (crc << 1) ^ 0x07 : crc << 1;
    return crc;
}

uint8_t compute_crc(uint8_t *buff,uint8_t len) {
    uint8_t crc=0;
    while(len--) crc=crc8_update(*buff++,crc);
    return crc;
}