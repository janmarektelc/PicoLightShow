function httpGet(theUrl)
{
    var xmlHttp = new XMLHttpRequest();
    xmlHttp.open( "GET", theUrl, false ); // false for synchronous request
    xmlHttp.send( null );
    return xmlHttp.responseText;
}

function runSwitchChanged(checked)
{
    if (checked)
        httpGet('/start_effect');
    else
        httpGet('/stop_effect');
}

function setRunnerSpeedFromRange(speed)
{
    document.getElementById('speedNumber').value = speed;
    httpGet('/configure_runner?speed='+speed);
}

function setRunnerSpeedFromNumber(speed)
{
    document.getElementById('speedRange').value = speed;
    httpGet('/configure_runner?speed='+speed);
}

function setRunnerBrightnessFromRange(brightness)
{
    document.getElementById('brightnessNumber').value = brightness;
    httpGet('/configure_runner?brightness='+brightness);
}

function setRunnerBrightnessFromNumber(brightness)
{
    document.getElementById('brightnessRange').value = brightness;
    httpGet('/configure_runner?brightness='+brightness);
}

function setLedCount(ledCount)
{
  httpGet('/configure_runner?ledCount='+ledCount);
}

function switchLightEffect(effect)
{
  httpGet('/switch_effect?effect='+effect);
}

function ipConfigAvailabilityReDraw()
{
  var isClientWifiMode = document.getElementById("wifiModeSelection").value == "1";
  var isDhcp = document.getElementById("isDhcp").checked;

  document.getElementById("ipDhcpRow").hidden = !isClientWifiMode;

  document.getElementById("ipAddressRow").hidden = !(isClientWifiMode && !isDhcp);
  document.getElementById("ipMaskRow").hidden = !(isClientWifiMode && !isDhcp);
  document.getElementById("ipGatewayRow").hidden = !(isClientWifiMode && !isDhcp);

}

function saveSettings()
{
  console.log("save settings");
}

function resetToDefault()
{
  httpGet('/reset_to_default');
  location.reload()
}

function rebootDevice()
{
  httpGet('/reboot');
  location.reload()
}

function includeHTML() {
    var z, i, elmnt, file, xhttp;
    /* Loop through a collection of all HTML elements: */
    z = document.getElementsByTagName("*");
    for (i = 0; i < z.length; i++) {
      elmnt = z[i];
      /*search for elements with a certain atrribute:*/
      file = elmnt.getAttribute("include-html");
      if (file) {
        /* Make an HTTP request using the attribute value as the file name: */
        xhttp = new XMLHttpRequest();
        xhttp.onreadystatechange = function() {
          if (this.readyState == 4) {
            if (this.status == 200) {elmnt.innerHTML = this.responseText;}
            if (this.status == 404) {elmnt.innerHTML = "Page not found.";}
            /* Remove the attribute, and call this function once more: */
            elmnt.removeAttribute("include-html");
            includeHTML();
          }
        }
        xhttp.open("GET", file, true);
        xhttp.send();
        /* Exit the function: */
        return;
      }
    }
  }