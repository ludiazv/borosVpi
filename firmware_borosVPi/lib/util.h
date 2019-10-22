#ifndef UTIL_H
#define UTIL_H


#define set_bit(register_8, bit) (register_8 |= (1 << bit))
#define clear_bit(register_8, bit) (register_8 &= ~(1 << bit))
#define toggle_bit(register_8, bit) (register_8 ^= (1 << bit))
#define SET_BIT set_bit
#define CLEAR_BIT clear_bit



#endif /* UTIL_H */