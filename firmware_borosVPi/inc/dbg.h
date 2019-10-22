#ifndef _H_VPI_DBG_H_
#define _H_VPI_DBG_H_

#if defined(VPI_DEBUG)

    #include<uart.h>
    #include<stdio.h>

    #define DBG_INIT()          uart_init() 
    #define DBG(...)            printf(__VA_ARGS__)
    #define DBG_EXP(X)          (X) 


#else
    #define DBG_INIT()      ( (void)0 )
    #define DBG(...)        ( (void)0 )
    #define DBG_EXP(X)      ( (void)0 )

#endif


#endif