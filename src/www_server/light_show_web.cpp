#include "include/www_server/light_show_web.h"
#include "include/persistent_settings/persistent_settings.h"
#include "include/wled_provider/wled_provider.h"
#include "include/light_show_runner/light_show_runner.h"
#include "include/www_server/assets.h"
#include <hardware/watchdog.h>

namespace PicoLightShow
{
    HttpResponse LightShowWeb::onRequest(const HttpRequest& req) {
        if (req.uri == "/index.html" || req.uri == "/" || req.uri == "/index.htm" || req.uri == "/index") {
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::index_html, PicoLightShow::Assets::index_html_size), "text/html"};
        }
        if (req.uri == "/favicon.ico") {
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::favicon_ico, PicoLightShow::Assets::favicon_ico_size), "image/x-icon"};
        }
        if (req.uri == "/main.js") {
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::main_js, PicoLightShow::Assets::main_js_size), "application/javascript"};
        }
        if (req.uri == "/bootstrap.min.css") {
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::bootstrap_min_css, PicoLightShow::Assets::bootstrap_min_css_size), "text/css"};
        }
        if (req.uri == "/bootstrap.bundle.min.js") {
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::bootstrap_bundle_min_js, PicoLightShow::Assets::bootstrap_bundle_min_js_size), "application/javascript"};
        }
        if (req.uri == "/favicon1.png") {
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::favicon1_png, PicoLightShow::Assets::favicon1_png_size), "image/png"};
        }
        if (req.uri == "/appletouchicon.png") {
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::appletouchicon_png, PicoLightShow::Assets::appletouchicon_png_size), "image/png"};
        }
        if (req.uri == "/start_effect") {
            LightShowRunner::Start();
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::success_html, PicoLightShow::Assets::success_html_size), "text/html"};
        }
        if (req.uri == "/stop_effect") {
            LightShowRunner::Stop();
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::success_html, PicoLightShow::Assets::success_html_size), "text/html"};
        }
        if (req.uri == "/switch_on") {
            LightShowRunner::SwitchOn();
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::success_html, PicoLightShow::Assets::success_html_size), "text/html"};
        }
        if (req.uri == "/switch_off") {
            LightShowRunner::SwitchOff();
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::success_html, PicoLightShow::Assets::success_html_size), "text/html"};
        }
        if (req.uri == "/configure_runner") {
            auto speed = req.parameters.find("speed");
            if (speed != req.parameters.end()) {
                LightShowRunner::SetFrameDelay(std::stoi(speed->second));
            }
            auto brightness = req.parameters.find("brightness");
            if (brightness != req.parameters.end()) {
                LightShowRunner::SetBrightness(std::stoi(brightness->second));
            }
            auto ledCount = req.parameters.find("ledCount");
            if (ledCount != req.parameters.end()) {
                LightShowRunner::SetLedCount(std::stoi(ledCount->second));
            }
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::success_html, PicoLightShow::Assets::success_html_size), "text/html"};
        }
        if (req.uri == "/set_effect_property") {

            for (const auto& [name, value] : req.parameters) {
                LightShowRunner::SetEffectProperty(name.c_str(), value.c_str());
            }
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::success_html, PicoLightShow::Assets::success_html_size), "text/html"};
        }        
        if (req.uri == "/switch_effect") {
            LightShowRunner::SetEffect(std::stoi(req.parameters.at("effect")));
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::success_html, PicoLightShow::Assets::success_html_size), "text/html"};
        }
        if (req.uri == "/reboot" || req.uri == "/reset") {
            watchdog_reboot(0,0,100);
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::success_html, PicoLightShow::Assets::success_html_size), "text/html"};
        }
        if (req.uri == "/reset_to_default") {
            PersistentSettings::SetToDefault();
            PersistentSettings::Save();
            watchdog_reboot(0,0,100);
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::success_html, PicoLightShow::Assets::success_html_size), "text/html"};
        }
        if (req.uri == "/network_data") {
            //expected data format
            //~wifiMode(ap=0/client=1)~ssid~passwd~isdhcp(0/1)~ipAddress~ipMask~gwIp~
            PicoLightShow::PersistentSettings::SetByConfigString(req.body);
            strcpy(PicoLightShow::PersistentSettings::Settings.CurrentEffectConfiguration, PicoLightShow::LightShowRunner::GetEffectConfigurationString().c_str());
            PicoLightShow::PersistentSettings::Save();
            return {200, std::make_unique<FlashSource>(PicoLightShow::Assets::success_html, PicoLightShow::Assets::success_html_size), "text/html"};
        }        
        if (req.uri == "/led_control.shtml") {
            std::string isRunning = LightShowRunner::GetIsRunning()? "checked" : "";
            std::string isSwitchOn = LightShowRunner::GetSwitchOn()? "checked" : "";
            std::string delay = std::to_string(LightShowRunner::GetFrameDelay());
            std::string brightness = std::to_string(LightShowRunner::GetBrightness());

            std::string html = R"raw(<div class="d-flex flex-column">
                <!-- on/off switch -->
                <div class="d-inline-flex align-items-center pb-2">
                    <div class="d-flex align-items-center">
                        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-power" viewBox="0 0 16 16">
                            <path d="M7.5 1v7h1V1z"/>
                            <path d="M3 8.812a5 5 0 0 1 2.578-4.375l-.485-.874A6 6 0 1 0 11 3.616l-.501.865A5 5 0 1 1 3 8.812"/>
                        </svg>
                    </div>
                    <div class="form-check form-switch mx-2">
                        <input class="form-check-input" type="checkbox" role="switch" )raw" + isSwitchOn + R"raw( onchange="checked ? httpGet('/switch_on') : httpGet('/switch_off')" />
                    </div>
                </div>

                <!-- run switch -->
                <div class="d-inline-flex align-items-center pb-2">
                    <div class="d-flex align-items-center">
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-repeat" viewBox="0 0 16 16">
                            <path d="M11 5.466V4H5a4 4 0 0 0-3.584 5.777.5.5 0 1 1-.896.446A5 5 0 0 1 5 3h6V1.534a.25.25 0 0 1 .41-.192l2.36 1.966c.12.1.12.284 0 .384l-2.36 1.966a.25.25 0 0 1-.41-.192m3.81.086a.5.5 0 0 1 .67.225A5 5 0 0 1 11 13H5v1.466a.25.25 0 0 1-.41.192l-2.36-1.966a.25.25 0 0 1 0-.384l2.36-1.966a.25.25 0 0 1 .41.192V12h6a4 4 0 0 0 3.585-5.777.5.5 0 0 1 .225-.67Z"/>
                        </svg>
                    </div>
                    <div class="form-check form-switch mx-2">
                        <input class="form-check-input" type="checkbox" role="switch" )raw" + isRunning + R"raw( onchange="runSwitchChanged(checked)" />
                    </div>
                </div>

                <!-- animation speed -->
                <div class="d-inline-flex align-items-center">
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-speedometer" viewBox="0 0 16 16">
                        <path d="M8 2a.5.5 0 0 1 .5.5V4a.5.5 0 0 1-1 0V2.5A.5.5 0 0 1 8 2M3.732 3.732a.5.5 0 0 1 .707 0l.915.914a.5.5 0 1 1-.708.708l-.914-.915a.5.5 0 0 1 0-.707M2 8a.5.5 0 0 1 .5-.5h1.586a.5.5 0 0 1 0 1H2.5A.5.5 0 0 1 2 8m9.5 0a.5.5 0 0 1 .5-.5h1.5a.5.5 0 0 1 0 1H12a.5.5 0 0 1-.5-.5m.754-4.246a.39.39 0 0 0-.527-.02L7.547 7.31A.91.91 0 1 0 8.85 8.569l3.434-4.297a.39.39 0 0 0-.029-.518z"/>
                        <path fill-rule="evenodd" d="M6.664 15.889A8 8 0 1 1 9.336.11a8 8 0 0 1-2.672 15.78zm-4.665-4.283A11.95 11.95 0 0 1 8 10c2.186 0 4.236.585 6.001 1.606a7 7 0 1 0-12.002 0"/>
                    </svg>
                    <div class="mx-2">Speed</div>
                    <input type="number" min="1" max="255" value=")raw" + delay + R"raw(" id="speedNumber" class="ms-auto" oninput="setRunnerSpeedFromNumber(value)" />
                </div>
                <div class="d-inline-flex align-items-center pb-2">
                    <input style="width: 100%;" type="range" class="form-range" min="1" max="255" value=")raw" + delay + R"raw(" id="speedRange" oninput="setRunnerSpeedFromRange(value)" />
                </div>

                <!-- brightness -->
                <div class="d-inline-flex align-items-center">
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-brightness-high-fill" viewBox="0 0 16 16">
                        <path d="M12 8a4 4 0 1 1-8 0 4 4 0 0 1 8 0M8 0a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 8 0m0 13a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 8 13m8-5a.5.5 0 0 1-.5.5h-2a.5.5 0 0 1 0-1h2a.5.5 0 0 1 .5.5M3 8a.5.5 0 0 1-.5.5h-2a.5.5 0 0 1 0-1h2A.5.5 0 0 1 3 8m10.657-5.657a.5.5 0 0 1 0 .707l-1.414 1.415a.5.5 0 1 1-.707-.708l1.414-1.414a.5.5 0 0 1 .707 0m-9.193 9.193a.5.5 0 0 1 0 .707L3.05 13.657a.5.5 0 0 1-.707-.707l1.414-1.414a.5.5 0 0 1 .707 0m9.193 2.121a.5.5 0 0 1-.707 0l-1.414-1.414a.5.5 0 0 1 .707-.707l1.414 1.414a.5.5 0 0 1 0 .707M4.464 4.465a.5.5 0 0 1-.707 0L2.343 3.05a.5.5 0 1 1 .707-.707l1.414 1.414a.5.5 0 0 1 0 .708"/>
                    </svg>
                    <div class="mx-2">Brightness</div>
                    <input type="number" min="0" max="255" value=")raw" + brightness + R"raw(" id="brightnessNumber" class="ms-auto" oninput="setRunnerBrightnessFromNumber(value)" />
                </div>
                <div class="d-inline-flex align-items-center">
                    <input style="width: 100%;" type="range" class="form-range" min="0" max="255" value=")raw" + brightness + R"raw(" id="brightnessRange" oninput="setRunnerBrightnessFromRange(value)" />
                </div>
            </div>)raw";

            HttpResponse res;
            res.statusCode = 200;
            res.contentType = "text/html";
            res.data = std::make_unique<RamSource>(html);
            return res;
        }
        if (req.uri == "/led_effect_setup.shtml") {

            int ledCount = LightShowRunner::GetLedCount();

            std::vector<std::string> effectNames = LightShowRunner::GetEffectNames();
            std::vector<std::string> setupPages = LightShowRunner::GetEffectSetupPages();
            int currentEffect = LightShowRunner::GetEffect();
            std::string effectsList = "";
            for (size_t i = 0; i < effectNames.size(); i++) {
                effectsList += "<option value=\"" + std::to_string(i) + 
                            "\" data-setup-page=\"" + setupPages[i] + "\"";
                if (static_cast<int>(i) == currentEffect) {
                    effectsList += " selected=\"selected\"";
                }
                effectsList += ">" + effectNames[i] + "</option>";
            }

            std::string html = R"raw(<div class="d-flex flex-column">
                <div class="d-sm-inline-flex align-items-center">
                    <select class="form-select" id="lightEffectSelect" onchange="switchLightEffect(value)">
                        )raw" + effectsList + R"raw(
                    </select>
                    <div class="d-inline-flex align-items-center pt-2 pt-sm-0 ps-sm-2">
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-lightbulb-fill" viewBox="0 0 16 16">
                            <path d="M2 6a6 6 0 1 1 10.174 4.31c-.203.196-.359.4-.453.619l-.762 1.769A.5.5 0 0 1 10.5 13h-5a.5.5 0 0 1-.46-.302l-.761-1.77a2 2 0 0 0-.453-.618A5.98 5.98 0 0 1 2 6m3 8.5a.5.5 0 0 1 .5-.5h5a.5.5 0 0 1 0 1l-.224.447a1 1 0 0 1-.894.553H6.618a1 1 0 0 1-.894-.553L5.5 15a.5.5 0 0 1-.5-.5"/>
                        </svg>
                        <div class="mx-2 text-nowrap">LED count</div>
                        <input type="number" min="1" max="500" value=")raw" + std::to_string(ledCount) + R"raw(" onchange="setLedCount(value)" />
                    </div>
                </div>
                <div id="effectCustomSetup"></div>
            </div>
            <script>
                loadSelectedLightEffectConfigPage();
            </script>)raw";

            HttpResponse res;
            res.statusCode = 200;
            res.contentType = "text/html";
            res.data = std::make_unique<RamSource>(html);
            return res;
        }
        if (req.uri == "/single_color_setup.shtml") {
            printf("Current effect configuration: %s\n", LightShowRunner::GetEffectConfigurationString().c_str());
            printf("Color value: %s\n", LightShowRunner::GetEffectConfigurationString().substr(6).c_str());
            std::string html = R"raw(<div class="row align-items-center mt-2">
                <div class="d-inline-flex flex-wrap gap-2 align-items-center mb-2" id="ColorSettings"></div>
                <input type="color" value="#)raw" + LightShowRunner::GetEffectConfigurationString().substr(6) + R"raw(" onchange="httpGet('/set_effect_property?color=' + this.value.replace('#', ''))" />
            </div>)raw";

            HttpResponse res;
            res.statusCode = 200;
            res.contentType = "text/html";
            res.data = std::make_unique<RamSource>(html);
            return res;
        }

        if (req.uri == "/change_color_setup.shtml") {
            std::string html = R"raw(<div class="row align-items-center mt-2">
                <div class="d-inline-flex flex-wrap gap-2 align-items-center mb-2" id="ColorPattern"></div>

                <button type="button" class="btn btn-primary col-sm-4 col-12 text-nowrap mb-2 mb-sm-0 me-sm-2" onclick="addColor()">
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-plus-circle me-2" viewBox="0 0 16 16">
                        <path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14m0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16"/>
                        <path d="M8 4a.5.5 0 0 1 .5.5v3h3a.5.5 0 0 1 0 1h-3v3a.5.5 0 0 1-1 0v-3h-3a.5.5 0 0 1 0-1h3v-3A.5.5 0 0 1 8 4"/>
                    </svg>
                    Add Color
                </button>

                <button type="button" class="btn btn-primary col-sm-4 col-12 text-nowrap mb-2 mb-sm-0" onclick="sendColorPattern()">
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-save2-fill me-2" viewBox="0 0 16 16">
                        <path d="M8.5 1.5A1.5 1.5 0 0 1 10 0h4a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2V2a2 2 0 0 1 2-2h6c-.314.418-.5.937-.5 1.5v6h-2a.5.5 0 0 0-.354.854l2.5 2.5a.5.5 0 0 0 .708 0l2.5-2.5A.5.5 0 0 0 10.5 7.5h-2z"/>
                    </svg>
                    Set Colors
                </button>
            </div>

            <script>
                configureEffectSettingsDialog(')raw" + LightShowRunner::GetEffectConfigurationString() + R"raw(');
            </script>)raw";

            HttpResponse res;
            res.statusCode = 200;
            res.contentType = "text/html";
            res.data = std::make_unique<RamSource>(html);
            return res;
        }
        if (req.uri == "/custom_pattern_setup.shtml") {
            std::string html = R"raw(<div class="row align-items-center mt-2">
                <div class="d-inline-flex flex-wrap gap-2 align-items-center mb-2" id="ColorPattern"></div>

                <button type="button" class="btn btn-primary col-sm-4 col-12 text-nowrap mb-2 mb-sm-0 me-sm-2" onclick="addColor()">
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-plus-circle me-2" viewBox="0 0 16 16">
                        <path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14m0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16"/>
                        <path d="M8 4a.5.5 0 0 1 .5.5v3h3a.5.5 0 0 1 0 1h-3v3a.5.5 0 0 1-1 0v-3h-3a.5.5 0 0 1 0-1h3v-3A.5.5 0 0 1 8 4"/>
                    </svg>
                    Add Color
                </button>

                <button type="button" class="btn btn-primary col-sm-4 col-12 text-nowrap mb-2 mb-sm-0" onclick="sendColorPattern()">
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-save2-fill me-2" viewBox="0 0 16 16">
                        <path d="M8.5 1.5A1.5 1.5 0 0 1 10 0h4a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2V2a2 2 0 0 1 2-2h6c-.314.418-.5.937-.5 1.5v6h-2a.5.5 0 0 0-.354.854l2.5 2.5a.5.5 0 0 0 .708 0l2.5-2.5A.5.5 0 0 0 10.5 7.5h-2z"/>
                    </svg>
                    Set Colors
                </button>
            </div>

            <div class="row align-items-center mt-2">
                <div class="col-sm-4">
                    <div class="d-inline-flex align-items-center pt-2 pt-sm-0 ps-sm-2">
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-pencil-fill" viewBox="0 0 16 16">
                            <path d="M12.854.146a.5.5 0 0 0-.707 0L10.5 1.793 14.207 5.5l1.647-1.646a.5.5 0 0 0 0-.708zm.646 6.061L9.793 2.5 3.293 9H3.5a.5.5 0 0 1 .5.5v.5h.5a.5.5 0 0 1 .5.5v.5h.5a.5.5 0 0 1 .5.5v.5h.5a.5.5 0 0 1 .5.5v.207zm-7.468 7.468A.5.5 0 0 1 6 13.5V13h-.5a.5.5 0 0 1-.5-.5V12h-.5a.5.5 0 0 1-.5-.5V11h-.5a.5.5 0 0 1-.5-.5V10h-.5a.5.5 0 0 1-.175-.032l-.179.178a.5.5 0 0 0-.11.168l-2 5a.5.5 0 0 0 .65.65l5-2a.5.5 0 0 0 .168-.11z"/>
                        </svg>
                        <div class="mx-2 text-nowrap">Draw Mode</div>
                    </div>
                </div>
                <div class="col-sm-8">
                    <select class="form-select" id="eff-cfg-draw-kind" onchange="httpGet('/set_effect_property?draw-kind='+value)">
                        <option value="0">Once</option>
                        <option value="1">Repeat</option>
                        <option value="2">Stretch</option>
                    </select>
                </div>
            </div>

            <div class="row align-items-center mt-2">
                <div class="col-sm-4">
                    <div class="d-inline-flex align-items-center pt-2 pt-sm-0 ps-sm-2">
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-arrow-left-right" viewBox="0 0 16 16">
                            <path fill-rule="evenodd" d="M1 11.5a.5.5 0 0 0 .5.5h11.793l-3.147 3.146a.5.5 0 0 0 .708.708l4-4a.5.5 0 0 0 0-.708l-4-4a.5.5 0 0 0-.708.708L13.293 11H1.5a.5.5 0 0 0-.5.5m14-7a.5.5 0 0 1-.5.5H2.707l3.147 3.146a.5.5 0 1 1-.708.708l-4-4a.5.5 0 0 1 0-.708l4-4a.5.5 0 1 1 .708.708L2.707 4H14.5a.5.5 0 0 1 .5.5"/>
                        </svg>
                        <div class="mx-2 text-nowrap">Ping pong</div>
                    </div>
                </div>
                <div class="col-sm-8">
                    <div class="form-check form-switch">
                        <input class="form-check-input" type="checkbox" role="switch" id="eff-cfg-ping-pong" onchange="httpGet('/set_effect_property?ping-pong='+(checked?1:0)+'&direction='+document.getElementById('eff-cfg-direction').value)" />
                    </div>
                </div>
            </div>

            <div class="row align-items-center mt-2">
                <div class="col-sm-4">
                    <div class="d-inline-flex align-items-center pt-2 pt-sm-0 ps-sm-2">
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-signpost-split-fill" viewBox="0 0 16 16">
                            <path d="M7 16h2V6h5a1 1 0 0 0 .8-.4l.975-1.3a.5.5 0 0 0 0-.6L14.8 2.4A1 1 0 0 0 14 2H9v-.586a1 1 0 0 0-2 0V7H2a1 1 0 0 0-.8.4L.225 8.7a.5.5 0 0 0 0 .6l.975 1.3a1 1 0 0 0 .8.4h5z"/>
                        </svg>
                        <div class="mx-2 text-nowrap">Direction</div>
                    </div>
                </div>
                <div class="col-sm-8">
                    <select class="form-select" id="eff-cfg-direction" onchange="httpGet('/set_effect_property?direction='+value)">
                        <option value="0">Left</option>
                        <option value="1">Right</option>
                    </select>
                </div>
            </div>

            <script>
                configureEffectSettingsDialog(')raw" + LightShowRunner::GetEffectConfigurationString() + R"raw(');
            </script>)raw";

            HttpResponse res;
            res.statusCode = 200;
            res.contentType = "text/html";
            res.data = std::make_unique<RamSource>(html);
            return res;
        }
        if (req.uri == "/connection_setup.shtml") {
            std::string wifiModes = PersistentSettings::Settings.WifiMode == AP ? "<option value=\"0\" selected=\"selected\">Access point</option><option value=\"1\">Client</option>" : "<option value=\"0\">Access point</option><option value=\"1\" selected=\"selected\">Client</option>";
            std::string isDhcp = PersistentSettings::Settings.IsDhcp ? "checked" : "";
            std::string html = R"raw(<div class="container p-0">
                <div class="alert alert-info" role="alert">
                    After changing the connection parameters, it is necessary to save these changes and restart the device to apply them. New connection settings will be used after the device reboots.
                </div>

                <div class="row align-items-center mb-2">
                    <div class="col-sm-4">
                        <div class="d-inline-flex align-items-center pt-2 pt-sm-0 ps-sm-2">
                            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-broadcast-pin" viewBox="0 0 16 16">
                                <path d="M3.05 3.05a7 7 0 0 0 0 9.9.5.5 0 0 1-.707.707 8 8 0 0 1 0-11.314.5.5 0 0 1 .707.707m2.122 2.122a4 4 0 0 0 0 5.656.5.5 0 1 1-.708.708 5 5 0 0 1 0-7.072.5.5 0 0 1 .708.708m5.656-.708a.5.5 0 0 1 .708 0 5 5 0 0 1 0 7.072.5.5 0 1 1-.708-.708 4 4 0 0 0 0-5.656.5.5 0 0 1 0-.708m2.122-2.12a.5.5 0 0 1 .707 0 8 8 0 0 1 0 11.313.5.5 0 0 1-.707-.707 7 7 0 0 0 0-9.9.5.5 0 0 1 0-.707zM6 8a2 2 0 1 1 2.5 1.937V15.5a.5.5 0 0 1-1 0V9.937A2 2 0 0 1 6 8"/>
                            </svg>
                            <div class="mx-2 text-nowrap">Wi-Fi Mode</div>
                        </div>
                    </div>
                    <div class="col-sm-8">
                        <select class="form-select" id="wifiModeSelection" onchange="ipConfigAvailabilityReDraw()">
                            )raw" + wifiModes + R"raw(
                        </select>
                    </div>
                </div>

                <div class="row align-items-center mb-2">
                    <div class="col-sm-4">
                        <div class="d-inline-flex align-items-center pt-2 pt-sm-0 ps-sm-2">
                            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-wifi" viewBox="0 0 16 16">
                                <path d="M15.384 6.115a.485.485 0 0 0-.047-.736A12.44 12.44 0 0 0 8 3C5.259 3 2.723 3.882.663 5.379a.485.485 0 0 0-.048.736.52.52 0 0 0 .668.05A11.45 11.45 0 0 1 8 4c2.507 0 4.827.802 6.716 2.164.205.148.49.13.668-.049"/>
                                <path d="M13.229 8.271a.482.482 0 0 0-.063-.745A9.46 9.46 0 0 0 8 6c-1.905 0-3.68.56-5.166 1.526a.48.48 0 0 0-.063.745.525.525 0 0 0 .652.065A8.46 8.46 0 0 1 8 7a8.46 8.46 0 0 1 4.576 1.336c.206.132.48.108.653-.065m-2.183 2.183c.226-.226.185-.605-.1-.75A6.5 6.5 0 0 0 8 9c-1.06 0-2.062.254-2.946.704-.285.145-.326.524-.1.75l.015.015c.16.16.407.19.611.09A5.5 5.5 0 0 1 8 10c.868 0 1.69.201 2.42.56.203.1.45.07.61-.091zM9.06 12.44c.196-.196.198-.52-.04-.66A2 2 0 0 0 8 11.5a2 2 0 0 0-1.02.28c-.238.14-.236.464-.04.66l.706.706a.5.5 0 0 0 .707 0l.707-.707z"/>
                            </svg>
                            <div class="mx-2 text-nowrap">SSID</div>
                        </div>
                    </div>
                    <div class="col-sm-8">
                        <input class="w-100" id="wifiSSID" type="text" value=")raw" + PersistentSettings::Settings.WifiName + R"raw(" />
                    </div>
                </div>

                <div class="row align-items-center mb-2">
                    <div class="col-sm-4">
                        <div class="d-inline-flex align-items-center pt-2 pt-sm-0 ps-sm-2">
                            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-key-fill" viewBox="0 0 16 16">
                                <path d="M3.5 11.5a3.5 3.5 0 1 1 3.163-5H14L15.5 8 14 9.5l-1-1-1 1-1-1-1 1-1-1-1 1H6.663a3.5 3.5 0 0 1-3.163 2M2.5 9a1 1 0 1 0 0-2 1 1 0 0 0 0 2"/>
                            </svg>
                            <div class="mx-2 text-nowrap">Password</div>
                        </div>
                    </div>
                    <div class="col-sm-8">
                        <input class="w-100" id="wifiPass" type="password" value="" onchange="passwordChanged()" />
                    </div>
                </div>

                <div class="row align-items-center mb-2">
                    <div class="col-sm-4">
                        <div class="d-inline-flex align-items-center pt-2 pt-sm-0 ps-sm-2">
                            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-key-fill" viewBox="0 0 16 16">
                                <path d="M3.5 11.5a3.5 3.5 0 1 1 3.163-5H14L15.5 8 14 9.5l-1-1-1 1-1-1-1 1-1-1-1 1H6.663a3.5 3.5 0 0 1-3.163 2M2.5 9a1 1 0 1 0 0-2 1 1 0 0 0 0 2"/>
                            </svg>
                            <div class="mx-2 text-nowrap">Confirm Password</div>
                        </div>
                    </div>
                    <div class="col-sm-8">
                        <input class="w-100" id="wifiPass1" type="password" value="" onchange="passwordChanged()" />
                    </div>
                </div>

                <div class="row align-items-center mb-2" id="ipDhcpRow" hidden="true">
                    <div class="col-sm-4">
                        <div class="d-inline-flex align-items-center pt-2 pt-sm-0 ps-sm-2">
                            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-hdd-network-fill" viewBox="0 0 16 16">
                                <path d="M2 2a2 2 0 0 0-2 2v1a2 2 0 0 0 2 2h5.5v3A1.5 1.5 0 0 0 6 11.5H.5a.5.5 0 0 0 0 1H6A1.5 1.5 0 0 0 7.5 14h1a1.5 1.5 0 0 0 1.5-1.5h5.5a.5.5 0 0 0 0-1H10A1.5 1.5 0 0 0 8.5 10V7H14a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2zm.5 3a.5.5 0 1 1 0-1 .5.5 0 0 1 0 1m2 0a.5.5 0 1 1 0-1 .5.5 0 0 1 0 1"/>
                            </svg>
                            <div class="mx-2 text-nowrap">IPs Automatically</div>
                        </div>
                    </div>
                    <div class="col-sm-8">
                        <div class="form-check form-switch">
                            <input class="form-check-input" type="checkbox" role="switch" id="isDhcp" )raw" + isDhcp + R"raw( onchange="ipConfigAvailabilityReDraw()" />
                        </div>
                    </div>
                </div>

                <div class="row align-items-center mb-2" id="ipAddressRow" hidden="true">
                    <div class="col-sm-4">
                        <div class="d-inline-flex align-items-center pt-2 pt-sm-0 ps-sm-2">
                            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-pci-card-network" viewBox="0 0 16 16">
                                <path d="M6.5 9.5v-2h.214a.5.5 0 0 0 .5-.5v-.5h2.572V7a.5.5 0 0 0 .5.5h.214v2z"/>
                                <path d="M0 1.5A.5.5 0 0 1 .5 1h1a.5.5 0 0 1 .5.5V4h13.5a.5.5 0 0 1 .5.5v7a.5.5 0 0 1-.5.5H2v2.5a.5.5 0 0 1-1 0V2H.5a.5.5 0 0 1-.5-.5m6.714 4a.5.5 0 0 0-.5.5v.5H6a.5.5 0 0 0-.5.5v3a.5.5 0 0 0 .5.5h5a.5.5 0 0 0 .5-.5V7a.5.5 0 0 0-.5-.5h-.214V6a.5.5 0 0 0-.5-.5z"/>
                                <path d="M3 12.5h3.5v1a.5.5 0 0 1-.5.5H3.5a.5.5 0 0 1-.5-.5zm8 0H7v1a.5.5 0 0 0 .5.5h3a.5.5 0 0 0 .5-.5z"/>
                            </svg>
                            <div class="mx-2 text-nowrap">IP Address</div>
                        </div>
                    </div>
                    <div class="col-sm-8 d-inline-flex">
                        <input id="ip1" type="number" min="0" max="255" value=")raw" + std::to_string(PersistentSettings::Settings.DeviceAddress & 0xFF) + R"raw(" />.
                        <input id="ip2" type="number" min="0" max="255" value=")raw" + std::to_string((PersistentSettings::Settings.DeviceAddress >> 8) & 0xFF) + R"raw(" />.
                        <input id="ip3" type="number" min="0" max="255" value=")raw" + std::to_string((PersistentSettings::Settings.DeviceAddress >> 16) & 0xFF) + R"raw(" />.
                        <input id="ip4" type="number" min="0" max="255" value=")raw" + std::to_string((PersistentSettings::Settings.DeviceAddress >> 24) & 0xFF) + R"raw(" />
                    </div>
                </div>

                <div class="row align-items-center mb-2" id="ipMaskRow" hidden="true">
                    <div class="col-sm-4">
                        <div class="d-inline-flex align-items-center pt-2 pt-sm-0 ps-sm-2">
                            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-key-fill" viewBox="0 0 16 16">
                                <path d="M3.5 11.5a3.5 3.5 0 1 1 3.163-5H14L15.5 8 14 9.5l-1-1-1 1-1-1-1 1-1-1-1 1H6.663a3.5 3.5 0 0 1-3.163 2M2.5 9a1 1 0 1 0 0-2 1 1 0 0 0 0 2"/>
                            </svg>
                            <div class="mx-2 text-nowrap">IP Mask</div>
                        </div>
                    </div>
                    <div class="col-sm-8 d-inline-flex">
                        <input id="mask1" type="number" min="0" max="255" value=")raw" + std::to_string(PersistentSettings::Settings.NetMask & 0xFF) + R"raw(" />.
                        <input id="mask2" type="number" min="0" max="255" value=")raw" + std::to_string((PersistentSettings::Settings.NetMask >> 8) & 0xFF) + R"raw(" />.
                        <input id="mask3" type="number" min="0" max="255" value=")raw" + std::to_string((PersistentSettings::Settings.NetMask >> 16) & 0xFF) + R"raw(" />.
                        <input id="mask4" type="number" min="0" max="255" value=")raw" + std::to_string((PersistentSettings::Settings.NetMask >> 24) & 0xFF) + R"raw(" />
                    </div>
                </div>

                <div class="row align-items-center mb-2" id="ipGatewayRow" hidden="true">
                    <div class="col-sm-4">
                        <div class="d-inline-flex align-items-center pt-2 pt-sm-0 ps-sm-2">
                            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-router-fill" viewBox="0 0 16 16">
                                <path d="M5.525 3.025a3.5 3.5 0 0 1 4.95 0 .5.5 0 1 0 .707-.707 4.5 4.5 0 0 0-6.364 0 .5.5 0 0 0 .707.707"/>
                                <path d="M6.94 4.44a1.5 1.5 0 0 1 2.12 0 .5.5 0 0 0 .708-.708 2.5 2.5 0 0 0-3.536 0 .5.5 0 0 0 .707.707Z"/>
                                <path d="M2.974 2.342a.5.5 0 1 0-.948.316L3.806 8H1.5A1.5 1.5 0 0 0 0 9.5v2A1.5 1.5 0 0 0 1.5 13H2a.5.5 0 0 0 .5.5h2A.5.5 0 0 0 5 13h6a.5.5 0 0 0 .5.5h2a.5.5 0 0 0 .5-.5h.5a1.5 1.5 0 0 0 1.5-1.5v-2A1.5 1.5 0 0 0 14.5 8h-2.306l1.78-5.342a.5.5 0 1 0-.948-.316L11.14 8H4.86zM2.5 11a.5.5 0 1 1 0-1 .5.5 0 0 1 0 1m4.5-.5a.5.5 0 1 1 1 0 .5.5 0 0 1-1 0m2.5.5a.5.5 0 1 1 0-1 .5.5 0 0 1 0 1m1.5-.5a.5.5 0 1 1 1 0 .5.5 0 0 1-1 0m2 0a.5.5 0 1 1 1 0 .5.5 0 0 1-1 0"/>
                                <path d="M8.5 5.5a.5.5 0 1 1-1 0 .5.5 0 0 1 1 0"/>
                            </svg>
                            <div class="mx-2 text-nowrap">Gateway IP</div>
                        </div>
                    </div>
                    <div class="col-sm-8 d-inline-flex">
                        <input id="gw1" type="number" min="0" max="255" value=")raw" + std::to_string(PersistentSettings::Settings.GatewayAddress & 0xFF) + R"raw(" />.
                        <input id="gw2" type="number" min="0" max="255" value=")raw" + std::to_string((PersistentSettings::Settings.GatewayAddress >> 8) & 0xFF) + R"raw(" />.
                        <input id="gw3" type="number" min="0" max="255" value=")raw" + std::to_string((PersistentSettings::Settings.GatewayAddress >> 16) & 0xFF) + R"raw(" />.
                        <input id="gw4" type="number" min="0" max="255" value=")raw" + std::to_string((PersistentSettings::Settings.GatewayAddress >> 24) & 0xFF) + R"raw(" />
                    </div>
                </div>
                    
            </div>
            <script>
                ipConfigAvailabilityReDraw();
            </script>
            )raw";

            HttpResponse res;
            res.statusCode = 200;
            res.contentType = "text/html";
            res.data = std::make_unique<RamSource>(html);
            return res;
        }

        if (req.uri == "/json" && req.method == "GET") {
            return HttpResponse::fromJsonString(WLEDProvider::HandleGetJson());
        }

        if (req.uri == "/json/state" && req.method == "POST") {
            return WLEDProvider::HandleSetJsonState(req.body);
        }

        if (req.uri == "/presets.json") {
            return HttpResponse::fromJsonString(WLEDProvider::HandleGetPresets());
        }
        
        return {404, std::make_unique<RamSource>("Not Found"), "text/plain"};
    }
}// namespace PicoLightShow