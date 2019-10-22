# VPI Firmware

This folder contains the source code for VPI main board controller. The development is baremetal and do not require external libraries or development enviroment.


## Project structure

 - booloader: binary i2c bootloader for STM8S003
 - build: Build folder
 - docs: Documentation associated with the project
 - inc: includes.
 - lib: minimal utilities and headers for STMS 
 - misc: Some misc files for development and testing
 - src: Source


# Build notes

- Build is supported in linux and macos with make software installed and python.
- running toolchain in macos require some libraries be sure to have ```Xcode``` installed. The easisest way is to install gawk via ```brew install gawk````
- For installing the toolchain run ```tools.py``` script this will install SDDC and STM8S tools under the directory toolchain


