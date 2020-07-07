
# VPi firmware make file

include makefile.config

# Directories
BUILDDIR  = build
LIBDIR    = lib
SRCDIR    = src
INCDIR    = inc

# arch & CPU parameters
ARCH   = stm8
MCU   ?= stm8s003f3
F_CPU ?= 2000000
SERIAL_BAUD ?= 115200


# Tools
CC 		?= sdcc
LD 		?= sdld
AS 		?= sdasstm8 
OBJCOPY ?= stm8-objcopy 
OBJSIZE ?= stm8-size
FLASH	?= stm8flash
OPENOCD ?= openocd
STAT    ?= stat

# Flags
ASFLAGS = -plogsgff
CFLAGS  = -m$(ARCH) -p$(MCU) --std-sdcc99
CFLAGS += -DF_CPU=$(F_CPU)UL -DBAUDRATE=$(SERIAL_BAUD) -I$(INCDIR) -I$(LIBDIR) $(CFLAGS_EXTRA)
CFLAGS += --opt-code-size --stack-auto 
#CFLAGS += --stack-auto --noinduction --opt-code-size
#--all-callee-saves --verbose --fverbose-asm --no-peep

LDFLAGS = -m$(ARCH) -l$(ARCH)


# Objects
SRC 	:= $(wildcard $(SRCDIR)/*.c $(LIBDIR)/*.c )
SRCAS 	:= $(wildcard $(SRCDIR)/*.s $(LIBDIR)/**/*.s )

# Debug and relase objects
FOBJS    = $(SRC:%.c=%.rel)
DOBJS    = $(addprefix $(BUILDDIR)/debug/,$(notdir $(FOBJS)) )
#DOBJS  += $(SRC:%.s=$(BUILDDIR)/debug/%.rel)

ROBJS   = $(addprefix $(BUILDDIR)/release/,$(notdir $(FOBJS)) )
#OBJS  += $(SRC:%.s=$(BUILDDIR)/release/%.rel)

# tester sources
PY_SOURCES = misc/ina219.py misc/logging.py misc/pcf8574.py misc/uploader.py misc/vpi.py misc/utelnet.py
PY_OBJS =$(addprefix $(BUILDDIR)/,$(PY_SOURCES:%.py=%.mpy) )

# Build debug & release
all: clean debug release

# Directory creation
debug_dir:
	@mkdir -p $(BUILDDIR)
	@mkdir -p $(BUILDDIR)/debug

relase_dir:
	@mkdir -p $(BUILDDIR)
	@mkdir -p $(BUILDDIR)/release

test_dir:
	@mkdir -p $(BUILDDIR)/misc

$(BUILDDIR)/debug/%.rel: **/%.c
	echo "Compiling debug [$< -> $@]"
	@$(CC) $(CFLAGS) --debug -c $< -o $@

$(BUILDDIR)/release/%.rel: **/%.c
	@echo "Compiling release [$< -> $@]"	
	@$(CC) $(CFLAGS) -c $< -o $@
	

debug: debug_dir $(DOBJS)
	@echo "Linking Debug main+[$(DOBJS)]"
	$(CC) $(CFLAGS) $(LDFLAGS)  --out-fmt-ihx --debug -o $(BUILDDIR)/debug/firmware.ihx main.c $(DOBJS) 
	$(CC) $(CFLAGS) $(LDFLAGS)  --out-fmt-elf -o $(BUILDDIR)/debug/firmware.elf main.c $(DOBJS)
	$(OBJCOPY) -I ihex -O binary $(BUILDDIR)/debug/firmware.ihx $(BUILDDIR)/debug/firmware.bin
	$(OBJSIZE) -d $(BUILDDIR)/debug/firmware.elf

