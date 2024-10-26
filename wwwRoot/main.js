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
  else
  {
    document.getElementById("effectCustomSetup").innerHTML='';
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
        if (this.status == 200) { 
          element.innerHTML = this.responseText; 
          var scripts = element.getElementsByTagName("script"); 
          for (i = 0; i < scripts.length; i++)
          {
            eval(scripts[i].innerText);
          }
        }
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

/* light effects functions */
function setProperty(name, value) {
  if (name == "colors") {
    setColorPattern(value);
    return;
  }

  var elm = document.getElementById("eff-cfg-" + name);
  if (elm) {
    if (name == "ping-pong") {
      elm.checked = value == 1;
    }
    else {
      elm.value = value;
    }
  }
}

function setColorPattern(colorPattern) {
  var colorsElm = document.getElementById("ColorPattern");
  colors = colorPattern.split(',');
  colorsElm.innerHTML = '';
  colors.forEach(color => {
    colorsElm.innerHTML += '<div class="d-flex flex-column"><button class="btn btn-outline-danger btn-sm mb-1 p-0" onClick="removeColor(this)">'
    +'<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" fill="currentColor" class="bi bi-x-circle" viewBox="0 0 16 16">'
    +'<path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14m0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16"/>'
    +'<path d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708"/>'
    +'</svg>'
    +'</button>'
    +'<input type="color" value="#' + color + '" /></div>';
  });
}

function removeColor(elm)
{
  elm.parentElement.remove();
}

function addColor() {
  elm = document.createElement('div');
  elm.classList.add('d-flex');
  elm.classList.add('flex-column');
  elm.innerHTML = '<button class="btn btn-outline-danger btn-sm mb-1 p-0" onClick="removeColor(this)">'
    +'<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" fill="currentColor" class="bi bi-x-circle" viewBox="0 0 16 16">'
    +'<path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14m0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16"/>'
    +'<path d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708"/>'
    +'</svg>'
    +'</button>'
    +'<input type="color" />';

  document.getElementById('ColorPattern').appendChild(elm);
  
}

function sendColorPattern() {
  colors = document.getElementById('ColorPattern').getElementsByTagName('input')
  colorsPattern = "";
  for (i=0; i<colors.length; i++)
  {
    if (i > 0)
      colorsPattern += ',';
    colorsPattern += colors[i].value.replace('#','');
  }
  httpGet('/set_effect_property?colors='+colorsPattern);
}

function configureEffectSettingsDialog(cfg) {
  properties = cfg.split('&');
  for (i = 0; i < properties.length; i++) {
    property = properties[i].split('=');
    setProperty(property[0], property[1]);
  }
}   