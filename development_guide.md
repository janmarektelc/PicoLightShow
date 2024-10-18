# <img src="icon.png" width="30"> How to Develop Pico Light Show

**Development Environment:**

The **PicoLightShow** project is developed using **Visual Studio Code** with the **Raspberry Pi Pico** addon. Follow these steps to set up your development environment and start working on the project.

## Prerequisites:

1. **Visual Studio Code:** Ensure you have Visual Studio Code installed on your computer. You can download it from [Visual Studio Code](https://code.visualstudio.com/).
2. **Raspberry Pi Pico Addon:** Install the Raspberry Pi Pico addon for Visual Studio Code. This addon simplifies the development process for Raspberry Pi Pico projects.
3. **Python:** Make sure Python is installed on your computer. You can download it from Python.org. Python is needed only when the web interface of is updated.

## Steps to Develop:

1. **Set Up Visual Studio Code:**
   - Open Visual Studio Code.
   - Install the Raspberry Pi Pico addon by navigating to the Extensions view (Ctrl+Shift+X) and searching for "Pico".
   - Install the addon and restart Visual Studio Code if necessary.

2. **Write Your Code:**
   - Develop your C++ code to control the WS2812 LED strips and handle the web server functionality.
   - Save your code files in the project folder.

3. **Update Web Interface:**
   - If you make changes to the web interface, you need to run the `makefsdata.bat` script to update the filesystem data.
   - Ensure Python is installed on your computer, as the script requires Python to run.

4. **Run `makefsdata.bat`:**
   - Open a terminal in Visual Studio Code.
   - Navigate to the directory containing the `makefsdata.bat` script.
   - Run the script by typing `makefsdata.bat` and pressing Enter.

5. **Upload Code to Raspberry Pi Pico:**
   - Connect your Raspberry Pi Pico to your computer via USB.
   - Use the Visual Studio Code interface to upload your code to the Raspberry Pi Pico.

6. **Test and Debug:**
   - Open a web browser and connect to the Raspberry Pi Pico's web server.
   - Test the light effects and make any necessary adjustments to your code.
