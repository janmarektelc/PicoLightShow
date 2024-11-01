# <img src="icon.png" width="30"> Connecting the WS2812 LED Strip to the Raspberry Pi Pico

## Hardware

1. **Components:**
   - Raspberry Pi Pico
   - WS2812 LED strip
   - Power supply (5V)
   - Connecting wires

2. **Wiring:**
   - **Power:** Connect the power wire of the LED strip to the 5V output on the Raspberry Pi Pico.
   - **Ground:** Connect the ground wire of the LED strip to a GND pin on the Raspberry Pi Pico.
   - **Data:** Connect the data wire of the LED strip to pin 2 on the Raspberry Pi Pico.

## Software

1. **Pin Configuration:**
   - In the `constants.h` file, you can change the pin configuration for connecting the LED strip. The default setting is:
     ```cpp
     #define WS2812_PIN 2
     ```

2. **Uploading Code:**
   - Upload the C++ code to the Raspberry Pi Pico, which includes the control for the LED strip and the web server setup.

## Notes

- Ensure that the power supply provides sufficient current for all the LEDs on the strip.
- If you change the pin configuration in the `constants.h` file, make sure to adjust the hardware wiring accordingly.
