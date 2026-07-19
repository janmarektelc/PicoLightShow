# <img src="icon.png" width="30"> Pico Light Show

**Project Description:**

**PicoLightShow** is a project that utilizes the **Raspberry Pi Pico** microcontroller to create stunning light effects using **WS2812 LED strips**. This project is designed to allow users to easily control and configure various light effects through an integrated web server.

## Main Features:

- **Light Effects Generation:** The Raspberry Pi Pico controls WS2812 LED strips, enabling the creation of various light effects such as color transitions, blinking, waves, and other dynamic patterns.
- **Web Server:** An integrated web server allows users to connect to the Raspberry Pi Pico via Wi-Fi and control the light effects using a simple web interface.
- **User-Friendly Interface:** The web interface is designed to be intuitive and easy to use, allowing users to quickly change light effect settings without the need for programming.
- **OpenRGB / DDP Support:** Since version 0.9.3, the device natively supports the Distributed Display Protocol (DDP) on the default port 4048. This ensures seamless integration and real-time synchronization with OpenRGB.
- **Home Assistant Integration:** Since version 0.9.4, the device natively supports the **WLED JSON API**, allowing seamless integration into **Home Assistant** for effortless control of effects, brightness, speed, presets, playlists, and segment freezing by embedded **WLED** integration.

## Technical Specifications:

- **Microcontroller:** Raspberry Pi Pico
- **LED Strips:** WS2812 (addressable RGB LEDs)
- **Programming Language:** C++
- **Communication:** Wi-Fi (as an access point or connect to Wi-Fi network)
- **Supported Protocols & APIs:**
  - **WLED JSON API:** For Home Assistant and smart home automation
  - **OpenRGB / DDP:** Real-time LED control via Distributed Display Protocol (UDP port 4048)
- **Web Server:** Hosted on the Raspberry Pi Pico, accessible via a web browser

## Download
To download the latest version of the PicoLightShow software, please click [here](releases/PicoLightShow-0.9.3.uf2).

## Hardware Setup

For detailed instructions on how to wire the LED strip to the Raspberry Pi, please refer to the [Hardware Setup](hardware_setup.md). 

## Software Setup

For detailed instructions on how to connect to the Raspberry Pi, please refer to the [Software Setup](software_setup.md). 

## Development Guide:

For detailed instructions on how to develop the PicoLightShow project, please refer to the [Development Guide](development_guide.md). 