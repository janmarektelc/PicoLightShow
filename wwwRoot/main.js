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
  loadSelectedLightEffectConfigPage();
}

function loadSelectedLightEffectConfigPage()
{
  effectSelection = document.getElementById("lightEffectSelect");
  setupPage = effectSelection.options[effectSelection.selectedIndex].getAttribute("data-setup-page")
  if (setupPage)
  {
    loadAndPlaceHTML(setupPage, document.getElementById("effectCustomSetup"));
  }
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
  var password = '*^NotChanged^*';
  if (usePassword)
  {
    if (document.getElementById("wifiPass").value != document.getElementById("wifiPass1").value)
    {
      var myModal = new bootstrap.Modal(document.getElementById('incorrectPasswordModal')); 
      myModal.show(); 

      return;
    }
    password = document.getElementById("wifiPass").value;
    if (password == '')
      password = '*^Empty^*';
  }

  var xhr = new XMLHttpRequest();
  xhr.open("POST", "/network_data", true);
  xhr.setRequestHeader('Content-Type', 'text/plain');
  data = '~'+document.getElementById("wifiModeSelection").value+'~'
    +document.getElementById("wifiSSID").value+'~'
    +password+'~'
    +(document.getElementById("isDhcp").checked ? '1~' : '0~')
    +document.getElementById("ip1").value+'.'+document.getElementById("ip2").value+'.'+document.getElementById("ip3").value+'.'+document.getElementById("ip4").value+'~'
    +document.getElementById("mask1").value+'.'+document.getElementById("mask2").value+'.'+document.getElementById("mask3").value+'.'+document.getElementById("mask4").value+'~'
    +document.getElementById("gw1").value+'.'+document.getElementById("gw2").value+'.'+document.getElementById("gw3").value+'.'+document.getElementById("gw4").value+'~'
    ;
  xhr.onreadystatechange = function() {
    if (xhr.readyState  == XMLHttpRequest.DONE)
    {
      if (xhr.status == 200)
      {
        var okToast = document.getElementById('settingsSavedOkToast');
        var bsAlert = new bootstrap.Toast(okToast);
        bsAlert.show();
      }
      else
      {
        var errorToast = document.getElementById('settingsSavedErrorToast');
        var bsAlert = new bootstrap.Toast(errorToast);
        bsAlert.show();
      }
    }
  }
  xhr.send(data);
  
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

var usePassword = false;
function passwordChanged()
{
  usePassword = true;
}

function loadAndPlaceHTML(fileName, element)
{    
  if (fileName) {
    xhttp = new XMLHttpRequest();
    xhttp.onreadystatechange = function () {
      if (this.readyState == 4) {
        if (this.status == 200) { element.innerHTML = this.responseText; }
        else { element.innerHTML = ""; }
      }
    }
    xhttp.open("GET", fileName, true);
    xhttp.send();
  }

}

function includeHTML() 
{
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
          if (this.status == 200) {
            elmnt.innerHTML = this.responseText; 
            var scripts = elmnt.getElementsByTagName("script"); 
            for (i = 0; i < scripts.length; i++)
            {
              eval(scripts[i].innerText);
            }
          }
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