release: relase_dir $(ROBJS)
	@echo "Linking Release main+[$(ROBJS)]"
	@$(CC) $(CFLAGS) $(LDFLAGS)  --out-fmt-ihx -o $(BUILDDIR)/release/firmware.ihx main.c $(ROBJS)
	@echo "Built ihx!"
	@$(CC) $(CFLAGS) $(LDFLAGS)  --out-fmt-elf -o $(BUILDDIR)/release/firmware.elf main.c $(ROBJS)
	@echo "Built elf!"
	@$(CC) $(CFLAGS) $(LDFLAGS)  --code-loc 0x8300 -o $(BUILDDIR)/release/firmware_boot.ihx main.c $(ROBJS)
	@echo "Built ihx for bootloader!"
	@$(OBJCOPY) -I ihex -O binary $(BUILDDIR)/release/firmware.ihx $(BUILDDIR)/release/firmware.bin
	@$(OBJCOPY) -I ihex -O binary $(BUILDDIR)/release/firmware_boot.ihx $(BUILDDIR)/release/firmware_boot.bin
	@$(OBJSIZE) -d  $(BUILDDIR)/release/firmware.elf
	@echo "Binary Size:"
	@$(STAT) $(BUILDDIR)/release/firmware.bin
	@$(STAT) $(BUILDDIR)/release/firmware_boot.bin


flash: release
	@echo "Uploading Release...."
	$(FLASH) -c stlinkv2 -p $(MCU) -s flash -w $(BUILDDIR)/release/firmware.ihx

flash-remote: release
	@echo "Uploading Remote..."
	#scp $(BUILDDIR)/release/firmware_boot.bin boros@dulcinea:/home/boros/vpid/firmware.bin
	#ssh dulcinea -t "cd vpid && target/debug/vpidctl firmware firmware.bin"
	scp $(BUILDDIR)/release/firmware_boot.bin boros@dulcinea:/home/boros/vpid_bin/firmware.bin
	ssh dulcinea -t "cd vpid_bin && ./vpidctl firmware firmware.bin"

bootloader:
	@echo "Cleaning and generating OPT bits..."
	cd bootloader && ./generate_optbits.sh
	@echo "Flashing OPT bits..."
	$(FLASH) -c stlinkv2 -p $(MCU) -s opt -w bootloader/rst.bin
	#@sleep 1 Enable AFR7 is not required in STMS8003 but in other chips yes
	#@$(FLASH) -c stlinkv2 -p $(MCU) -s opt -w bootloader/beep_enable.bin
	@rm bootloader/rst.bin bootloader/beep_enable.bin
	$(FLASH) -c stlinkv2 -p $(MCU) -s opt -r opt.bin
	@echo "Readed OPT bits"
	@xxd opt.bin
	@rm opt.bin
	@echo "Flashing Bootloader..."
	$(FLASH) -c stlinkv2 -p $(MCU) -s flash -w bootloader/empty.bin
	$(FLASH) -c stlinkv2 -p $(MCU) -s flash -w bootloader/i2c_overwrite_0x8300.ihx

$(BUILDDIR)/%.mpy: %.py
	@echo "Compile upython [$< -> $@]"
	@python -m mpy_cross -O3 -o $@ -march=xtensa $<

testboard: test_dir release $(PY_OBJS)
	@echo "Uploading firmaware to test board [Press reset on the board]..."
	@cp misc/main.py $(BUILDDIR)/misc/main.py
	@cp $(BUILDDIR)/release/firmware_boot.bin $(BUILDDIR)/misc/firmware.bin
	@rshell   -p "$(shell python list_serial.py)"  rsync $(BUILDDIR)/misc /pyboard
	#@rshell -p "$(shell python list_serial.py)"  cp misc/*.mpy /pyboard/
	#@rshell -p "$(shell python list_serial.py)"  cp misc/main.py /pyboard/main.py
	#@rshell -p "$(shell python list_serial.py)"  cp $(BUILDDIR)/release/firmware_boot.bin /pyboard/firmware.bin	

testboard-flash: testboard
	@echo "Flashing firmware on test board..."
	#@rshell -p "$(shell python list_serial.py)" cp misc/main.py /pyboard/main.py
	#@rshell -p "$(shell python list_serial.py)" cp $(BUILDDIR)/release/firmware_boot.bin /pyboard/firmware.bin
	#@rshell -p "$(shell python list_serial.py)" --quiet "repl ~ import machine ~ machine.reset() ~"
	@rshell -p "$(shell python list_serial.py)" "repl ~ import main ~ main.upload_firmware()"
	

qopenocd:
	@echo "Stopping any openocd running..."
	-@killall openocd
	@echo "Done"

openocd: debug qopenocd
	@echo "Opening OpenOCD....."
	$(OPENOCD) -f interface/stlink.cfg -f target/stm8s003.cfg -c "init" -c "reset halt"


clean:
	rm -fr $(BUILDDIR)

.PHONY: all release_dir debug_dir clean debug openocd flash qopenocd bootloader	
